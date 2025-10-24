use super::{v01, v03};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::{OffsetDateTime, UtcOffset};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 2, from = v01::Torrent)]
#[native_db]
pub struct Torrent {
    #[primary_key]
    pub hash: String,
    pub library_path: Option<PathBuf>,
    pub library_files: Vec<PathBuf>,
    #[secondary_key]
    pub title_search: String,
    pub meta: v01::TorrentMeta,
    pub created_at: OffsetDateTime,
    pub replaced_with: Option<(String, OffsetDateTime)>,
    pub request_matadata_update: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 2, from = v01::SelectedTorrent)]
#[native_db]
pub struct SelectedTorrent {
    #[primary_key]
    pub mam_id: u64,
    pub dl_link: String,
    pub unsat_buffer: Option<u64>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: v01::TorrentMeta,
    pub created_at: OffsetDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 4, version = 2, from = v01::DuplicateTorrent)]
#[native_db]
pub struct DuplicateTorrent {
    #[primary_key]
    pub mam_id: u64,
    #[secondary_key]
    pub title_search: String,
    pub meta: v01::TorrentMeta,
    pub created_at: OffsetDateTime,
    pub duplicate_of: Option<String>,
    pub request_replace: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 5, version = 2, from = v01::ErroredTorrent)]
#[native_db]
pub struct ErroredTorrent {
    #[primary_key]
    pub id: v01::ErroredTorrentId,
    pub title: String,
    pub error: String,
    pub meta: Option<v01::TorrentMeta>,
    pub created_at: OffsetDateTime,
}

impl From<v01::Torrent> for Torrent {
    fn from(t: v01::Torrent) -> Self {
        Self {
            hash: t.hash,
            library_path: t.library_path,
            library_files: t.library_files,
            title_search: t.title_search,
            meta: t.meta,
            created_at: OffsetDateTime::now_utc(),
            replaced_with: t.replaced_with.map(|r| (r, OffsetDateTime::now_utc())),
            request_matadata_update: t.request_matadata_update,
        }
    }
}

impl From<v01::SelectedTorrent> for SelectedTorrent {
    fn from(t: v01::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

impl From<v01::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v01::DuplicateTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            title_search: t.title_search,
            meta: t.meta,
            duplicate_of: t.duplicate_of,
            request_replace: t.request_replace,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

impl From<v01::ErroredTorrent> for ErroredTorrent {
    fn from(t: v01::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

impl From<v03::Torrent> for Torrent {
    fn from(t: v03::Torrent) -> Self {
        Self {
            hash: t.hash,
            library_path: t.library_path,
            library_files: t.library_files,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at.0.to_offset(UtcOffset::UTC),
            replaced_with: t
                .replaced_with
                .map(|(with, when)| (with, when.0.to_offset(UtcOffset::UTC))),
            request_matadata_update: t.request_matadata_update,
        }
    }
}

impl From<v03::SelectedTorrent> for SelectedTorrent {
    fn from(t: v03::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at.0.to_offset(UtcOffset::UTC),
        }
    }
}

impl From<v03::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v03::DuplicateTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            title_search: t.title_search,
            meta: t.meta.into(),
            duplicate_of: t.duplicate_of,
            request_replace: t.request_replace,
            created_at: t.created_at.0.to_offset(UtcOffset::UTC),
        }
    }
}

impl From<v03::ErroredTorrent> for ErroredTorrent {
    fn from(t: v03::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(Into::into),
            created_at: t.created_at.0.to_offset(UtcOffset::UTC),
        }
    }
}

impl From<v03::TorrentMeta> for v01::TorrentMeta {
    fn from(t: v03::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            main_cat: t.main_cat,
            filetypes: t.filetypes,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
        }
    }
}
