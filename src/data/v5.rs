use super::{v1, v3, v4, v6};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 5, from = v3::Torrent)]
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
    pub meta: v3::TorrentMeta,
    #[secondary_key]
    pub created_at: v3::Timestamp,
    pub replaced_with: Option<(String, v3::Timestamp)>,
    pub request_matadata_update: bool,
    pub library_mismatch: Option<LibraryMismatch>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum LibraryMismatch {
    NewPath(PathBuf),
    NoLibrary,
    TorrentRemoved,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 7, version = 5, from = v4::List)]
#[native_db]
pub struct List {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub title: String,
    pub updated_at: Option<v3::Timestamp>,
    pub build_date: Option<v3::Timestamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 8, version = 5, from = v4::ListItem)]
#[native_db]
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
    pub prefer_format: Option<v1::MainCat>,
    pub allow_audio: bool,
    pub audio_torrent: Option<v4::ListItemTorrent>,
    pub allow_ebook: bool,
    pub ebook_torrent: Option<v4::ListItemTorrent>,
    #[secondary_key]
    pub created_at: v3::Timestamp,
    pub marked_done_at: Option<v3::Timestamp>,
}

impl From<v3::Torrent> for Torrent {
    fn from(t: v3::Torrent) -> Self {
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
            library_mismatch: None,
        }
    }
}

impl From<v4::List> for List {
    fn from(t: v4::List) -> Self {
        Self {
            id: t.id,
            title: t.title,
            updated_at: None,
            build_date: None,
        }
    }
}

impl From<v4::ListItem> for ListItem {
    fn from(t: v4::ListItem) -> Self {
        Self {
            guid: t.guid,
            list_id: t.list_id,
            title: t.title,
            authors: t.authors,
            series: t
                .series
                .into_iter()
                .map(|(name, num)| (name, num as f64))
                .collect(),
            cover_url: t.cover_url,
            book_url: t.book_url,
            isbn: t.isbn,
            prefer_format: t.prefer_format,
            allow_audio: t.allow_audio,
            audio_torrent: t.audio_torrent,
            allow_ebook: t.allow_ebook,
            ebook_torrent: t.ebook_torrent,
            created_at: t.created_at,
            marked_done_at: None,
        }
    }
}

impl From<v6::Torrent> for Torrent {
    fn from(t: v6::Torrent) -> Self {
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
