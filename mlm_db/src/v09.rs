use super::{v01, v03, v04, v06, v08, v10};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::warn;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 9, from = v08::Torrent)]
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
    pub created_at: v03::Timestamp,
    pub replaced_with: Option<(String, v03::Timestamp)>,
    pub request_matadata_update: bool,
    pub library_mismatch: Option<v08::LibraryMismatch>,
    pub client_status: Option<v08::ClientStatus>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 9, from = v08::SelectedTorrent)]
#[native_db]
pub struct SelectedTorrent {
    #[primary_key]
    pub mam_id: u64,
    pub dl_link: String,
    pub unsat_buffer: Option<u64>,
    pub cost: v04::TorrentCost,
    pub category: Option<String>,
    pub tags: Vec<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    pub created_at: v03::Timestamp,
    pub removed_at: Option<v03::Timestamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 4, version = 9, from = v08::DuplicateTorrent)]
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
#[native_model(id = 5, version = 9, from = v08::ErroredTorrent)]
#[native_db]
pub struct ErroredTorrent {
    #[primary_key]
    pub id: v01::ErroredTorrentId,
    pub title: String,
    pub error: String,
    pub meta: Option<TorrentMeta>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TorrentMeta {
    pub mam_id: u64,
    pub main_cat: v01::MainCat,
    pub cat: Option<v06::Category>,
    pub language: Option<v03::Language>,
    pub flags: Option<v08::FlagBits>,
    pub filetypes: Vec<String>,
    pub size: v03::Size,
    pub title: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<Series>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Series {
    pub name: String,
    pub entries: SeriesEntries,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SeriesEntries(pub Vec<SeriesEntry>);

impl SeriesEntries {
    pub fn new(entries: Vec<SeriesEntry>) -> SeriesEntries {
        SeriesEntries(entries)
    }
}

impl AsRef<Vec<SeriesEntry>> for SeriesEntries {
    fn as_ref(&self) -> &Vec<SeriesEntry> {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SeriesEntry {
    Num(f32),
    Range(f32, f32),
    Part(f32, f32),
}

impl From<v08::Torrent> for Torrent {
    fn from(t: v08::Torrent) -> Self {
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
            library_mismatch: t.library_mismatch,
            client_status: t.client_status,
        }
    }
}

impl From<v08::SelectedTorrent> for SelectedTorrent {
    fn from(t: v08::SelectedTorrent) -> Self {
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

impl From<v08::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v08::DuplicateTorrent) -> Self {
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

impl From<v08::ErroredTorrent> for ErroredTorrent {
    fn from(t: v08::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v08::TorrentMeta> for TorrentMeta {
    fn from(t: v08::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            main_cat: t.main_cat,
            cat: t.cat,
            language: t.language,
            flags: t.flags,
            filetypes: t.filetypes,
            size: t.size,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t
                .series
                .into_iter()
                .map(|(name, num)| {
                    Series::try_from((name.clone(), num.clone())).unwrap_or_else(|err| {
                        warn!("error parsing series num: {err}, num: \"{num}\"");
                        Series {
                            name,
                            entries: SeriesEntries::new(vec![]),
                        }
                    })
                })
                .collect(),
        }
    }
}

impl From<v10::Torrent> for Torrent {
    fn from(t: v10::Torrent) -> Self {
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
            library_mismatch: t.library_mismatch,
            client_status: t.client_status,
        }
    }
}

impl From<v10::SelectedTorrent> for SelectedTorrent {
    fn from(t: v10::SelectedTorrent) -> Self {
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
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v10::TorrentMeta> for TorrentMeta {
    fn from(t: v10::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
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
        }
    }
}
