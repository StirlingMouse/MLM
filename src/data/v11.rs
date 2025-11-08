use super::{v01, v03, v04, v06, v08, v09, v10, v12};
use native_db::{Key, ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::Date;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 11, from = v10::Torrent)]
#[native_db]
pub struct Torrent {
    #[primary_key]
    pub hash: String,
    #[secondary_key]
    pub mam_id: u64,
    pub abs_id: Option<String>,
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
    pub request_matadata_update: bool,
    pub library_mismatch: Option<v08::LibraryMismatch>,
    pub client_status: Option<v08::ClientStatus>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 11, from = v10::SelectedTorrent)]
#[native_db]
pub struct SelectedTorrent {
    #[primary_key]
    pub mam_id: u64,
    pub hash: Option<String>,
    pub dl_link: String,
    pub unsat_buffer: Option<u64>,
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
#[native_model(id = 4, version = 11, from = v10::DuplicateTorrent)]
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
#[native_model(id = 5, version = 11, from = v10::ErroredTorrent)]
#[native_db]
pub struct ErroredTorrent {
    #[primary_key]
    pub id: ErroredTorrentId,
    pub title: String,
    pub error: String,
    pub meta: Option<TorrentMeta>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ErroredTorrentId {
    Grabber(/* mam_id */ u64),
    Linker(/* hash */ String),
    Cleaner(/* hash */ String),
}

impl ToKey for ErroredTorrentId {
    fn to_key(&self) -> Key {
        match self {
            ErroredTorrentId::Grabber(mam_id) => {
                Key::new([&[0u8] as &[u8], &mam_id.to_le_bytes()].concat())
            }
            ErroredTorrentId::Linker(hash) => Key::new([&[1u8] as &[u8], hash.as_bytes()].concat()),
            ErroredTorrentId::Cleaner(hash) => {
                Key::new([&[2u8] as &[u8], hash.as_bytes()].concat())
            }
        }
    }

    fn key_names() -> Vec<String> {
        vec!["ErroredTorrentHash".to_string()]
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TorrentMeta {
    pub mam_id: u64,
    pub vip_status: Option<VipStatus>,
    pub main_cat: v01::MainCat,
    pub cat: Option<v06::Category>,
    pub language: Option<v03::Language>,
    pub flags: Option<v08::FlagBits>,
    pub filetypes: Vec<String>,
    pub size: v03::Size,
    pub title: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<v09::Series>,
    pub source: v10::MetadataSource,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum VipStatus {
    NotVip,
    Permanent,
    Temp(Date),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 6, version = 11, from = v10::Event)]
#[native_db]
pub struct Event {
    #[primary_key]
    pub id: v03::Uuid,
    #[secondary_key]
    pub hash: Option<String>,
    #[secondary_key]
    pub mam_id: Option<u64>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
    pub event: EventType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    },
    RemovedFromMam,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TorrentMetaDiff {
    pub field: TorrentMetaField,
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TorrentMetaField {
    MamId,
    Vip,
    MainCat,
    Cat,
    Language,
    Flags,
    Filetypes,
    Size,
    Title,
    Authors,
    Narrators,
    Series,
    Source,
}

impl From<v10::Torrent> for Torrent {
    fn from(t: v10::Torrent) -> Self {
        Self {
            hash: t.hash,
            mam_id: t.meta.mam_id,
            abs_id: t.abs_id,
            library_path: t.library_path,
            library_files: t.library_files,
            linker: None,
            category: None,
            selected_audio_format: t.selected_audio_format,
            selected_ebook_format: t.selected_ebook_format,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at,
            replaced_with: t.replaced_with,
            request_matadata_update: t.request_matadata_update,
            library_mismatch: t.library_mismatch,
            client_status: t.client_status,
        }
    }
}

impl From<v10::SelectedTorrent> for SelectedTorrent {
    fn from(t: v10::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            hash: None,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            cost: t.cost,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta.into(),
            grabber: t.grabber,
            created_at: t.created_at,
            started_at: None,
            removed_at: t.removed_at,
        }
    }
}

impl From<v10::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v10::DuplicateTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            dl_link: t.dl_link,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at,
            duplicate_of: t.duplicate_of,
        }
    }
}

impl From<v10::ErroredTorrent> for ErroredTorrent {
    fn from(t: v10::ErroredTorrent) -> Self {
        Self {
            id: t.id.into(),
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v01::ErroredTorrentId> for ErroredTorrentId {
    fn from(t: v01::ErroredTorrentId) -> Self {
        match t {
            v01::ErroredTorrentId::Grabber(id) => ErroredTorrentId::Grabber(id),
            v01::ErroredTorrentId::Linker(hash) => ErroredTorrentId::Linker(hash),
            v01::ErroredTorrentId::Cleaner(hash) => ErroredTorrentId::Cleaner(hash),
        }
    }
}

impl From<v10::TorrentMeta> for TorrentMeta {
    fn from(t: v10::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            vip_status: None,
            main_cat: t.main_cat,
            cat: t.cat,
            language: t.language,
            flags: t.flags,
            filetypes: t.filetypes,
            size: t.size,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
            source: t.source,
        }
    }
}

impl From<v10::Event> for Event {
    fn from(t: v10::Event) -> Self {
        Self {
            id: t.id,
            hash: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v10::EventType> for EventType {
    fn from(t: v10::EventType) -> Self {
        match t {
            v10::EventType::Grabbed {
                grabber,
                cost,
                wedged,
            } => Self::Grabbed {
                grabber,
                cost,
                wedged,
            },
            v10::EventType::Linked {
                linker,
                library_path,
            } => Self::Linked {
                linker,
                library_path,
            },
            v10::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
            v10::EventType::Updated { fields } => Self::Updated {
                fields: fields.into_iter().map(Into::into).collect(),
            },
            v10::EventType::RemovedFromMam => Self::RemovedFromMam,
        }
    }
}

impl From<v08::TorrentMetaDiff> for TorrentMetaDiff {
    fn from(value: v08::TorrentMetaDiff) -> Self {
        Self {
            field: value.field.into(),
            from: value.from,
            to: value.to,
        }
    }
}

impl From<v08::TorrentMetaField> for TorrentMetaField {
    fn from(value: v08::TorrentMetaField) -> Self {
        match value {
            v08::TorrentMetaField::MamId => TorrentMetaField::MamId,
            v08::TorrentMetaField::MainCat => TorrentMetaField::MainCat,
            v08::TorrentMetaField::Cat => TorrentMetaField::Cat,
            v08::TorrentMetaField::Language => TorrentMetaField::Language,
            v08::TorrentMetaField::Flags => TorrentMetaField::Flags,
            v08::TorrentMetaField::Filetypes => TorrentMetaField::Filetypes,
            v08::TorrentMetaField::Size => TorrentMetaField::Size,
            v08::TorrentMetaField::Title => TorrentMetaField::Title,
            v08::TorrentMetaField::Authors => TorrentMetaField::Authors,
            v08::TorrentMetaField::Narrators => TorrentMetaField::Narrators,
            v08::TorrentMetaField::Series => TorrentMetaField::Series,
        }
    }
}

impl From<v12::Torrent> for Torrent {
    fn from(t: v12::Torrent) -> Self {
        Self {
            hash: t.hash,
            mam_id: t.meta.mam_id,
            abs_id: t.abs_id,
            library_path: t.library_path,
            library_files: t.library_files,
            linker: None,
            category: None,
            selected_audio_format: t.selected_audio_format,
            selected_ebook_format: t.selected_ebook_format,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at,
            replaced_with: t.replaced_with,
            request_matadata_update: t.request_matadata_update,
            library_mismatch: t.library_mismatch,
            client_status: t.client_status,
        }
    }
}

impl From<v12::SelectedTorrent> for SelectedTorrent {
    fn from(t: v12::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            hash: None,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            cost: t.cost,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta.into(),
            grabber: t.grabber,
            created_at: t.created_at,
            started_at: None,
            removed_at: t.removed_at,
        }
    }
}

impl From<v12::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v12::DuplicateTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            dl_link: t.dl_link,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at,
            duplicate_of: t.duplicate_of,
        }
    }
}

impl From<v12::ErroredTorrent> for ErroredTorrent {
    fn from(t: v12::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v12::TorrentMeta> for TorrentMeta {
    fn from(t: v12::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            vip_status: None,
            main_cat: t.media_type.into(),
            cat: t.cat,
            language: t.language,
            flags: t.flags,
            filetypes: t.filetypes,
            size: t.size,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
            source: t.source,
        }
    }
}

impl From<v12::MediaType> for v01::MainCat {
    fn from(value: v12::MediaType) -> Self {
        match value {
            v12::MediaType::Audiobook => v01::MainCat::Audio,
            v12::MediaType::Ebook => v01::MainCat::Ebook,
            v12::MediaType::Musicology => unimplemented!(),
            v12::MediaType::Radio => unimplemented!(),
            v12::MediaType::Manga => v01::MainCat::Ebook,
            v12::MediaType::ComicBook => v01::MainCat::Ebook,
            v12::MediaType::Periodical => v01::MainCat::Ebook,
        }
    }
}

impl From<v12::Event> for Event {
    fn from(t: v12::Event) -> Self {
        Self {
            id: t.id,
            hash: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v12::EventType> for EventType {
    fn from(t: v12::EventType) -> Self {
        match t {
            v12::EventType::Grabbed {
                grabber,
                cost,
                wedged,
            } => Self::Grabbed {
                grabber,
                cost,
                wedged,
            },
            v12::EventType::Linked {
                linker,
                library_path,
            } => Self::Linked {
                linker,
                library_path,
            },
            v12::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
            v12::EventType::Updated { fields } => Self::Updated {
                fields: fields.into_iter().map(Into::into).collect(),
            },
            v12::EventType::RemovedFromMam => Self::RemovedFromMam,
        }
    }
}

impl From<v12::TorrentMetaDiff> for TorrentMetaDiff {
    fn from(value: v12::TorrentMetaDiff) -> Self {
        Self {
            field: value.field.into(),
            from: value.from,
            to: value.to,
        }
    }
}

impl From<v12::TorrentMetaField> for TorrentMetaField {
    fn from(value: v12::TorrentMetaField) -> Self {
        match value {
            v12::TorrentMetaField::MamId => TorrentMetaField::MamId,
            v12::TorrentMetaField::Vip => TorrentMetaField::Vip,
            v12::TorrentMetaField::Cat => TorrentMetaField::Cat,
            v12::TorrentMetaField::MediaType => TorrentMetaField::MainCat,
            v12::TorrentMetaField::MainCat => unimplemented!(),
            v12::TorrentMetaField::Categories => unimplemented!(),
            v12::TorrentMetaField::Language => TorrentMetaField::Language,
            v12::TorrentMetaField::Flags => TorrentMetaField::Flags,
            v12::TorrentMetaField::Filetypes => TorrentMetaField::Filetypes,
            v12::TorrentMetaField::Size => TorrentMetaField::Size,
            v12::TorrentMetaField::Title => TorrentMetaField::Title,
            v12::TorrentMetaField::Authors => TorrentMetaField::Authors,
            v12::TorrentMetaField::Narrators => TorrentMetaField::Narrators,
            v12::TorrentMetaField::Series => TorrentMetaField::Series,
            v12::TorrentMetaField::Source => TorrentMetaField::Source,
        }
    }
}
