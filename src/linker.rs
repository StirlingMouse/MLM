use std::{
    fmt::Display,
    fs::{self, File, create_dir_all, hard_link},
    io::{BufWriter, ErrorKind, Write},
    os::unix::fs::MetadataExt as _,
    path::{Component, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Error, Result, bail};
use file_id::get_file_id;
use native_db::Database;
use once_cell::sync::Lazy;
use qbit::{
    models::{TorrentContent, TorrentInfo},
    parameters::TorrentListParams,
};
use regex::Regex;
use serde_json::json;
use time::OffsetDateTime;
use tracing::{Level, debug, error, instrument, span, trace, warn};

use crate::{
    config::{Config, Library, QbitConfig},
    data::{self, ErroredTorrent, ErroredTorrentId, TorrentMeta},
    mam::{MaM, MaMTorrent, clean_value, normalize_title},
    mam_enums::Size,
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
        {
            let r = db.r_transaction()?;
            let torrent: Option<data::Torrent> = r.get().primary(hash.clone())?;
            if torrent.is_some_and(|t| t.library_path.is_some() || t.replaced_with.is_some()) {
                continue;
            }
        }
        if torrent.progress < 1.0 {
            continue;
        }
        let torrent_tags = torrent.tags.split(", ").collect::<Vec<_>>();
        if let Some(ref wanted_tags) = qbit.0.tags {
            let wanted = torrent_tags
                .iter()
                .any(|ttag| wanted_tags.iter().any(|wtag| ttag == wtag));
            if !wanted {
                continue;
            };
        }
        let Some(library) = config
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
                    .any(|tag| torrent_tags.contains(&tag.as_str()))
                {
                    return false;
                }
                if filters.allow_tags.is_empty() {
                    return true;
                }
                filters
                    .allow_tags
                    .iter()
                    .any(|tag| torrent_tags.contains(&tag.as_str()))
            })
        else {
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
        .await;
        if let Err(err) = update_errored_torrent(&db, hash, torrent.name, result) {
            error!("Error writing errored torrent: {err}");
        }
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
    let selected_audio_format = select_format(&config.audio_types, &files);
    let selected_ebook_format = select_format(&config.ebook_types, &files);
    debug!("{selected_audio_format:?} {selected_ebook_format:?}");

    if selected_audio_format.is_none() && selected_ebook_format.is_none() {
        bail!("Could not find any wanted formats in torrent");
    }
    let Some(mam_torrent) = mam.get_torrent_info(hash).await.context("get_mam_info")? else {
        bail!("Could not find torrent on mam");
    };
    let meta = mam_torrent.as_meta()?;

    link_torrent(
        config,
        db,
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
    .map_err(|err| anyhow::Error::new(TorrentMetaError(meta, err)))
}

#[derive(Debug)]
struct TorrentMetaError(TorrentMeta, anyhow::Error);
impl Display for TorrentMetaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.1.fmt(f)
    }
}
impl std::error::Error for TorrentMetaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.1.source()
    }
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
async fn link_torrent(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
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
        debug!("linking: {:?} -> {:?}", download_path, library_path);
        hard_link(&download_path, &library_path).or_else(|err| {
            if err.kind() == ErrorKind::AlreadyExists {
                println!("AlreadyExists: {}", err);
                let download_id = get_file_id(&download_path);
                println!("got 1: {download_id:?}");
                let library_id = get_file_id(&library_path);
                println!("got 2: {library_id:?}");
                if let (Ok(download_id), Ok(library_id)) = (download_id, library_id) {
                println!("got both");
                    if download_id == library_id {
                println!("both match");
                        return Ok(());
                    } else {
                println!("no match");
                        bail!(
                            "File \"{:?}\" already exists, torrent file size: {}, library file size: {}",
                            file_path,
                            fs::metadata(&download_path).map_or("?".to_string(), |s| Size::from_bytes(s.size()).to_string()),
                            fs::metadata(&library_path).map_or("?".to_string(), |s| Size::from_bytes(s.size()).to_string())
                        );
                    }
                }
            }
            Err(err.into())
        })?;
    }

    let file = File::create(dir.join("metadata.json"))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, &metadata)?;
    writer.flush()?;

    {
        let rw = db.rw_transaction()?;
        rw.upsert(data::Torrent {
            hash: hash.to_owned(),
            library_path: Some(dir),
            library_files,
            title_search: normalize_title(&mam_torrent.title),
            meta: meta.clone(),
            created_at: OffsetDateTime::now_utc(),
            replaced_with: None,
            request_matadata_update: false,
        })?;
        rw.commit()?;
    }

    Ok(())
}

fn select_format(wanted_formats: &[String], files: &[TorrentContent]) -> Option<String> {
    wanted_formats
        .iter()
        .map(|ext| {
            if ext.starts_with(".") {
                ext.clone()
            } else {
                format!(".{ext}")
            }
        })
        .find(|ext| files.iter().any(|f| f.name.ends_with(ext)))
}

fn update_errored_torrent(
    db: &Database<'_>,
    hash: &str,
    torrent: String,
    result: Result<(), Error>,
) -> Result<()> {
    let rw = db.rw_transaction()?;
    let id = ErroredTorrentId::Linker(hash.to_owned());
    if let Err(err) = result {
        warn!("Linker error for {torrent}: {err}");
        let (err, meta) = match err.downcast::<TorrentMetaError>() {
            Ok(TorrentMetaError(meta, err)) => (err, Some(meta)),
            Err(err) => (err, None),
        };
        rw.upsert(ErroredTorrent {
            id,
            title: torrent,
            error: format!("{err}"),
            meta,
            created_at: OffsetDateTime::now_utc(),
        })?;
    } else if let Some(error) = rw.get().primary::<ErroredTorrent>(id)? {
        rw.remove(error)?;
    }
    rw.commit()?;
    Ok(())
}
