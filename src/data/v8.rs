use super::{v1, v3, v4, v5, v6, v7};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 8, from = v7::Torrent)]
#[native_db]
pub struct Torrent {
    #[primary_key]
    pub hash: String,
    #[secondary_key]
    pub mam_id: u64,
    pub abs_id: Option<String>,
    pub library_path: Option<PathBuf>,
    pub library_files: Vec<PathBuf>,
    pub selected_audio_format: Option<String>,
    pub selected_ebook_format: Option<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    #[secondary_key]
    pub created_at: v3::Timestamp,
    pub replaced_with: Option<(String, v3::Timestamp)>,
    pub request_matadata_update: bool,
    pub library_mismatch: Option<LibraryMismatch>,
    pub client_status: Option<ClientStatus>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum LibraryMismatch {
    NewLibraryDir(PathBuf),
    NewPath(PathBuf),
    NoLibrary,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ClientStatus {
    NotInClient,
    RemovedFromMam,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 8, from = v7::SelectedTorrent)]
#[native_db]
pub struct SelectedTorrent {
    #[primary_key]
    pub mam_id: u64,
    pub dl_link: String,
    pub unsat_buffer: Option<u64>,
    pub cost: v4::TorrentCost,
    pub category: Option<String>,
    pub tags: Vec<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    pub created_at: v3::Timestamp,
    pub removed_at: Option<v3::Timestamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 4, version = 8, from = v7::DuplicateTorrent)]
#[native_db]
pub struct DuplicateTorrent {
    #[primary_key]
    pub mam_id: u64,
    pub dl_link: Option<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    pub created_at: v3::Timestamp,
    pub duplicate_of: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 5, version = 8, from = v6::ErroredTorrent)]
#[native_db]
pub struct ErroredTorrent {
    #[primary_key]
    pub id: v1::ErroredTorrentId,
    pub title: String,
    pub error: String,
    pub meta: Option<TorrentMeta>,
    #[secondary_key]
    pub created_at: v3::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TorrentMeta {
    pub mam_id: u64,
    pub main_cat: v1::MainCat,
    pub cat: Option<v6::Category>,
    pub language: Option<v3::Language>,
    pub flags: Option<FlagBits>,
    pub filetypes: Vec<String>,
    pub size: v3::Size,
    pub title: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagBits(pub u8);

impl FlagBits {
    pub fn new(bits: u8) -> FlagBits {
        FlagBits(bits)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 6, version = 8, from = v7::Event)]
#[native_db]
pub struct Event {
    #[primary_key]
    pub id: v3::Uuid,
    #[secondary_key]
    pub hash: Option<String>,
    #[secondary_key]
    pub mam_id: Option<u64>,
    #[secondary_key]
    pub created_at: v3::Timestamp,
    pub event: EventType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EventType {
    Grabbed {
        cost: Option<v4::TorrentCost>,
        wedged: bool,
    },
    Linked {
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
}

impl From<v7::Torrent> for Torrent {
    fn from(t: v7::Torrent) -> Self {
        Self {
            hash: t.hash,
            mam_id: t.meta.mam_id,
            abs_id: t.abs_id,
            library_path: t.library_path,
            library_files: t.library_files,
            selected_audio_format: t.selected_audio_format,
            selected_ebook_format: t.selected_ebook_format,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at,
            replaced_with: t.replaced_with,
            request_matadata_update: t.request_matadata_update,
            library_mismatch: t.library_mismatch.map(Into::into),
            client_status: None,
        }
    }
}

impl From<v5::LibraryMismatch> for LibraryMismatch {
    fn from(value: v5::LibraryMismatch) -> Self {
        match value {
            v5::LibraryMismatch::NewPath(path_buf) => LibraryMismatch::NewLibraryDir(path_buf),
            v5::LibraryMismatch::NoLibrary => LibraryMismatch::NoLibrary,
            v5::LibraryMismatch::TorrentRemoved => unimplemented!(),
        }
    }
}

impl From<v7::SelectedTorrent> for SelectedTorrent {
    fn from(t: v7::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            cost: t.cost,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at,
            removed_at: t.removed_at,
        }
    }
}

impl From<v7::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v7::DuplicateTorrent) -> Self {
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

impl From<v6::ErroredTorrent> for ErroredTorrent {
    fn from(t: v6::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v6::TorrentMeta> for TorrentMeta {
    fn from(t: v6::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            main_cat: t.main_cat,
            cat: t.cat,
            language: t.language,
            flags: None,
            filetypes: t.filetypes,
            size: t.size,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
        }
    }
}

impl From<v7::Event> for Event {
    fn from(t: v7::Event) -> Self {
        Self {
            id: t.id,
            hash: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v7::EventType> for EventType {
    fn from(t: v7::EventType) -> Self {
        match t {
            v7::EventType::Grabbed { cost, wedged } => Self::Grabbed { cost, wedged },
            v7::EventType::Linked { library_path } => Self::Linked { library_path },
            v7::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
            v7::EventType::Updated { fields } => Self::Updated {
                fields: fields
                    .into_iter()
                    .map(|f| TorrentMetaDiff {
                        field: f.field.into(),
                        from: f.from,
                        to: f.to,
                    })
                    .collect(),
            },
        }
    }
}

impl From<v7::TorrentMetaField> for TorrentMetaField {
    fn from(value: v7::TorrentMetaField) -> Self {
        match value {
            v7::TorrentMetaField::MamId => TorrentMetaField::MamId,
            v7::TorrentMetaField::MainCat => TorrentMetaField::MainCat,
            v7::TorrentMetaField::Cat => TorrentMetaField::Cat,
            v7::TorrentMetaField::Language => TorrentMetaField::Language,
            v7::TorrentMetaField::Filetypes => TorrentMetaField::Filetypes,
            v7::TorrentMetaField::Size => TorrentMetaField::Size,
            v7::TorrentMetaField::Title => TorrentMetaField::Title,
            v7::TorrentMetaField::Authors => TorrentMetaField::Authors,
            v7::TorrentMetaField::Narrators => TorrentMetaField::Narrators,
            v7::TorrentMetaField::Series => TorrentMetaField::Series,
        }
    }
}
