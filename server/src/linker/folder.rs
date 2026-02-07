use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write as _},
    path::PathBuf,
    str::FromStr as _,
    sync::Arc,
};

use anyhow::{Result, bail};
use mlm_db::{
    DatabaseExt as _, ErroredTorrentId, Event, EventType, FlagBits, Flags, Language, MediaType,
    MetadataSource, Series, SeriesEntries, SeriesEntry, Size, Timestamp, Torrent, TorrentMeta, ids,
};
use mlm_mam::meta::clean_meta;
use mlm_parse::{normalize_title, parse_series_from_title};
use native_db::Database;
use serde_derive::{Deserialize, Serialize};
use time::UtcDateTime;
use tokio::fs::{DirEntry, create_dir_all, read_dir, read_to_string};
use tracing::{Level, instrument, span, trace, warn};

use crate::audiobookshelf as abs;
use crate::config::{Config, Library, LibraryLinkMethod};
use crate::linker::{
    copy, file_size, find_matches, hard_link, library_dir, rank_torrents, select_format, symlink,
};
use crate::logging::{update_errored_torrent, write_event};

#[instrument(skip_all)]
pub async fn link_folders_to_library(config: Arc<Config>, db: Arc<Database<'_>>) -> Result<()> {
    for library in &config.libraries {
        if let Library::ByRipDir(l) = library {
            let mut entries = read_dir(&l.rip_dir).await?;
            while let Some(folder) = entries.next_entry().await? {
                link_folder(&config, library, &db, folder).await?;
            }
        }
    }

    Ok(())
}

async fn link_folder(
    config: &Config,
    library: &Library,
    db: &Database<'_>,
    folder: DirEntry,
) -> Result<()> {
    let span = span!(
        Level::TRACE,
        "link_folder",
        folder = folder.path().to_string_lossy().to_string(),
    );
    let _s = span.enter();
    let mut entries = read_dir(&folder.path()).await?;

    let mut audio_files = vec![];
    let mut ebook_files = vec![];
    let mut metadata_file = None;
    // let mut cover_file = None;
    while let Some(entry) = entries.next_entry().await? {
        match entry.path().extension() {
            Some(ext) if ext == "json" => metadata_file = Some(entry),
            // Some(ext) if ext == "jpg" || ext == "png" => cover_file = Some(entry),
            Some(ext)
                if config
                    .audio_types
                    .iter()
                    .any(|e| e == &ext.to_string_lossy()) =>
            {
                audio_files.push(entry)
            }
            Some(ext)
                if config
                    .ebook_types
                    .iter()
                    .any(|e| e == &ext.to_string_lossy()) =>
            {
                ebook_files.push(entry)
            }
            _ => {}
        }
    }

    let Some(metadata_file) = metadata_file else {
        warn!("Missing metadata file");
        return Ok(());
    };

    let json = read_to_string(metadata_file.path()).await?;
    if let Ok(libation_meta) = serde_json::from_str::<Libation>(&json) {
        trace!("Linking libation folder");
        let asin = libation_meta.asin.clone();
        let title = libation_meta.title.clone();
        let result =
            link_libation_folder(config, library, db, libation_meta, audio_files, ebook_files)
                .await;
        update_errored_torrent(db, ErroredTorrentId::Linker(asin), title, result).await;
    }

    Ok(())
}

async fn link_libation_folder(
    config: &Config,
    library: &Library,
    db: &Database<'_>,
    libation_meta: Libation,
    audio_files: Vec<DirEntry>,
    ebook_files: Vec<DirEntry>,
) -> Result<()> {
    let r = db.r_transaction()?;
    let existing_torrent: Option<Torrent> = r.get().primary(libation_meta.asin.clone())?;
    if existing_torrent.is_some() {
        return Ok(());
    }

    let title = if libation_meta.subtitle.is_empty() {
        libation_meta.title
    } else {
        format!("{}: {}", libation_meta.title, libation_meta.subtitle)
    };

    let mut series = libation_meta
        .series
        .into_iter()
        .filter_map(|s| Series::try_from((s.title, s.sequence)).ok())
        .collect::<Vec<_>>();
    if series.is_empty()
        && let Some((name, num)) = parse_series_from_title(&title)
    {
        series.push(Series {
            name: name.to_string(),
            entries: SeriesEntries::new(num.into_iter().map(SeriesEntry::Num).collect()),
        });
    }

    let mut ids = BTreeMap::new();
    ids.insert(ids::ASIN.to_string(), libation_meta.asin.clone());
    let mut flags = Flags::default();
    if libation_meta.format_type.starts_with("abridged") {
        flags.abridged = Some(true);
    }
    let mut size = 0;
    for file in &audio_files {
        size += file_size(&file.metadata().await?);
    }
    for file in &ebook_files {
        size += file_size(&file.metadata().await?);
    }

    let meta = TorrentMeta {
        ids,
        vip_status: None,
        cat: None,
        media_type: MediaType::Audiobook,
        main_cat: None,
        categories: vec![],
        tags: vec![],
        language: Language::from_str(&libation_meta.language).ok(),
        flags: Some(FlagBits::new(flags.as_bitfield())),
        filetypes: vec!["m4b".to_string()],
        num_files: audio_files.len() as u64,
        size: Size::from_bytes(size),
        title,
        edition: None,
        description: libation_meta.publisher_summary,
        authors: libation_meta.authors.into_iter().map(|a| a.name).collect(),
        narrators: libation_meta
            .narrators
            .into_iter()
            .map(|a| a.name)
            .collect(),
        series,
        source: MetadataSource::File,
        uploaded_at: Timestamp::from(UtcDateTime::UNIX_EPOCH),
    };
    let meta = clean_meta(meta, "")?;

    let mut torrent = Torrent {
        id: libation_meta.asin.clone(),
        id_is_hash: false,
        mam_id: meta.mam_id(),
        library_path: None,
        library_files: vec![],
        linker: library.options().name.clone(),
        category: None,
        selected_audio_format: None,
        selected_ebook_format: None,
        title_search: normalize_title(&meta.title),
        meta: meta.clone(),
        created_at: Timestamp::now(),
        replaced_with: None,
        library_mismatch: None,
        client_status: None,
    };

    let matches = find_matches(db, &torrent)?;
    if !matches.is_empty() {
        let mut batch = matches;
        batch.push(torrent.clone());
        let ranked = rank_torrents(config, batch);
        if ranked[0].id != torrent.id {
            trace!("Skipping folder as it is a duplicate of a better torrent already in library");
            return Ok(());
        }
    }

    if let Some(filter) = library.edition_filter()
        && !filter.matches_meta(&meta).is_ok_and(|matches| matches)
    {
        trace!("Skipping folder due to edition filter");
        return Ok(());
    }

    let mut library_files = vec![];
    let selected_audio_format = select_format(
        &library.options().audio_types,
        &config.audio_types,
        &audio_files,
    );
    let selected_ebook_format = select_format(
        &library.options().ebook_types,
        &config.ebook_types,
        &ebook_files,
    );

    let library_path = if library.options().method != LibraryLinkMethod::NoLink {
        let Some(mut dir) = library_dir(config.exclude_narrator_in_library_dir, library, &meta)
        else {
            bail!("Torrent has no author");
        };
        if config.exclude_narrator_in_library_dir && !meta.narrators.is_empty() && dir.exists() {
            dir = library_dir(false, library, &meta).unwrap();
        }
        let metadata = abs::create_metadata(&meta);

        create_dir_all(&dir).await?;
        for file in audio_files {
            let file_path = PathBuf::from(&file.file_name());
            let library_path = dir.join(&file_path);
            library_files.push(file_path.clone());
            let download_path = file.path();
            match library.options().method {
                LibraryLinkMethod::Hardlink => {
                    hard_link(&download_path, &library_path, &file_path)?
                }
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
                LibraryLinkMethod::NoLink => {}
            };
        }
        library_files.sort();

        let file = File::create(dir.join("metadata.json"))?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &metadata)?;
        writer.flush()?;
        Some(dir.clone())
    } else {
        None
    };

    {
        let (_guard, rw) = db.rw_async().await?;
        torrent.library_path = library_path.clone();
        torrent.library_files = library_files;
        torrent.selected_audio_format = selected_audio_format;
        torrent.selected_ebook_format = selected_ebook_format;
        rw.upsert(torrent)?;
        rw.commit()?;
    }

    if let Some(library_path) = library_path {
        write_event(
            db,
            Event::new(
                Some(libation_meta.asin),
                None,
                EventType::Linked {
                    linker: library.options().name.clone(),
                    library_path,
                },
            ),
        )
        .await;
    }

    Ok(())
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Libation {
    pub asin: String,
    pub authors: Vec<Name>,
    pub category_ladders: Vec<CategoryLadder>,
    pub format_type: String,
    pub is_adult_product: bool,
    pub issue_date: String,
    pub language: String,
    pub merchandising_summary: String,
    pub narrators: Vec<Name>,
    pub publication_datetime: String,
    pub publication_name: String,
    pub publisher_name: String,
    pub publisher_summary: String,
    pub release_date: String,
    pub runtime_length_min: u64,
    #[serde(default)]
    pub series: Vec<LibationSeries>,
    pub subtitle: String,
    pub title: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryLadder {
    pub ladder: Vec<Ladder>,
    pub root: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ladder {
    pub id: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Name {
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibationSeries {
    pub sequence: String,
    pub title: String,
}
