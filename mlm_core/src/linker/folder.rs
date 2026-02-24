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
pub async fn link_folders_to_library(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    events: &crate::stats::Events,
) -> Result<()> {
    for library in &config.libraries {
        if let Library::ByRipDir(l) = library {
            let mut entries = read_dir(&l.rip_dir).await?;
            while let Some(folder) = entries.next_entry().await? {
                link_folder(&config, library, &db, folder, events).await?;
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
    events: &crate::stats::Events,
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
    let mut metadata_files = vec![];
    // let mut cover_file = None;
    while let Some(entry) = entries.next_entry().await? {
        match entry.path().extension() {
            Some(ext) if ext == "json" => metadata_files.push(entry),
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

    if metadata_files.is_empty() {
        warn!("Missing metadata file");
        return Ok(());
    }

    metadata_files.sort_by_key(|entry| entry.file_name());
    for metadata_file in metadata_files {
        let json = read_to_string(metadata_file.path()).await?;
        if let Ok(libation_meta) = serde_json::from_str::<Libation>(&json) {
            trace!("Linking libation folder");
            let asin = libation_meta.asin.clone();
            let title = libation_meta.title.clone();
            let result = link_libation_folder(
                config,
                library,
                db,
                libation_meta,
                audio_files,
                ebook_files,
                events,
            )
            .await;
            update_errored_torrent(db, ErroredTorrentId::Linker(asin), title, result).await;
            return Ok(());
        }
        if let Some(nextory_meta) = parse_nextory_meta(&json) {
            trace!("Linking nextory folder");
            let id = nextory_torrent_id(nextory_meta.id);
            let title = nextory_meta.title.clone();
            let result = link_nextory_folder(
                config,
                library,
                db,
                nextory_meta,
                audio_files,
                ebook_files,
                events,
            )
            .await;
            update_errored_torrent(db, ErroredTorrentId::Linker(id), title, result).await;
            return Ok(());
        }
    }

    warn!(
        folder = folder.path().to_string_lossy().to_string(),
        "Unsupported metadata format"
    );

    Ok(())
}

async fn link_libation_folder(
    config: &Config,
    library: &Library,
    db: &Database<'_>,
    libation_meta: Libation,
    audio_files: Vec<DirEntry>,
    ebook_files: Vec<DirEntry>,
    events: &crate::stats::Events,
) -> Result<()> {
    let torrent =
        build_libation_torrent(library, libation_meta, &audio_files, &ebook_files).await?;
    link_prepared_folder_torrent(
        config,
        library,
        db,
        torrent,
        audio_files,
        ebook_files,
        events,
    )
    .await
}

async fn link_nextory_folder(
    config: &Config,
    library: &Library,
    db: &Database<'_>,
    nextory_meta: NextoryRaw,
    audio_files: Vec<DirEntry>,
    ebook_files: Vec<DirEntry>,
    events: &crate::stats::Events,
) -> Result<()> {
    let torrent = build_nextory_torrent(library, nextory_meta, &audio_files, &ebook_files).await?;
    link_prepared_folder_torrent(
        config,
        library,
        db,
        torrent,
        audio_files,
        ebook_files,
        events,
    )
    .await
}

async fn build_libation_torrent(
    library: &Library,
    libation_meta: Libation,
    audio_files: &[DirEntry],
    ebook_files: &[DirEntry],
) -> Result<Torrent> {
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
    let (size, filetypes) = folder_file_stats(audio_files, ebook_files).await?;

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
        filetypes,
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
    build_torrent(library, libation_meta.asin, clean_meta(meta, "")?)
}

async fn build_nextory_torrent(
    library: &Library,
    nextory_meta: NextoryRaw,
    audio_files: &[DirEntry],
    ebook_files: &[DirEntry],
) -> Result<Torrent> {
    let mut series = vec![];
    if let Some(raw_series) = nextory_meta.series
        && !raw_series.name.is_empty()
    {
        let sequence = nextory_meta
            .volume
            .map(|v| v.to_string())
            .unwrap_or_default();
        if let Ok(parsed) = Series::try_from((raw_series.name, sequence)) {
            series.push(parsed);
        }
    }
    if series.is_empty()
        && let Some((name, num)) = parse_series_from_title(&nextory_meta.title)
    {
        series.push(Series {
            name: name.to_string(),
            entries: SeriesEntries::new(num.into_iter().map(SeriesEntry::Num).collect()),
        });
    }

    let mut ids = BTreeMap::new();
    ids.insert(ids::NEXTORY.to_string(), nextory_meta.id.to_string());
    if let Some(isbn) = nextory_isbn(&nextory_meta.formats) {
        ids.insert(ids::ISBN.to_string(), isbn);
    }
    let (size, filetypes) = folder_file_stats(audio_files, ebook_files).await?;

    let description = if nextory_meta.description_full.is_empty() {
        nextory_meta.blurb
    } else {
        nextory_meta.description_full
    };
    let meta = TorrentMeta {
        ids,
        vip_status: None,
        cat: None,
        media_type: MediaType::Audiobook,
        main_cat: None,
        categories: vec![],
        tags: vec![],
        language: parse_nextory_language(&nextory_meta.language),
        flags: None,
        filetypes,
        num_files: audio_files.len() as u64,
        size: Size::from_bytes(size),
        title: nextory_meta.title,
        edition: None,
        description,
        authors: nextory_meta.authors.into_iter().map(|a| a.name).collect(),
        narrators: nextory_meta.narrators.into_iter().map(|n| n.name).collect(),
        series,
        source: MetadataSource::File,
        uploaded_at: Timestamp::from(UtcDateTime::UNIX_EPOCH),
    };
    build_torrent(
        library,
        nextory_torrent_id(nextory_meta.id),
        clean_meta(meta, "")?,
    )
}

async fn folder_file_stats(
    audio_files: &[DirEntry],
    ebook_files: &[DirEntry],
) -> Result<(u64, Vec<String>)> {
    let mut size = 0;
    let mut filetypes = vec![];
    for file in audio_files {
        size += file_size(&file.metadata().await?);
        if let Some(ext) = file.path().extension() {
            filetypes.push(ext.to_string_lossy().to_lowercase());
        }
    }
    for file in ebook_files {
        size += file_size(&file.metadata().await?);
        if let Some(ext) = file.path().extension() {
            filetypes.push(ext.to_string_lossy().to_lowercase());
        }
    }
    filetypes.sort();
    filetypes.dedup();
    Ok((size, filetypes))
}

fn build_torrent(library: &Library, id: String, meta: TorrentMeta) -> Result<Torrent> {
    Ok(Torrent {
        id,
        id_is_hash: false,
        mam_id: meta.mam_id(),
        library_path: None,
        library_files: vec![],
        linker: library.options().name.clone(),
        category: None,
        selected_audio_format: None,
        selected_ebook_format: None,
        title_search: normalize_title(&meta.title),
        meta,
        created_at: Timestamp::now(),
        replaced_with: None,
        library_mismatch: None,
        client_status: None,
    })
}

async fn link_prepared_folder_torrent(
    config: &Config,
    library: &Library,
    db: &Database<'_>,
    mut torrent: Torrent,
    audio_files: Vec<DirEntry>,
    ebook_files: Vec<DirEntry>,
    events: &crate::stats::Events,
) -> Result<()> {
    let torrent_id = torrent.id.clone();
    let r = db.r_transaction()?;
    let existing_torrent: Option<Torrent> = r.get().primary(torrent_id.clone())?;
    if existing_torrent.is_some() {
        return Ok(());
    }

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
        && !filter
            .matches_meta(&torrent.meta)
            .is_ok_and(|matches: bool| matches)
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
        let Some(mut dir) = library_dir(
            config.exclude_narrator_in_library_dir,
            library,
            &torrent.meta,
        ) else {
            bail!("Torrent has no author");
        };
        if config.exclude_narrator_in_library_dir
            && !torrent.meta.narrators.is_empty()
            && dir.exists()
        {
            dir = library_dir(false, library, &torrent.meta).unwrap();
        }
        let metadata = abs::create_metadata(&torrent.meta);

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
            events,
            Event::new(
                Some(torrent_id),
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

fn parse_nextory_meta(json: &str) -> Option<NextoryRaw> {
    if let Ok(meta) = serde_json::from_str::<NextoryWrapped>(json) {
        return Some(meta.raw);
    }
    serde_json::from_str::<NextoryRaw>(json).ok()
}

fn nextory_torrent_id(nextory_id: u64) -> String {
    format!("nextory_{nextory_id}")
}

fn nextory_isbn(formats: &[NextoryFormat]) -> Option<String> {
    formats
        .iter()
        .find(|f| f.format_type == "hls")
        .or_else(|| formats.first())
        .and_then(|f| f.isbn.clone())
}

fn parse_nextory_language(value: &str) -> Option<Language> {
    if let Ok(language) = Language::from_str(value) {
        return Some(language);
    }
    match value.to_lowercase().as_str() {
        "sv" | "swe" => Some(Language::Swedish),
        "en" | "eng" => Some(Language::English),
        _ => None,
    }
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NextoryWrapped {
    pub raw: NextoryRaw,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NextoryRaw {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub blurb: String,
    #[serde(default)]
    pub description_full: String,
    pub language: String,
    #[serde(default)]
    pub volume: Option<f64>,
    #[serde(default)]
    pub series: Option<NextorySeries>,
    #[serde(default)]
    pub formats: Vec<NextoryFormat>,
    #[serde(default)]
    pub authors: Vec<NextoryName>,
    #[serde(default)]
    pub narrators: Vec<NextoryName>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NextorySeries {
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NextoryFormat {
    #[serde(rename = "type")]
    pub format_type: String,
    #[serde(default)]
    pub isbn: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NextoryName {
    pub name: String,
}
