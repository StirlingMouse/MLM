use super::{v02, v11};
use native_db::{Key, ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
#[native_model(id = 1, version = 1)]
#[native_db]
pub struct Config {
    #[primary_key]
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 1)]
#[native_db]
pub struct Torrent {
    #[primary_key]
    pub hash: String,
    pub library_path: Option<PathBuf>,
    pub library_files: Vec<PathBuf>,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    pub replaced_with: Option<String>,
    pub request_matadata_update: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 1)]
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
    pub meta: TorrentMeta,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 4, version = 1)]
#[native_db]
pub struct DuplicateTorrent {
    #[primary_key]
    pub mam_id: u64,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    pub duplicate_of: Option<String>,
    pub request_replace: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 5, version = 1)]
#[native_db]
pub struct ErroredTorrent {
    #[primary_key]
    pub id: ErroredTorrentId,
    pub title: String,
    pub error: String,
    pub meta: Option<TorrentMeta>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TorrentMeta {
    pub mam_id: u64,
    pub main_cat: MainCat,
    pub filetypes: Vec<String>,
    pub title: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MainCat {
    Audio,
    Ebook,
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
            ErroredTorrentId::Linker(hash) => Key::new([&[0u8] as &[u8], hash.as_bytes()].concat()),
            ErroredTorrentId::Cleaner(hash) => {
                Key::new([&[0u8] as &[u8], hash.as_bytes()].concat())
            }
        }
    }

    fn key_names() -> Vec<String> {
        vec!["ErroredTorrentHash".to_string()]
    }
}

impl From<v02::Torrent> for Torrent {
    fn from(t: v02::Torrent) -> Self {
        Self {
            hash: t.hash,
            library_path: t.library_path,
            library_files: t.library_files,
            title_search: t.title_search,
            meta: t.meta,
            replaced_with: t.replaced_with.map(|(r, _)| r),
            request_matadata_update: t.request_matadata_update,
        }
    }
}

impl From<v02::SelectedTorrent> for SelectedTorrent {
    fn from(t: v02::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta,
        }
    }
}

impl From<v02::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v02::DuplicateTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            title_search: t.title_search,
            meta: t.meta,
            duplicate_of: t.duplicate_of,
            request_replace: t.request_replace,
        }
    }
}

impl From<v02::ErroredTorrent> for ErroredTorrent {
    fn from(t: v02::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta,
        }
    }
}

impl From<v11::ErroredTorrentId> for ErroredTorrentId {
    fn from(t: v11::ErroredTorrentId) -> Self {
        match t {
            v11::ErroredTorrentId::Grabber(id) => ErroredTorrentId::Grabber(id),
            v11::ErroredTorrentId::Linker(hash) => ErroredTorrentId::Linker(hash),
            v11::ErroredTorrentId::Cleaner(hash) => ErroredTorrentId::Cleaner(hash),
        }
    }
}
