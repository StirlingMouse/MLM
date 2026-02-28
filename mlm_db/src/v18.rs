use crate::ids;

use super::{v01, v03, v04, v05, v08, v09, v10, v11, v12, v13, v16, v17};
use mlm_parse::{normalize_title, parse_edition};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 18, from = v17::Torrent)]
#[native_db(export_keys = true)]
pub struct Torrent {
    #[primary_key]
    pub id: String,
    pub id_is_hash: bool,
    #[secondary_key(unique, optional)]
    pub mam_id: Option<u64>,
    pub library_path: Option<PathBuf>,
    pub library_files: Vec<PathBuf>,
    pub linker: Option<String>,
    pub category: Option<String>,
    pub selected_audio_format: Option<String>,
    pub selected_ebook_format: Option<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    #[secondary_key]
    pub created_at: v03::Timestamp,
    pub replaced_with: Option<(String, v03::Timestamp)>,
    pub library_mismatch: Option<v08::LibraryMismatch>,
    pub client_status: Option<ClientStatus>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ClientStatus {
    NotInClient,
    RemovedFromTracker,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 18, from = v17::SelectedTorrent)]
#[native_db(export_keys = true)]
pub struct SelectedTorrent {
    #[primary_key]
    pub mam_id: u64,
    #[secondary_key(unique, optional)]
    pub hash: Option<String>,
    pub dl_link: String,
    pub unsat_buffer: Option<u64>,
    pub wedge_buffer: Option<u64>,
    pub cost: v04::TorrentCost,
    pub category: Option<String>,
    pub tags: Vec<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    pub grabber: Option<String>,
    pub created_at: v03::Timestamp,
    pub started_at: Option<v03::Timestamp>,
    pub removed_at: Option<v03::Timestamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 4, version = 18, from = v17::DuplicateTorrent)]
#[native_db]
pub struct DuplicateTorrent {
    #[primary_key]
    pub mam_id: u64,
    pub dl_link: Option<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    pub created_at: v03::Timestamp,
    pub duplicate_of: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 5, version = 18, from = v17::ErroredTorrent)]
#[native_db(export_keys = true)]
pub struct ErroredTorrent {
    #[primary_key]
    pub id: v11::ErroredTorrentId,
    pub title: String,
    pub error: String,
    pub meta: Option<TorrentMeta>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct TorrentMeta {
    pub ids: BTreeMap<String, String>,
    pub vip_status: Option<v11::VipStatus>,
    pub cat: Option<v16::OldCategory>,
    pub media_type: v13::MediaType,
    pub main_cat: Option<v12::MainCat>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub language: Option<v03::Language>,
    pub flags: Option<v08::FlagBits>,
    pub filetypes: Vec<String>,
    pub num_files: u64,
    pub size: v03::Size,
    pub title: String,
    pub edition: Option<(String, u64)>,
    pub description: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<v09::Series>,
    pub source: MetadataSource,
    pub uploaded_at: v03::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub enum MetadataSource {
    #[default]
    Mam,
    Manual,
    File,
    Match,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[native_model(id = 6, version = 18, from = v17::Event)]
#[native_db(export_keys = true)]
pub struct Event {
    #[primary_key]
    pub id: v03::Uuid,
    #[secondary_key]
    pub torrent_id: Option<String>,
    #[secondary_key]
    pub mam_id: Option<u64>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
    pub event: EventType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    Grabbed {
        grabber: Option<String>,
        cost: Option<v04::TorrentCost>,
        wedged: bool,
    },
    Linked {
        linker: Option<String>,
        library_path: PathBuf,
    },
    Cleaned {
        library_path: PathBuf,
        files: Vec<PathBuf>,
    },
    Updated {
        fields: Vec<TorrentMetaDiff>,
        source: (MetadataSource, String),
    },
    RemovedFromTracker,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TorrentMetaDiff {
    pub field: TorrentMetaField,
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum TorrentMetaField {
    Ids,
    Vip,
    Cat,
    MediaType,
    MainCat,
    Categories,
    Tags,
    Language,
    Flags,
    Filetypes,
    Size,
    Title,
    Edition,
    Authors,
    Narrators,
    Series,
    Source,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 8, version = 18, from = v05::ListItem)]
#[native_db(export_keys = true)]
pub struct ListItem {
    #[primary_key]
    pub guid: (String, String),
    #[secondary_key]
    pub list_id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub series: Vec<(String, f64)>,
    pub cover_url: String,
    pub book_url: Option<String>,
    pub isbn: Option<u64>,
    pub prefer_format: Option<v01::MainCat>,
    pub allow_audio: bool,
    pub audio_torrent: Option<ListItemTorrent>,
    pub allow_ebook: bool,
    pub ebook_torrent: Option<ListItemTorrent>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
    pub marked_done_at: Option<v03::Timestamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListItemTorrent {
    pub torrent_id: Option<String>,
    pub mam_id: Option<u64>,
    pub status: v04::TorrentStatus,
    pub at: v03::Timestamp,
}

impl From<v17::Torrent> for Torrent {
    fn from(t: v17::Torrent) -> Self {
        let mut meta: TorrentMeta = t.meta.into();
        if let Some(abs_id) = t.abs_id {
            meta.ids.insert(ids::ABS.to_string(), abs_id.to_string());
        }
        if let Some(goodreads_id) = t.goodreads_id {
            meta.ids
                .insert(ids::GOODREADS.to_string(), goodreads_id.to_string());
        }

        Self {
            id: t.id,
            id_is_hash: t.id_is_hash,
            mam_id: Some(t.mam_id),
            library_path: t.library_path,
            library_files: t.library_files,
            linker: t.linker,
            category: t.category,
            selected_audio_format: t.selected_audio_format,
            selected_ebook_format: t.selected_ebook_format,
            title_search: normalize_title(&meta.title),
            meta,
            created_at: t.created_at,
            replaced_with: t.replaced_with,
            library_mismatch: t.library_mismatch,
            client_status: t.client_status.map(Into::into),
        }
    }
}

impl From<v08::ClientStatus> for ClientStatus {
    fn from(value: v08::ClientStatus) -> Self {
        match value {
            v08::ClientStatus::NotInClient => Self::NotInClient,
            v08::ClientStatus::RemovedFromMam => Self::RemovedFromTracker,
        }
    }
}

impl From<v17::SelectedTorrent> for SelectedTorrent {
    fn from(t: v17::SelectedTorrent) -> Self {
        let mut meta: TorrentMeta = t.meta.into();
        if let Some(goodreads_id) = t.goodreads_id {
            meta.ids
                .insert(ids::GOODREADS.to_string(), goodreads_id.to_string());
        }

        Self {
            mam_id: t.mam_id,
            hash: t.hash,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            wedge_buffer: None,
            cost: t.cost,
            category: t.category,
            tags: t.tags,
            title_search: normalize_title(&meta.title),
            meta,
            grabber: t.grabber,
            created_at: t.created_at,
            started_at: t.started_at,
            removed_at: t.removed_at,
        }
    }
}

impl From<v17::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v17::DuplicateTorrent) -> Self {
        let meta: TorrentMeta = t.meta.into();
        Self {
            mam_id: t.mam_id,
            dl_link: t.dl_link,
            title_search: normalize_title(&meta.title),
            meta,
            created_at: t.created_at,
            duplicate_of: t.duplicate_of,
        }
    }
}

impl From<v17::ErroredTorrent> for ErroredTorrent {
    fn from(t: v17::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v17::TorrentMeta> for TorrentMeta {
    fn from(t: v17::TorrentMeta) -> Self {
        let (title, edition) = parse_edition(&t.title, "");
        let mut ids = BTreeMap::default();
        ids.insert(ids::MAM.to_string(), t.mam_id.to_string());

        Self {
            ids,
            vip_status: t.vip_status,
            cat: t.cat,
            media_type: t.media_type,
            main_cat: t.main_cat,
            categories: t.categories.iter().map(ToString::to_string).collect(),
            tags: vec![],
            language: t.language,
            flags: t.flags,
            filetypes: t.filetypes,
            num_files: 0,
            size: t.size,
            title,
            edition,
            description: String::new(),
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
            source: t.source.into(),
            uploaded_at: t.uploaded_at,
        }
    }
}

impl From<v10::MetadataSource> for MetadataSource {
    fn from(t: v10::MetadataSource) -> Self {
        match t {
            v10::MetadataSource::Mam => Self::Mam,
            v10::MetadataSource::Manual => Self::Manual,
        }
    }
}

impl From<v17::Event> for Event {
    fn from(t: v17::Event) -> Self {
        Self {
            id: t.id,
            torrent_id: t.torrent_id,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v17::EventType> for EventType {
    fn from(t: v17::EventType) -> Self {
        match t {
            v17::EventType::Grabbed {
                grabber,
                cost,
                wedged,
            } => Self::Grabbed {
                grabber,
                cost,
                wedged,
            },
            v17::EventType::Linked {
                linker,
                library_path,
            } => Self::Linked {
                linker,
                library_path,
            },
            v17::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
            v17::EventType::Updated { fields } => Self::Updated {
                fields: fields.into_iter().map(Into::into).collect(),
                source: (MetadataSource::Mam, String::new()),
            },
            v17::EventType::RemovedFromMam => Self::RemovedFromTracker,
        }
    }
}

impl From<v17::TorrentMetaDiff> for TorrentMetaDiff {
    fn from(value: v17::TorrentMetaDiff) -> Self {
        Self {
            field: value.field.into(),
            from: value.from,
            to: value.to,
        }
    }
}

impl From<v17::TorrentMetaField> for TorrentMetaField {
    fn from(value: v17::TorrentMetaField) -> Self {
        match value {
            v17::TorrentMetaField::MamId => TorrentMetaField::Ids,
            v17::TorrentMetaField::Vip => TorrentMetaField::Vip,
            v17::TorrentMetaField::MediaType => TorrentMetaField::MediaType,
            v17::TorrentMetaField::MainCat => TorrentMetaField::MainCat,
            v17::TorrentMetaField::Categories => TorrentMetaField::Categories,
            v17::TorrentMetaField::Cat => TorrentMetaField::Cat,
            v17::TorrentMetaField::Language => TorrentMetaField::Language,
            v17::TorrentMetaField::Flags => TorrentMetaField::Flags,
            v17::TorrentMetaField::Filetypes => TorrentMetaField::Filetypes,
            v17::TorrentMetaField::Size => TorrentMetaField::Size,
            v17::TorrentMetaField::Title => TorrentMetaField::Title,
            v17::TorrentMetaField::Authors => TorrentMetaField::Authors,
            v17::TorrentMetaField::Narrators => TorrentMetaField::Narrators,
            v17::TorrentMetaField::Series => TorrentMetaField::Series,
            v17::TorrentMetaField::Source => TorrentMetaField::Source,
            v17::TorrentMetaField::Edition => TorrentMetaField::Edition,
        }
    }
}

impl From<v05::ListItem> for ListItem {
    fn from(t: v05::ListItem) -> Self {
        Self {
            guid: t.guid,
            list_id: t.list_id,
            title: t.title,
            authors: t.authors,
            series: t.series,
            cover_url: t.cover_url,
            book_url: t.book_url,
            isbn: t.isbn,
            prefer_format: t.prefer_format,
            allow_audio: t.allow_audio,
            audio_torrent: t.audio_torrent.map(Into::into),
            allow_ebook: t.allow_ebook,
            ebook_torrent: t.ebook_torrent.map(Into::into),
            created_at: t.created_at,
            marked_done_at: t.marked_done_at,
        }
    }
}

impl From<v04::ListItemTorrent> for ListItemTorrent {
    fn from(t: v04::ListItemTorrent) -> Self {
        Self {
            torrent_id: None,
            mam_id: Some(t.mam_id),
            status: t.status,
            at: t.at,
        }
    }
}
