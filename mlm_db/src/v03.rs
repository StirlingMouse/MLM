use super::{v01, v02, v04, v05, v06};
use native_db::{native_db, Key, ToKey};
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::{OffsetDateTime, UtcDateTime};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 3, from = v02::Torrent)]
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
    pub created_at: Timestamp,
    pub replaced_with: Option<(String, Timestamp)>,
    pub request_matadata_update: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 3, from = v02::SelectedTorrent)]
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
    pub created_at: Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 4, version = 3, from = v02::DuplicateTorrent)]
#[native_db]
pub struct DuplicateTorrent {
    #[primary_key]
    pub mam_id: u64,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    pub created_at: Timestamp,
    pub duplicate_of: Option<String>,
    pub request_replace: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 5, version = 3, from = v02::ErroredTorrent)]
#[native_db]
pub struct ErroredTorrent {
    #[primary_key]
    pub id: v01::ErroredTorrentId,
    pub title: String,
    pub error: String,
    pub meta: Option<TorrentMeta>,
    #[secondary_key]
    pub created_at: Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 6, version = 3)]
#[native_db]
pub struct Event {
    #[primary_key]
    pub id: Uuid,
    #[secondary_key]
    pub hash: Option<String>,
    #[secondary_key]
    pub mam_id: Option<u64>,
    #[secondary_key]
    pub created_at: Timestamp,
    pub event: EventType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 7, version = 3)]
#[native_db]
pub struct List {
    #[primary_key]
    pub id: u64,
    #[secondary_key]
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 8, version = 3)]
#[native_db]
pub struct ListItem {
    #[primary_key]
    pub guid: (u64, String),
    #[secondary_key]
    pub list_id: u64,
    pub title: String,
    pub authors: Vec<String>,
    pub series: Vec<(String, u64)>,
    pub cover_url: String,
    pub book_url: Option<String>,
    pub isbn: Option<u64>,
    pub prefer_format: Option<v01::MainCat>,
    pub audio_torrent: Option<(u64, Timestamp)>,
    pub wanted_audio_torrent: Option<(u64, Timestamp)>,
    pub ebook_torrent: Option<(u64, Timestamp)>,
    pub wanted_ebook_torrent: Option<(u64, Timestamp)>,
    #[secondary_key]
    pub created_at: Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EventType {
    Grabbed,
    Linked {
        library_path: PathBuf,
    },
    Cleaned {
        library_path: PathBuf,
        files: Vec<PathBuf>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TorrentMeta {
    pub mam_id: u64,
    pub main_cat: v01::MainCat,
    pub language: Option<Language>,
    pub filetypes: Vec<String>,
    pub size: Size,
    pub title: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language {
    #[default]
    English,
    Afrikaans,
    Arabic,
    Bengali,
    Bosnian,
    Bulgarian,
    Burmese,
    Cantonese,
    Catalan,
    Chinese,
    Croatian,
    Czech,
    Danish,
    Dutch,
    Estonian,
    Farsi,
    Finnish,
    French,
    German,
    Greek,
    GreekAncient,
    Gujarati,
    Hebrew,
    Hindi,
    Hungarian,
    Icelandic,
    Indonesian,
    Irish,
    Italian,
    Japanese,
    Javanese,
    Kannada,
    Korean,
    Lithuanian,
    Latin,
    Latvian,
    Malay,
    Malayalam,
    Manx,
    Marathi,
    Norwegian,
    Polish,
    Portuguese,
    BrazilianPortuguese,
    Punjabi,
    Romanian,
    Russian,
    ScottishGaelic,
    Sanskrit,
    Serbian,
    Slovenian,
    Spanish,
    CastilianSpanish,
    Swedish,
    Tagalog,
    Tamil,
    Telugu,
    Thai,
    Turkish,
    Ukrainian,
    Urdu,
    Vietnamese,
    Other,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Size(u64);
impl Size {
    pub fn from_bytes(bytes: u64) -> Size {
        Size(bytes)
    }

    pub fn bytes(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Timestamp(pub UtcDateTime);
impl Timestamp {
    pub fn now() -> Self {
        Self(UtcDateTime::now())
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

impl From<UtcDateTime> for Timestamp {
    fn from(value: UtcDateTime) -> Self {
        Self(value)
    }
}
impl From<OffsetDateTime> for Timestamp {
    fn from(value: OffsetDateTime) -> Self {
        Self(value.to_utc())
    }
}

impl ToKey for Timestamp {
    fn to_key(&self) -> Key {
        Key::new(self.0.unix_timestamp().to_be_bytes().into())
    }

    fn key_names() -> Vec<String> {
        vec!["Timestamp".to_string()]
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone, Hash)]
pub struct Uuid(uuid::Uuid);
impl Uuid {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for Uuid {
    fn default() -> Self {
        Self::new()
    }
}

impl ToKey for Uuid {
    fn to_key(&self) -> Key {
        Key::new(self.0.as_bytes().to_vec())
    }

    fn key_names() -> Vec<String> {
        vec!["Uuid".to_string()]
    }
}

impl From<v02::Torrent> for Torrent {
    fn from(t: v02::Torrent) -> Self {
        Self {
            hash: t.hash,
            library_path: t.library_path,
            library_files: t.library_files,
            selected_audio_format: None,
            selected_ebook_format: None,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at.into(),
            replaced_with: t.replaced_with.map(|(with, when)| (with, when.into())),
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
            meta: t.meta.into(),
            created_at: t.created_at.into(),
        }
    }
}

impl From<v02::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v02::DuplicateTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            title_search: t.title_search,
            meta: t.meta.into(),
            duplicate_of: t.duplicate_of,
            request_replace: t.request_replace,
            created_at: t.created_at.into(),
        }
    }
}

impl From<v02::ErroredTorrent> for ErroredTorrent {
    fn from(t: v02::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(Into::into),
            created_at: t.created_at.into(),
        }
    }
}

impl From<v01::TorrentMeta> for TorrentMeta {
    fn from(t: v01::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            main_cat: t.main_cat,
            language: None,
            filetypes: t.filetypes,
            size: Size(0),
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
        }
    }
}

impl From<v05::Torrent> for Torrent {
    fn from(t: v05::Torrent) -> Self {
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
        }
    }
}

impl From<v04::SelectedTorrent> for SelectedTorrent {
    fn from(t: v04::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta,
            created_at: t.created_at,
        }
    }
}

impl From<v04::Event> for Event {
    fn from(t: v04::Event) -> Self {
        Self {
            id: t.id,
            hash: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v04::List> for List {
    fn from(t: v04::List) -> Self {
        Self {
            id: t.id.split(':').next().unwrap().parse().unwrap(),
            title: t.title,
        }
    }
}

impl From<v04::ListItem> for ListItem {
    fn from(t: v04::ListItem) -> Self {
        let list_id = t.list_id.split(':').next().unwrap().parse().unwrap();

        Self {
            guid: (list_id, t.guid.1),
            list_id,
            title: t.title,
            authors: t.authors,
            series: t.series,
            cover_url: t.cover_url,
            book_url: t.book_url,
            isbn: t.isbn,
            prefer_format: t.prefer_format,
            audio_torrent: t.audio_torrent.as_ref().and_then(|t| {
                if t.status == v04::TorrentStatus::Selected {
                    Some((t.mam_id, t.at))
                } else {
                    None
                }
            }),
            wanted_audio_torrent: t.audio_torrent.as_ref().and_then(|t| {
                if t.status == v04::TorrentStatus::Wanted {
                    Some((t.mam_id, t.at))
                } else {
                    None
                }
            }),
            ebook_torrent: t.ebook_torrent.as_ref().and_then(|t| {
                if t.status == v04::TorrentStatus::Selected {
                    Some((t.mam_id, t.at))
                } else {
                    None
                }
            }),
            wanted_ebook_torrent: t.ebook_torrent.as_ref().and_then(|t| {
                if t.status == v04::TorrentStatus::Wanted {
                    Some((t.mam_id, t.at))
                } else {
                    None
                }
            }),
            created_at: t.created_at,
        }
    }
}

impl From<v04::EventType> for EventType {
    fn from(t: v04::EventType) -> Self {
        match t {
            v04::EventType::Grabbed { .. } => Self::Grabbed,
            v04::EventType::Linked { library_path } => Self::Linked { library_path },
            v04::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
        }
    }
}

impl From<v06::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v06::DuplicateTorrent) -> Self {
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

impl From<v06::ErroredTorrent> for ErroredTorrent {
    fn from(t: v06::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v06::TorrentMeta> for TorrentMeta {
    fn from(t: v06::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            main_cat: t.main_cat,
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
