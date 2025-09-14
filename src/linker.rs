#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt as _;
#[cfg(target_family = "windows")]
use std::os::windows::fs::MetadataExt as _;
use std::{
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
    models::{TorrentContent, TorrentInfo},
    parameters::TorrentListParams,
};
use regex::Regex;
use serde_json::json;
use tracing::{Level, debug, instrument, span, trace};

use crate::{
    cleaner::remove_library_files,
    config::{Config, Library, LibraryLinkMethod, QbitConfig},
    data::{
        ErroredTorrentId, Event, EventType, LibraryMismatch, Size, Timestamp, Torrent, TorrentMeta,
    },
    logging::{TorrentMetaError, update_errored_torrent, write_event},
    mam::{MaM, MaMTorrent, clean_value, normalize_title},
    qbittorrent::QbitError,
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
        .torrents(TorrentListParams::default())
        .await
        .map_err(QbitError)
        .context("qbit main data")?;

    for torrent in torrents {
        let Some(hash) = &torrent.hash else {
            continue;
        };
        if torrent.progress < 1.0 {
            continue;
        }
        {
            let r = db.r_transaction()?;
            let t: Option<Torrent> = r.get().primary(hash.clone())?;
            if let Some(mut t) = t {
                if let Some(library_path) = &t.library_path {
                    let Some(library) = find_library(&config, &torrent) else {
                        if t.library_mismatch != Some(LibraryMismatch::NoLibrary) {
                            println!("no library: {library_path:?}",);
                            t.library_mismatch = Some(LibraryMismatch::NoLibrary);
                            let rw = db.rw_transaction()?;
                            rw.upsert(t)?;
                            rw.commit()?;
                        }
                        continue;
                    };
                    if !library_path.starts_with(library.library_dir()) {
                        let wanted = Some(LibraryMismatch::NewPath(library.library_dir().clone()));
                        if t.library_mismatch != wanted {
                            println!(
                                "path differs: {library_path:?} != {:?}",
                                library.library_dir()
                            );
                            t.library_mismatch = wanted;
                            let rw = db.rw_transaction()?;
                            rw.upsert(t)?;
                            rw.commit()?;
                        }
                    }
                    continue;
                }
                if t.replaced_with.is_some() {
                    continue;
                }
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
            hash,
            &torrent,
            library,
        )
        .await
        .context("match_torrent");
        update_errored_torrent(
            &db,
            ErroredTorrentId::Linker(hash.clone()),
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
    torrent: &TorrentInfo,
    library: &Library,
) -> Result<()> {
    let files = qbit.1.files(hash, None).await.map_err(QbitError)?;
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
        &db,
        hash,
        torrent,
        files,
        selected_audio_format,
        selected_ebook_format,
        library,
        mam_torrent,
        &meta,
    )
    .await
    .context("link_torrent")
    .map_err(|err| anyhow::Error::new(TorrentMetaError(meta, err)))
}

#[instrument(skip_all)]
pub async fn refresh_metadata(
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

    let metadata = create_metadata(&mam_torrent, &meta);
    if let (Some(libary_path), serde_json::Value::Object(new)) = (&torrent.library_path, metadata) {
        let metadata_path = libary_path.join("metadata.json");
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
    }

    torrent.meta = meta.clone();
    {
        let rw = db.rw_transaction()?;
        rw.upsert(torrent.clone())?;
        rw.commit()?;
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
        let qbit = match qbit::Api::login(&qbit_conf.url, &qbit_conf.username, &qbit_conf.password)
            .await
            .map_err(QbitError)
        {
            Ok(qbit) => qbit,
            Err(err) => {
                error!("Error logging in to qbit {}: {err}", qbit_conf.url);
                continue;
            }
        };
        let mut torrents = match qbit
            .torrents(TorrentListParams {
                hashes: Some(vec![hash.clone()]),
                ..TorrentListParams::default()
            })
            .await
            .map_err(QbitError)
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
        torrent.replace((qbit, t));
        break;
    }
    let Some((qbit, qbit_torrent)) = torrent else {
        bail!("Could not find torrent in qbit");
    };
    let Some(library) = find_library(config, &qbit_torrent) else {
        bail!("Could not find matching library for torrent");
    };
    let files = qbit.files(&hash, None).await.map_err(QbitError)?;
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
    let (torrent, mam_torrent) = refresh_metadata(db, mam, hash.clone()).await?;
    remove_library_files(&torrent)?;
    link_torrent(
        config,
        db,
        &hash,
        &qbit_torrent,
        files,
        selected_audio_format,
        selected_ebook_format,
        library,
        mam_torrent,
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
    db: &Database<'_>,
    hash: &str,
    torrent: &TorrentInfo,
    files: Vec<TorrentContent>,
    selected_audio_format: Option<String>,
    selected_ebook_format: Option<String>,
    library: &Library,
    mam_torrent: MaMTorrent,
    meta: &TorrentMeta,
) -> Result<()> {
    let Some((_, author)) = mam_torrent.author_info.first_key_value() else {
        bail!("Torrent has no author");
    };
    let author = clean_value(author)?;
    let series = mam_torrent.series_info.first_key_value();

    let mut dir = match series {
        Some((_, (series_name, series_num))) => {
            let series_name = clean_value(series_name)?;
            PathBuf::from(author)
                .join(&series_name)
                .join(if series_num.is_empty() {
                    mam_torrent.title.clone()
                } else {
                    format!("{series_name} #{series_num} - {}", mam_torrent.title)
                })
        }
        None => PathBuf::from(author).join(&mam_torrent.title),
    };
    if let Some(narrator) = &meta.narrators.first() {
        let mut force_narrator = false;
        if config.exclude_narrator_in_library_dir && library.library_dir().join(&dir).exists() {
            force_narrator = true;
        }
        if !config.exclude_narrator_in_library_dir || force_narrator {
            dir.set_file_name(format!(
                "{} {{{}}}",
                dir.file_name().unwrap().to_string_lossy(),
                narrator
            ));
        }
    }
    let dir = library.library_dir().join(dir);
    trace!("out_dir: {:?}", dir);

    let metadata = create_metadata(&mam_torrent, meta);
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
        let download_path = PathBuf::from(&torrent.save_path).join(&file.name);
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
            library_path: Some(dir.clone()),
            library_files,
            selected_audio_format,
            selected_ebook_format,
            title_search: normalize_title(&mam_torrent.title),
            meta: meta.clone(),
            created_at: Timestamp::now(),
            replaced_with: None,
            request_matadata_update: false,
            library_mismatch: None,
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

fn find_library<'a>(config: &'a Config, torrent: &TorrentInfo) -> Option<&'a Library> {
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

fn create_metadata(mam_torrent: &MaMTorrent, meta: &TorrentMeta) -> serde_json::Value {
    let mut titles = mam_torrent.title.splitn(2, ":");
    let title = titles.next().unwrap();
    let subtitle = titles.next().map(|t| t.trim());
    let isbn_raw: &str = mam_torrent.isbn.as_deref().unwrap_or("");
    let isbn = if isbn_raw.is_empty() || isbn_raw.starts_with("ASIN:") {
        None
    } else {
        Some(isbn_raw)
    };
    let asin = isbn_raw.strip_prefix("ASIN:");

    let metadata = json!({
        "authors": &meta.authors,
        "narrators": &meta.narrators,
        "series": &meta.series.iter().map(|(series_name, series_num)| {
            if series_num.is_empty() { series_name.clone() } else { format!("{series_name} #{series_num}") }
        }).collect::<Vec<_>>(),
        "title": title,
        "subtitle": subtitle,
        "description": mam_torrent.description,
        "isbn": isbn,
        "asin": asin,
    });

    metadata
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
