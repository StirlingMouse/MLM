use super::{v03, v04, v06, v08, v09, v10, v11, v12, v13, v15, v17};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 16, from = v15::Torrent)]
#[native_db]
pub struct Torrent {
    #[primary_key]
    pub id: String,
    pub id_is_hash: bool,
    #[secondary_key(unique)]
    pub mam_id: u64,
    pub abs_id: Option<String>,
    pub goodreads_id: Option<u64>,
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
#[native_model(id = 3, version = 16, from = v15::SelectedTorrent)]
#[native_db]
pub struct SelectedTorrent {
    #[primary_key]
    pub mam_id: u64,
    pub goodreads_id: Option<u64>,
    #[secondary_key(unique, optional)]
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
#[native_model(id = 4, version = 16, from = v15::DuplicateTorrent)]
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
#[native_model(id = 5, version = 16, from = v15::ErroredTorrent)]
#[native_db]
pub struct ErroredTorrent {
    #[primary_key]
    pub id: v11::ErroredTorrentId,
    pub title: String,
    pub error: String,
    pub meta: Option<TorrentMeta>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TorrentMeta {
    pub mam_id: u64,
    pub vip_status: Option<v11::VipStatus>,
    pub cat: Option<OldCategory>,
    pub media_type: v13::MediaType,
    pub main_cat: Option<v12::MainCat>,
    pub categories: Vec<v15::Category>,
    pub language: Option<v03::Language>,
    pub flags: Option<v08::FlagBits>,
    pub filetypes: Vec<String>,
    pub size: v03::Size,
    pub title: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<v09::Series>,
    pub source: v10::MetadataSource,
    pub uploaded_at: v03::Timestamp,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MusicologyCategory {
    GuitarBassTabs,
    IndividualSheet,
    IndividualSheetMP3,
    InstructionalBookWithVideo,
    InstructionalMediaMusic,
    LickLibraryLTPJamWith,
    LickLibraryTechniquesQL,
    MusicCompleteEditions,
    MusicBook,
    MusicBookMP3,
    SheetCollection,
    SheetCollectionMP3,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RadioCategory {
    Comedy,
    Drama,
    FactualDocumentary,
    Reading,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum OldCategory {
    Audio(v06::AudiobookCategory),
    Ebook(v06::EbookCategory),
    Musicology(MusicologyCategory),
    Radio(RadioCategory),
}

impl From<v15::Torrent> for Torrent {
    fn from(t: v15::Torrent) -> Self {
        Self {
            id: t.id,
            id_is_hash: t.id_is_hash,
            mam_id: t.mam_id,
            abs_id: t.abs_id,
            goodreads_id: t.goodreads_id,
            library_path: t.library_path,
            library_files: t.library_files,
            linker: t.linker,
            category: t.category,
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

impl From<v15::SelectedTorrent> for SelectedTorrent {
    fn from(t: v15::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            goodreads_id: t.goodreads_id,
            hash: t.hash,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            cost: t.cost,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta.into(),
            grabber: t.grabber,
            created_at: t.created_at,
            started_at: t.started_at,
            removed_at: t.removed_at,
        }
    }
}

impl From<v15::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v15::DuplicateTorrent) -> Self {
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

impl From<v15::ErroredTorrent> for ErroredTorrent {
    fn from(t: v15::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v15::TorrentMeta> for TorrentMeta {
    fn from(t: v15::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            vip_status: t.vip_status,
            cat: t.cat.map(|c| c.into()),
            media_type: t.media_type,
            main_cat: t.main_cat,
            categories: t.categories,
            language: t.language,
            flags: t.flags,
            filetypes: t.filetypes,
            size: t.size,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
            source: t.source,
            uploaded_at: t.uploaded_at,
        }
    }
}

impl From<v06::Category> for OldCategory {
    fn from(value: v06::Category) -> Self {
        match value {
            v06::Category::Audio(audiobook_category) => Self::Audio(audiobook_category),
            v06::Category::Ebook(ebook_category) => Self::Ebook(ebook_category),
        }
    }
}

impl From<v17::Torrent> for Torrent {
    fn from(t: v17::Torrent) -> Self {
        Self {
            id: t.id,
            id_is_hash: t.id_is_hash,
            mam_id: t.mam_id,
            abs_id: t.abs_id,
            goodreads_id: t.goodreads_id,
            library_path: t.library_path,
            library_files: t.library_files,
            linker: t.linker,
            category: t.category,
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

impl From<v17::SelectedTorrent> for SelectedTorrent {
    fn from(t: v17::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            goodreads_id: t.goodreads_id,
            hash: t.hash,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            cost: t.cost,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta.into(),
            grabber: t.grabber,
            created_at: t.created_at,
            started_at: t.started_at,
            removed_at: t.removed_at,
        }
    }
}

impl From<v17::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v17::DuplicateTorrent) -> Self {
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
        Self {
            mam_id: t.mam_id,
            vip_status: t.vip_status,
            cat: t.cat,
            media_type: t.media_type,
            main_cat: t.main_cat,
            categories: t.categories,
            language: t.language,
            flags: t.flags,
            filetypes: t.filetypes,
            size: t.size,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
            source: t.source,
            uploaded_at: t.uploaded_at,
        }
    }
}
