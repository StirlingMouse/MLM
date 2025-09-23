use std::{fs, io::ErrorKind, mem, ops::Deref, sync::Arc};

use anyhow::Result;
use native_db::Database;
use tracing::{debug, info, instrument, trace};

use crate::{
    config::Config,
    data::{self, ErroredTorrentId, Event, EventType, Timestamp, Torrent},
    linker::file_size,
    logging::{TorrentMetaError, update_errored_torrent, write_event},
    qbittorrent::QbitError,
};

#[instrument(skip_all)]
pub async fn run_library_cleaner(config: Arc<Config>, db: Arc<Database<'_>>) -> Result<()> {
    let torrents: Vec<data::Torrent> = {
        let r = db.r_transaction()?;
        let torrents = r
            .scan()
            .secondary::<data::Torrent>(data::TorrentKey::title_search)?;
        torrents
            .all()?
            .filter_map(|t| t.ok())
            .filter(|t| t.library_path.is_some())
            .collect()
    };
    let mut batch: Vec<data::Torrent> = vec![];
    for torrent in torrents {
        if let Some(current) = batch.first() {
            if current.title_search != torrent.title_search {
                process_batch(&config, &db, mem::take(&mut batch)).await?;
            }
            batch.push(torrent);
        } else {
            batch.push(torrent);
        }
    }
    process_batch(&config, &db, batch).await?;

    Ok(())
}

#[instrument(skip_all)]
async fn process_batch(
    config: &Config,
    db: &Database<'_>,
    batch: Vec<data::Torrent>,
) -> Result<()> {
    if batch.len() == 1 {
        return Ok(());
    };
    let mut batches: Vec<Vec<data::Torrent>> = vec![];

    for torrent in batch {
        if let Some(sub_batch) = batches
            .iter_mut()
            .find(|b| b.iter().any(|t| t.matches(&torrent)))
        {
            sub_batch.push(torrent);
        } else {
            batches.push(vec![torrent]);
        }
    }

    for batch in batches {
        if batch.len() == 1 {
            continue;
        }
        let mut batch = batch
            .into_iter()
            .map(|torrent| {
                let preferred_types = match torrent.meta.main_cat {
                    data::MainCat::Audio => &config.audio_types,
                    data::MainCat::Ebook => &config.ebook_types,
                };
                let preference = preferred_types
                    .iter()
                    .position(|t| torrent.meta.filetypes.contains(t))
                    .unwrap_or(usize::MAX);
                (torrent, preference)
            })
            .collect::<Vec<_>>();
        batch.sort_by_key(|(_, preference)| *preference);
        if batch[0].1 == batch[1].1 {
            trace!(
                "need to compare torrent \"{}\" and \"{}\" by size",
                batch[0].0.meta.title, batch[1].0.meta.title
            );
            let mut new_batch = batch
                .into_iter()
                .map(|(torrent, preference)| {
                    let mut size = 0;
                    if let Some(library_path) = &torrent.library_path {
                        for file in &torrent.library_files {
                            let path = library_path.join(file);
                            size += fs::metadata(path).map_or(0, |s| file_size(&s));
                        }
                    }
                    (torrent, preference, size)
                })
                .collect::<Vec<_>>();
            new_batch.sort_by(|a, b| a.1.cmp(&b.1).then(b.2.cmp(&a.2)));
            trace!("new_batch {:?}", new_batch);
            batch = new_batch
                .into_iter()
                .map(|(torrent, preference, _)| (torrent, preference))
                .collect();
        }
        let (keep, _) = batch.remove(0);
        for (mut remove, _) in batch {
            info!(
                "Replacing library torrent \"{}\" with formats {:?} with {:?}",
                remove.meta.title, remove.meta.filetypes, keep.meta.filetypes
            );
            remove.replaced_with = Some((keep.hash.clone(), Timestamp::now()));
            debug!(
                "keep files: {:?} {:?}",
                keep.library_path, keep.library_files
            );
            debug!("main_cat: {:?}", keep.meta.main_cat);
            debug!("authors: {:?}", keep.meta.authors);
            debug!("narrators: {:?}", keep.meta.narrators);
            debug!(
                "remove files: {:?} {:?}",
                remove.library_path, remove.library_files
            );
            debug!("main_cat: {:?}", remove.meta.main_cat);
            debug!("authors: {:?}", remove.meta.authors);
            debug!("narrators: {:?}", remove.meta.narrators);
            let result = clean_torrent(config, db, remove.clone())
                .await
                .map_err(|err| anyhow::Error::new(TorrentMetaError(remove.meta.clone(), err)));
            update_errored_torrent(
                db,
                ErroredTorrentId::Cleaner(remove.hash),
                remove.meta.title,
                result,
            )
        }
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn clean_torrent(config: &Config, db: &Database<'_>, mut remove: Torrent) -> Result<()> {
    for qbit_conf in config.qbittorrent.iter() {
        if let Some(on_cleaned) = &qbit_conf.on_cleaned {
            let qbit = qbit::Api::login(&qbit_conf.url, &qbit_conf.username, &qbit_conf.password)
                .await
                .map_err(QbitError)?;

            if let Some(category) = &on_cleaned.category {
                qbit.set_category(Some(vec![&remove.hash]), category)
                    .await
                    .map_err(QbitError)?;
            }

            if !on_cleaned.tags.is_empty() {
                qbit.add_tags(
                    Some(vec![&remove.hash]),
                    on_cleaned.tags.iter().map(Deref::deref).collect(),
                )
                .await
                .map_err(QbitError)?;
            }
        }
        trace!("qbit updated");
    }

    remove_library_files(&remove)?;

    let hash = remove.hash.clone();
    let mam_id = remove.meta.mam_id;
    let library_path = remove.library_path.take();
    let mut library_files = remove.library_files.clone();
    remove.library_mismatch = None;
    library_files.sort();
    {
        let rw = db.rw_transaction()?;
        rw.upsert(remove)?;
        rw.commit()?;
    }

    if let Some(library_path) = library_path {
        write_event(
            db,
            Event::new(
                Some(hash),
                Some(mam_id),
                EventType::Cleaned {
                    library_path,
                    files: library_files,
                },
            ),
        );
    }

    Ok(())
}

#[instrument(skip_all)]
pub fn remove_library_files(remove: &Torrent) -> Result<()> {
    if let Some(library_path) = &remove.library_path {
        debug!("Removing library files for torrent {}", remove.meta.mam_id);
        for file in remove.library_files.iter() {
            let path = library_path.join(file);
            fs::remove_file(path).or_else(|err| {
                if err.kind() == ErrorKind::NotFound {
                    trace!("file already missing");
                    Ok(())
                } else {
                    Err(err)
                }
            })?;
            if let Some(sub_dir) = file.parent() {
                fs::remove_dir(sub_dir).ok();
            }
        }
        let mut remove_files = true;
        let mut files_to_remove = vec![];
        if let Ok(files) = fs::read_dir(library_path) {
            for file in files {
                if let Ok(file) = file {
                    if file.file_name() == "cover.jpg" || file.file_name() == "metadata.json" {
                        files_to_remove.push(file);
                    } else {
                        remove_files = false;
                    }
                } else {
                    remove_files = false;
                }
            }
            if remove_files {
                for file in files_to_remove {
                    fs::remove_file(file.path()).ok();
                }
            }
        }
        fs::remove_dir(library_path).ok();
        trace!("files removed");
    }
    Ok(())
}
