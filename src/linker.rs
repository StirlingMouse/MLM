#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt as _;
#[cfg(target_family = "windows")]
use std::os::windows::fs::MetadataExt as _;
use std::{
    collections::BTreeMap,
    fs::{self, File, Metadata, create_dir_all},
    io::{BufWriter, ErrorKind, Write},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use file_id::get_file_id;
use log::error;
use native_db::Database;
use once_cell::sync::Lazy;
use qbit::{
    models::{Torrent as QbitTorrent, TorrentContent},
    parameters::TorrentListParams,
};
use regex::Regex;
use tracing::{Level, debug, instrument, span, trace, warn};

use crate::{
    audiobookshelf::{self as abs, Abs},
    autograbber::update_torrent_meta,
    cleaner::remove_library_files,
    config::{Config, Library, LibraryLinkMethod, QbitConfig},
    data::{
        ClientStatus, ErroredTorrentId, Event, EventType, LibraryMismatch, Size, Timestamp,
        Torrent, TorrentMeta,
    },
    logging::{TorrentMetaError, update_errored_torrent, write_event},
    mam::{MaM, MaMTorrent, normalize_title},
};

pub static DISK_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:CD|Disc|Disk)\s*(\d+)").unwrap());

#[instrument(skip_all)]
pub async fn link_torrents_to_library(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    qbit: (&QbitConfig, &qbit::Api),
    mam: Arc<MaM<'_>>,
) -> Result<()> {
    let torrents = qbit
        .1
        .torrents(Some(TorrentListParams::default()))
        .await
        .context("qbit main data")?;

    for torrent in torrents {
        if torrent.progress < 1.0 {
            continue;
        }
        let r = db.r_transaction()?;
        let mut existing_torrent: Option<Torrent> = r.get().primary(torrent.hash.clone())?;
        if let Some(t) = &mut existing_torrent {
            if t.client_status.is_none() {
                let trackers = qbit.1.trackers(&torrent.hash).await?;
                if let Some(mam_tracker) = trackers.last() {
                    if mam_tracker.msg == "torrent not registered with this tracker" {
                        let rw = db.rw_transaction()?;
                        t.client_status = Some(ClientStatus::RemovedFromMam);
                        rw.upsert(t.clone())?;
                        rw.commit()?;
                        write_event(
                            &db,
                            Event::new(
                                Some(torrent.hash.clone()),
                                Some(t.mam_id),
                                EventType::RemovedFromMam,
                            ),
                        );
                    }
                }
            }
            if let Some(library_path) = &t.library_path {
                let Some(library) = find_library(&config, &torrent) else {
                    if t.library_mismatch != Some(LibraryMismatch::NoLibrary) {
                        debug!("no library: {library_path:?}",);
                        t.library_mismatch = Some(LibraryMismatch::NoLibrary);
                        let rw = db.rw_transaction()?;
                        rw.upsert(t.clone())?;
                        rw.commit()?;
                    }
                    continue;
                };
                if !library_path.starts_with(library.library_dir()) {
                    let wanted = Some(LibraryMismatch::NewLibraryDir(
                        library.library_dir().clone(),
                    ));
                    if t.library_mismatch != wanted {
                        debug!(
                            "library differs: {library_path:?} != {:?}",
                            library.library_dir()
                        );
                        t.library_mismatch = wanted;
                        let rw = db.rw_transaction()?;
                        rw.upsert(t.clone())?;
                        rw.commit()?;
                    }
                } else {
                    let dir = library_dir(config.exclude_narrator_in_library_dir, library, &t.meta);
                    let mut is_wrong = Some(library_path) != dir.as_ref();
                    let wanted = match dir {
                        Some(dir) => Some(LibraryMismatch::NewPath(dir)),
                        None => Some(LibraryMismatch::NoLibrary),
                    };

                    if t.library_mismatch != wanted {
                        if is_wrong {
                            // Try another attempt at matching with exclude_narrator flipped
                            let dir_2 = library_dir(
                                !config.exclude_narrator_in_library_dir,
                                library,
                                &t.meta,
                            );
                            if Some(library_path) == dir_2.as_ref() {
                                is_wrong = false
                            }
                        }
                        if is_wrong {
                            debug!("path differs: {library_path:?} != {:?}", wanted);
                            t.library_mismatch = wanted;
                            let rw = db.rw_transaction()?;
                            rw.upsert(t.clone())?;
                            rw.commit()?;
                        } else if t.library_mismatch.is_some() {
                            t.library_mismatch = None;
                            let rw = db.rw_transaction()?;
                            rw.upsert(t.clone())?;
                            rw.commit()?;
                        }
                    }
                }
                continue;
            }
            if t.replaced_with.is_some() {
                continue;
            }
        }
        let Some(library) = find_library(&config, &torrent) else {
            trace!(
                "Could not find matching library for torrent \"{}\", save_path {}",
                torrent.name, torrent.save_path
            );
            continue;
        };

        let result = match_torrent(
            config.clone(),
            db.clone(),
            qbit,
            mam.clone(),
            &torrent.hash,
            &torrent,
            library,
            existing_torrent,
        )
        .await
        .context("match_torrent");
        update_errored_torrent(
            &db,
            ErroredTorrentId::Linker(torrent.hash.clone()),
            torrent.name,
            result,
        )
    }

    Ok(())
}

#[instrument(skip_all)]
async fn match_torrent(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    qbit: (&QbitConfig, &qbit::Api),
    mam: Arc<MaM<'_>>,
    hash: &str,
    torrent: &QbitTorrent,
    library: &Library,
    existing_torrent: Option<Torrent>,
) -> Result<()> {
    let files = qbit.1.files(hash, None).await?;
    let selected_audio_format = select_format(
        &library.tag_filters().audio_types,
        &config.audio_types,
        &files,
    );
    let selected_ebook_format = select_format(
        &library.tag_filters().ebook_types,
        &config.ebook_types,
        &files,
    );

    if selected_audio_format.is_none() && selected_ebook_format.is_none() {
        bail!("Could not find any wanted formats in torrent");
    }
    let Some(mam_torrent) = mam.get_torrent_info(hash).await.context("get_mam_info")? else {
        bail!("Could not find torrent on mam");
    };
    let meta = mam_torrent.as_meta().context("as_meta")?;

    link_torrent(
        &config,
        qbit.0,
        &db,
        hash,
        torrent,
        files,
        selected_audio_format,
        selected_ebook_format,
        library,
        mam_torrent,
        existing_torrent.as_ref(),
        &meta,
    )
    .await
    .context("link_torrent")
    .map_err(|err| anyhow::Error::new(TorrentMetaError(meta, err)))
}

#[instrument(skip_all)]
pub async fn refresh_metadata(
    config: &Config,
    db: &Database<'_>,
    mam: &MaM<'_>,
    hash: String,
) -> Result<(Torrent, MaMTorrent)> {
    let Some(mut torrent): Option<Torrent> = db.r_transaction()?.get().primary(hash)? else {
        bail!("Could not find torrent hash");
    };
    debug!("refreshing metadata for torrent {}", torrent.meta.mam_id);
    let Some(mam_torrent) = mam
        .get_torrent_info(&torrent.hash)
        .await
        .context("get_mam_info")?
    else {
        bail!("Could not find torrent \"{}\" on mam", torrent.meta.title);
    };
    let meta = mam_torrent.as_meta().context("as_meta")?;

    let metadata = abs::create_metadata(&mam_torrent, &meta);
    if let (Some(library_path), serde_json::Value::Object(new)) = (&torrent.library_path, metadata)
    {
        let metadata_path = library_path.join("metadata.json");
        if metadata_path.exists() {
            let existing = fs::read_to_string(&metadata_path)?;
            let mut existing: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&existing)?;
            for (key, value) in new {
                existing.insert(key, value);
            }
            let file = File::create(&metadata_path)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer(&mut writer, &serde_json::Value::Object(existing))?;
            writer.flush()?;
            debug!("updated ABS metadata file {}", torrent.meta.mam_id);
        }
        if let (Some(abs_id), Some(abs_config)) = (&torrent.abs_id, &config.audiobookshelf) {
            let abs = Abs::new(abs_config)?;
            match abs.update_book(abs_id, &mam_torrent, &meta).await {
                Ok(_) => debug!("updated ABS via API {}", torrent.meta.mam_id),
                Err(err) => warn!("Failed updating book {} in abs: {err}", torrent.meta.mam_id),
            }
        }
    }

    if torrent.meta != meta {
        update_torrent_meta(db, db.rw_transaction()?, torrent.clone(), meta.clone())?;
        torrent.meta = meta;
    }
    Ok((torrent, mam_torrent))
}

#[instrument(skip_all)]
pub async fn refresh_metadata_relink(
    config: &Config,
    db: &Database<'_>,
    mam: &MaM<'_>,
    hash: String,
) -> Result<()> {
    let mut torrent = None;
    for qbit_conf in &config.qbittorrent {
        let qbit = match qbit::Api::new_login_username_password(
            &qbit_conf.url,
            &qbit_conf.username,
            &qbit_conf.password,
        )
        .await
        {
            Ok(qbit) => qbit,
            Err(err) => {
                error!("Error logging in to qbit {}: {err}", qbit_conf.url);
                continue;
            }
        };
        let mut torrents = match qbit
            .torrents(Some(TorrentListParams {
                hashes: Some(vec![hash.clone()]),
                ..TorrentListParams::default()
            }))
            .await
        {
            Ok(torrents) => torrents,
            Err(err) => {
                error!("Error getting torrents from qbit {}: {err}", qbit_conf.url);
                continue;
            }
        };
        let Some(t) = torrents.pop() else {
            continue;
        };
        torrent.replace((qbit_conf, qbit, t));
        break;
    }
    let Some((qbit_conf, qbit, qbit_torrent)) = torrent else {
        bail!("Could not find torrent in qbit");
    };
    let Some(library) = find_library(config, &qbit_torrent) else {
        bail!("Could not find matching library for torrent");
    };
    let files = qbit.files(&hash, None).await?;
    let selected_audio_format = select_format(
        &library.tag_filters().audio_types,
        &config.audio_types,
        &files,
    );
    let selected_ebook_format = select_format(
        &library.tag_filters().ebook_types,
        &config.ebook_types,
        &files,
    );

    if selected_audio_format.is_none() && selected_ebook_format.is_none() {
        bail!("Could not find any wanted formats in torrent");
    }
    let (torrent, mam_torrent) = refresh_metadata(config, db, mam, hash.clone()).await?;
    let library_path_changed = torrent.library_path
        != library_dir(
            config.exclude_narrator_in_library_dir,
            library,
            &torrent.meta,
        );
    remove_library_files(config, &torrent, library_path_changed).await?;
    link_torrent(
        config,
        qbit_conf,
        db,
        &hash,
        &qbit_torrent,
        files,
        selected_audio_format,
        selected_ebook_format,
        library,
        mam_torrent,
        Some(&torrent),
        &torrent.meta,
    )
    .await
    .context("link_torrent")
    .map_err(|err| anyhow::Error::new(TorrentMetaError(torrent.meta, err)))
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
async fn link_torrent(
    config: &Config,
    qbit_config: &QbitConfig,
    db: &Database<'_>,
    hash: &str,
    torrent: &QbitTorrent,
    files: Vec<TorrentContent>,
    selected_audio_format: Option<String>,
    selected_ebook_format: Option<String>,
    library: &Library,
    mam_torrent: MaMTorrent,
    existing_torrent: Option<&Torrent>,
    meta: &TorrentMeta,
) -> Result<()> {
    let Some(mut dir) = library_dir(config.exclude_narrator_in_library_dir, library, meta) else {
        bail!("Torrent has no author");
    };
    if config.exclude_narrator_in_library_dir && !meta.narrators.is_empty() && dir.exists() {
        dir = library_dir(false, library, meta).unwrap();
    }

    let metadata = abs::create_metadata(&mam_torrent, meta);
    create_dir_all(&dir)?;

    let mut library_files = vec![];
    for file in files {
        let span = span!(Level::TRACE, "file: {:?}", file.name);
        let _s = span.enter();
        if !(selected_audio_format
            .as_ref()
            .is_some_and(|ext| file.name.ends_with(ext))
            || selected_ebook_format
                .as_ref()
                .is_some_and(|ext| file.name.ends_with(ext)))
        {
            debug!("Skiping \"{}\"", file.name);
            continue;
        }
        let torrent_path = PathBuf::from(&file.name);
        let mut path_components = torrent_path.components();
        let file_name = path_components.next_back().unwrap();
        let dir_name = path_components.next_back().and_then(|dir_name| {
            if let Component::Normal(dir_name) = dir_name {
                let dir_name = dir_name.to_string_lossy().to_string();
                if let Some(disc) = DISK_PATTERN.captures(&dir_name).and_then(|c| c.get(1)) {
                    return Some(format!("Disc {}", disc.as_str()));
                }
            }
            None
        });
        let file_path = if let Some(dir_name) = dir_name {
            let sub_dir = PathBuf::from(dir_name);
            create_dir_all(dir.join(&sub_dir))?;
            sub_dir.join(file_name)
        } else {
            PathBuf::from(&file_name)
        };
        let library_path = dir.join(&file_path);
        library_files.push(file_path.clone());
        let download_path =
            map_path(&qbit_config.path_mapping, &torrent.save_path).join(&file.name);
        match library.method() {
            LibraryLinkMethod::Hardlink => hard_link(&download_path, &library_path, &file_path)?,
            LibraryLinkMethod::HardlinkOrCopy => {
                hard_link(&download_path, &library_path, &file_path)
                    .or_else(|_| copy(&download_path, &library_path))?
            }
            LibraryLinkMethod::Copy => copy(&download_path, &library_path)?,
            LibraryLinkMethod::HardlinkOrSymlink => {
                hard_link(&download_path, &library_path, &file_path)
                    .or_else(|_| symlink(&download_path, &library_path))?
            }
            LibraryLinkMethod::Symlink => symlink(&download_path, &library_path)?,
        };
    }
    library_files.sort();

    let file = File::create(dir.join("metadata.json"))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, &metadata)?;
    writer.flush()?;

    {
        let rw = db.rw_transaction()?;
        rw.upsert(Torrent {
            hash: hash.to_owned(),
            mam_id: meta.mam_id,
            abs_id: existing_torrent.and_then(|t| t.abs_id.clone()),
            library_path: Some(dir.clone()),
            library_files,
            selected_audio_format,
            selected_ebook_format,
            title_search: normalize_title(&mam_torrent.title),
            meta: meta.clone(),
            created_at: existing_torrent
                .map(|t| t.created_at)
                .unwrap_or_else(Timestamp::now),
            replaced_with: existing_torrent.and_then(|t| t.replaced_with.clone()),
            request_matadata_update: false,
            library_mismatch: None,
            client_status: existing_torrent.and_then(|t| t.client_status.clone()),
        })?;
        rw.commit()?;
    }

    write_event(
        db,
        Event::new(
            Some(hash.to_owned()),
            Some(meta.mam_id),
            EventType::Linked { library_path: dir },
        ),
    );

    Ok(())
}

fn map_path(path_mapping: &BTreeMap<PathBuf, PathBuf>, save_path: &str) -> PathBuf {
    let mut path = PathBuf::from(save_path);
    for (from, to) in path_mapping.iter().rev() {
        if path.starts_with(from) {
            let mut components = path.components();
            for _ in from {
                components.next();
            }
            path = to.join(components.as_path());
            break;
        }
    }
    path
}

pub fn find_library<'a>(config: &'a Config, torrent: &QbitTorrent) -> Option<&'a Library> {
    config
        .libraries
        .iter()
        .filter(|l| match l {
            Library::ByDir(l) => PathBuf::from(&torrent.save_path).starts_with(&l.download_dir),
            Library::ByCategory(l) => torrent.category == l.category,
        })
        .find(|l| {
            let filters = l.tag_filters();
            if filters
                .deny_tags
                .iter()
                .any(|tag| torrent.tags.split(", ").any(|t| t == tag.as_str()))
            {
                return false;
            }
            if filters.allow_tags.is_empty() {
                return true;
            }
            filters
                .allow_tags
                .iter()
                .any(|tag| torrent.tags.split(", ").any(|t| t == tag.as_str()))
        })
}

pub fn library_dir(
    exclude_narrator_in_library_dir: bool,
    library: &Library,
    meta: &TorrentMeta,
) -> Option<PathBuf> {
    let author = meta.authors.first()?;
    let mut dir = match meta
        .series
        .iter()
        .find(|s| !s.entries.0.is_empty())
        .or(meta.series.first())
    {
        Some(series) => {
            PathBuf::from(author)
                .join(&series.name)
                .join(if series.entries.0.is_empty() {
                    meta.title.clone()
                } else {
                    format!("{} #{} - {}", series.name, series.entries, meta.title)
                })
        }
        None => PathBuf::from(author).join(&meta.title),
    };
    if let Some(narrator) = meta.narrators.first() {
        if !exclude_narrator_in_library_dir {
            dir.set_file_name(format!(
                "{} {{{}}}",
                dir.file_name().unwrap().to_string_lossy(),
                narrator
            ));
        }
    }
    let dir = library.library_dir().join(dir);
    Some(dir)
}

fn select_format(
    overridden_wanted_formats: &Option<Vec<String>>,
    wanted_formats: &[String],
    files: &[TorrentContent],
) -> Option<String> {
    overridden_wanted_formats
        .as_deref()
        .unwrap_or(wanted_formats)
        .iter()
        .map(|ext| {
            let ext = ext.to_lowercase();
            if ext.starts_with(".") {
                ext.clone()
            } else {
                format!(".{ext}")
            }
        })
        .find(|ext| files.iter().any(|f| f.name.to_lowercase().ends_with(ext)))
}

#[instrument(skip_all)]
fn hard_link(download_path: &Path, library_path: &Path, file_path: &Path) -> Result<()> {
    debug!("linking: {:?} -> {:?}", download_path, library_path);
    fs::hard_link(download_path, library_path).or_else(|err| {
            if err.kind() == ErrorKind::AlreadyExists {
                trace!("AlreadyExists: {}", err);
                let download_id = get_file_id(download_path);
                trace!("got 1: {download_id:?}");
                let library_id = get_file_id(library_path);
                trace!("got 2: {library_id:?}");
                if let (Ok(download_id), Ok(library_id)) = (download_id, library_id) {
                    trace!("got both");
                    if download_id == library_id {
                        trace!("both match");
                        return Ok(());
                    } else {
                        trace!("no match");
                        bail!(
                            "File \"{:?}\" already exists, torrent file size: {}, library file size: {}",
                            file_path,
                            fs::metadata(download_path).map_or("?".to_string(), |s| Size::from_bytes(file_size(&s)).to_string()),
                            fs::metadata(library_path).map_or("?".to_string(), |s| Size::from_bytes(file_size(&s)).to_string())
                        );
                    }
                }
            }
            Err(err.into())
        })?;
    Ok(())
}

#[instrument(skip_all)]
fn copy(download_path: &Path, library_path: &Path) -> Result<()> {
    debug!("copying: {:?} -> {:?}", download_path, library_path);
    fs::copy(download_path, library_path)?;
    Ok(())
}

#[instrument(skip_all)]
fn symlink(download_path: &Path, library_path: &Path) -> Result<()> {
    debug!("symlinking: {:?} -> {:?}", download_path, library_path);
    #[cfg(target_family = "unix")]
    std::os::unix::fs::symlink(download_path, library_path)?;
    #[cfg(target_family = "windows")]
    bail!("symlink is not supported on Windows");
    #[allow(unreachable_code)]
    Ok(())
}

pub fn file_size(m: &Metadata) -> u64 {
    #[cfg(target_family = "unix")]
    return m.size();
    #[cfg(target_family = "windows")]
    return m.file_size();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_path() {
        let mut mappings = BTreeMap::new();
        mappings.insert(PathBuf::from("/downloads"), PathBuf::from("/books"));
        mappings.insert(
            PathBuf::from("/downloads/audiobooks"),
            PathBuf::from("/audiobooks"),
        );
        mappings.insert(PathBuf::from("/audiobooks"), PathBuf::from("/audiobooks"));

        assert_eq!(
            map_path(&mappings, "/downloads/torrent"),
            PathBuf::from("/books/torrent")
        );
        assert_eq!(
            map_path(&mappings, "/downloads/audiobooks/torrent"),
            PathBuf::from("/audiobooks/torrent")
        );
        assert_eq!(
            map_path(&mappings, "/downloads/audiobooks/torrent/deep"),
            PathBuf::from("/audiobooks/torrent/deep")
        );
        assert_eq!(
            map_path(&mappings, "/audiobooks/torrent"),
            PathBuf::from("/audiobooks/torrent")
        );
        assert_eq!(
            map_path(&mappings, "/ebooks/torrent"),
            PathBuf::from("/ebooks/torrent")
        );
    }
}
