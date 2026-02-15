use std::{fs, io::ErrorKind, mem, ops::Deref, sync::Arc};

use anyhow::Result;
use mlm_db::{
    self, DatabaseExt as _, ErroredTorrentId, Event, EventType, Timestamp, Torrent, TorrentKey, ids,
};
use native_db::Database;
use tracing::{debug, info, instrument, trace, warn};

use crate::config::Config;
use crate::{
    audiobookshelf::Abs,
    linker::rank_torrents,
    logging::{TorrentMetaError, update_errored_torrent, write_event},
    qbittorrent::ensure_category_exists,
};

#[instrument(skip_all)]
pub async fn run_library_cleaner(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    events: &crate::stats::Events,
) -> Result<()> {
    let torrents: Vec<Torrent> = {
        let r = db.r_transaction()?;
        let torrents = r.scan().secondary::<Torrent>(TorrentKey::title_search)?;
        torrents
            .all()?
            .filter_map(|t| t.ok())
            .filter(|t| t.library_path.is_some())
            .collect()
    };
    let mut batch: Vec<Torrent> = vec![];
    for torrent in torrents {
        if let Some(current) = batch.first() {
            if !current.matches(&torrent) {
                process_batch(&config, &db, mem::take(&mut batch), events).await?;
            }
            batch.push(torrent);
        } else {
            batch.push(torrent);
        }
    }
    process_batch(&config, &db, batch, events).await?;

    Ok(())
}

#[instrument(skip_all)]
async fn process_batch(
    config: &Config,
    db: &Database<'_>,
    batch: Vec<Torrent>,
    events: &crate::stats::Events,
) -> Result<()> {
    if batch.len() == 1 {
        return Ok(());
    };
    let mut batch = rank_torrents(config, batch);
    let keep = batch.remove(0);
    for mut remove in batch {
        info!(
            "Replacing library torrent \"{}\" {} with {}",
            remove.meta.title, remove.id, keep.id
        );
        remove.replaced_with = Some((keep.id.clone(), Timestamp::now()));
        let result = clean_torrent(
            config,
            db,
            remove.clone(),
            keep.library_path.is_some() && keep.library_path != remove.library_path,
            events,
        )
        .await
        .map_err(|err| anyhow::Error::new(TorrentMetaError(remove.meta.clone(), err)));
        update_errored_torrent(
            db,
            ErroredTorrentId::Cleaner(remove.id),
            remove.meta.title,
            result,
        )
        .await
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn clean_torrent(
    config: &Config,
    db: &Database<'_>,
    mut remove: Torrent,
    delete_in_abs: bool,
    events: &crate::stats::Events,
) -> Result<()> {
    if remove.id_is_hash {
        for qbit_conf in config.qbittorrent.iter() {
            if let Some(on_cleaned) = &qbit_conf.on_cleaned {
                let qbit: qbit::Api = qbit::Api::new_login_username_password(
                    &qbit_conf.url,
                    &qbit_conf.username,
                    &qbit_conf.password,
                )
                .await?;

                if let Some(category) = &on_cleaned.category {
                    ensure_category_exists(&qbit, &qbit_conf.url, category).await?;
                    qbit.set_category(Some(vec![&remove.id]), category).await?;
                }

                if !on_cleaned.tags.is_empty() {
                    qbit.add_tags(
                        Some(vec![&remove.id]),
                        on_cleaned.tags.iter().map(Deref::deref).collect(),
                    )
                    .await?;
                }
            }
            trace!("qbit updated");
        }
    }

    remove_library_files(config, &remove, delete_in_abs).await?;

    let id = remove.id.clone();
    let mam_id = remove.meta.mam_id();
    let library_path = remove.library_path.take();
    let mut library_files = remove.library_files.clone();
    remove.library_mismatch = None;
    remove.meta.ids.remove(ids::ABS);
    library_files.sort();
    {
        let (_guard, rw) = db.rw_async().await?;
        rw.upsert(remove)?;
        rw.commit()?;
    }

    if let Some(library_path) = library_path {
        write_event(
            db,
            events,
            Event::new(
                Some(id),
                mam_id,
                EventType::Cleaned {
                    library_path,
                    files: library_files,
                },
            ),
        )
        .await;
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn remove_library_files(
    config: &Config,
    remove: &Torrent,
    delete_in_abs: bool,
) -> Result<()> {
    if delete_in_abs
        && let (Some(abs_id), Some(abs_config)) =
            (&remove.meta.ids.get(ids::ABS), &config.audiobookshelf)
    {
        let abs = Abs::new(abs_config)?;
        if let Err(err) = abs.delete_book(abs_id).await {
            warn!("Failed deleting book from abs: {err}");
        }
    }

    if let Some(library_path) = &remove.library_path {
        debug!("Removing library files for torrent {}", remove.id);
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
                fs::remove_dir(library_path.join(sub_dir)).ok();
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
