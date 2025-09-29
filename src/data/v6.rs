use super::{v1, v3, v4, v5, v7, v8};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 6, from = v5::Torrent)]
#[native_db]
pub struct Torrent {
    #[primary_key]
    pub hash: String,
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
    pub library_mismatch: Option<v5::LibraryMismatch>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 6, from = v4::SelectedTorrent)]
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 4, version = 6, from = v3::DuplicateTorrent)]
#[native_db]
pub struct DuplicateTorrent {
    #[primary_key]
    pub mam_id: u64,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    pub created_at: v3::Timestamp,
    pub duplicate_of: Option<String>,
    pub request_replace: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 5, version = 6, from = v3::ErroredTorrent)]
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
    pub cat: Option<Category>,
    pub language: Option<v3::Language>,
    pub filetypes: Vec<String>,
    pub size: v3::Size,
    pub title: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudiobookCategory {
    ActionAdventure,
    Art,
    Biographical,
    Business,
    ComputerInternet,
    Crafts,
    CrimeThriller,
    Fantasy,
    Food,
    GeneralFiction,
    GeneralNonFic,
    HistoricalFiction,
    History,
    HomeGarden,
    Horror,
    Humor,
    Instructional,
    Juvenile,
    Language,
    LiteraryClassics,
    MathScienceTech,
    Medical,
    Mystery,
    Nature,
    Philosophy,
    PolSocRelig,
    Recreation,
    Romance,
    ScienceFiction,
    SelfHelp,
    TravelAdventure,
    TrueCrime,
    UrbanFantasy,
    Western,
    YoungAdult,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EbookCategory {
    ActionAdventure,
    Art,
    Biographical,
    Business,
    ComicsGraphicnovels,
    ComputerInternet,
    Crafts,
    CrimeThriller,
    Fantasy,
    Food,
    GeneralFiction,
    GeneralNonFiction,
    HistoricalFiction,
    History,
    HomeGarden,
    Horror,
    Humor,
    IllusionMagic,
    Instructional,
    Juvenile,
    Language,
    LiteraryClassics,
    MagazinesNewspapers,
    MathScienceTech,
    Medical,
    MixedCollections,
    Mystery,
    Nature,
    Philosophy,
    PolSocRelig,
    Recreation,
    Romance,
    ScienceFiction,
    SelfHelp,
    TravelAdventure,
    TrueCrime,
    UrbanFantasy,
    Western,
    YoungAdult,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Category {
    Audio(AudiobookCategory),
    Ebook(EbookCategory),
}

impl From<v5::Torrent> for Torrent {
    fn from(t: v5::Torrent) -> Self {
        Self {
            hash: t.hash,
            library_path: t.library_path,
            library_files: t.library_files,
            selected_audio_format: t.selected_audio_format,
            selected_ebook_format: t.selected_ebook_format,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at,
            replaced_with: t.replaced_with,
            request_matadata_update: t.request_matadata_update,
            library_mismatch: t.library_mismatch,
        }
    }
}

impl From<v4::SelectedTorrent> for SelectedTorrent {
    fn from(t: v4::SelectedTorrent) -> Self {
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
        }
    }
}

impl From<v3::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v3::DuplicateTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at,
            duplicate_of: t.duplicate_of,
            request_replace: t.request_replace,
        }
    }
}

impl From<v3::ErroredTorrent> for ErroredTorrent {
    fn from(t: v3::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v3::TorrentMeta> for TorrentMeta {
    fn from(t: v3::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            main_cat: t.main_cat,
            cat: None,
            language: t.language,
            filetypes: t.filetypes,
            size: t.size,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
        }
    }
}

impl From<v7::Torrent> for Torrent {
    fn from(t: v7::Torrent) -> Self {
        Self {
            hash: t.hash,
            library_path: t.library_path,
            library_files: t.library_files,
            selected_audio_format: t.selected_audio_format,
            selected_ebook_format: t.selected_ebook_format,
            title_search: t.title_search,
            meta: t.meta,
            created_at: t.created_at,
            replaced_with: t.replaced_with,
            request_matadata_update: t.request_matadata_update,
            library_mismatch: t.library_mismatch,
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
            meta: t.meta,
            created_at: t.created_at,
        }
    }
}

impl From<v7::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v7::DuplicateTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            title_search: t.title_search,
            meta: t.meta,
            created_at: t.created_at,
            duplicate_of: t.duplicate_of,
            request_replace: false,
        }
    }
}

impl From<v8::ErroredTorrent> for ErroredTorrent {
    fn from(t: v8::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v8::TorrentMeta> for TorrentMeta {
    fn from(t: v8::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            main_cat: t.main_cat,
            cat: None,
            language: t.language,
            filetypes: t.filetypes,
            size: t.size,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
        }
    }
}
