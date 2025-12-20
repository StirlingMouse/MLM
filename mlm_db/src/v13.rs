use super::{v03, v04, v06, v08, v09, v10, v11, v12, v14};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 13, from = v12::Torrent)]
#[native_db]
pub struct Torrent {
    #[primary_key]
    pub hash: String,
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
#[native_model(id = 3, version = 13, from = v12::SelectedTorrent)]
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
#[native_model(id = 4, version = 13, from = v12::DuplicateTorrent)]
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
#[native_model(id = 5, version = 13, from = v12::ErroredTorrent)]
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
    pub cat: Option<v06::Category>,
    pub media_type: MediaType,
    pub main_cat: Option<v12::MainCat>,
    pub categories: Vec<v12::Category>,
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MediaType {
    Audiobook,
    Ebook,
    Musicology,
    Radio,
    Manga,
    #[doc(alias = "GraphicNovel")]
    ComicBook,
    PeriodicalEbook,
    PeriodicalAudiobook,
}

impl From<v12::Torrent> for Torrent {
    fn from(t: v12::Torrent) -> Self {
        Self {
            hash: t.hash,
            mam_id: t.meta.mam_id,
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

impl From<v12::SelectedTorrent> for SelectedTorrent {
    fn from(t: v12::SelectedTorrent) -> Self {
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
            vip_status: t.vip_status,
            cat: t.cat,
            media_type: t.media_type.into(),
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
        }
    }
}

impl From<v12::MediaType> for MediaType {
    fn from(value: v12::MediaType) -> Self {
        match value {
            v12::MediaType::Audiobook => MediaType::Audiobook,
            v12::MediaType::Ebook => MediaType::Ebook,
            v12::MediaType::Musicology => MediaType::Musicology,
            v12::MediaType::Radio => MediaType::Radio,
            v12::MediaType::Manga => MediaType::Manga,
            v12::MediaType::ComicBook => MediaType::ComicBook,
            v12::MediaType::Periodical => MediaType::PeriodicalEbook,
        }
    }
}

impl From<v14::Torrent> for Torrent {
    fn from(t: v14::Torrent) -> Self {
        Self {
            hash: t.hash,
            mam_id: t.meta.mam_id,
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

impl From<v14::SelectedTorrent> for SelectedTorrent {
    fn from(t: v14::SelectedTorrent) -> Self {
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

impl From<v14::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v14::DuplicateTorrent) -> Self {
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

impl From<v14::ErroredTorrent> for ErroredTorrent {
    fn from(t: v14::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v14::TorrentMeta> for TorrentMeta {
    fn from(t: v14::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            vip_status: t.vip_status,
            cat: t.cat,
            media_type: t.media_type,
            main_cat: t.main_cat,
            categories: t.categories.into_iter().map(|c| c.into()).collect(),
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
