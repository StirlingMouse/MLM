use super::{v1, v3, v5, v6, v7};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 4, from = v3::SelectedTorrent)]
#[native_db]
pub struct SelectedTorrent {
    #[primary_key]
    pub mam_id: u64,
    pub dl_link: String,
    pub unsat_buffer: Option<u64>,
    pub cost: TorrentCost,
    pub category: Option<String>,
    pub tags: Vec<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: v3::TorrentMeta,
    pub created_at: v3::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 6, version = 4, from = v3::Event)]
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
#[native_model(id = 7, version = 4, from = v3::List)]
#[native_db]
pub struct List {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 8, version = 4, from = v3::ListItem)]
#[native_db]
pub struct ListItem {
    #[primary_key]
    pub guid: (String, String),
    #[secondary_key]
    pub list_id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub series: Vec<(String, u64)>,
    pub cover_url: String,
    pub book_url: Option<String>,
    pub isbn: Option<u64>,
    pub prefer_format: Option<v1::MainCat>,
    pub allow_audio: bool,
    pub audio_torrent: Option<ListItemTorrent>,
    pub allow_ebook: bool,
    pub ebook_torrent: Option<ListItemTorrent>,
    #[secondary_key]
    pub created_at: v3::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EventType {
    Grabbed {
        cost: Option<TorrentCost>,
        wedged: bool,
    },
    Linked {
        library_path: PathBuf,
    },
    Cleaned {
        library_path: PathBuf,
        files: Vec<PathBuf>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TorrentCost {
    GlobalFreeleech,
    PersonalFreeleech,
    Vip,
    UseWedge,
    TryWedge,
    Ratio,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListItemTorrent {
    pub mam_id: u64,
    pub status: TorrentStatus,
    pub at: v3::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum TorrentStatus {
    Selected,
    Wanted,
    NotWanted,
    Existing,
}

impl From<v3::SelectedTorrent> for SelectedTorrent {
    fn from(t: v3::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            cost: TorrentCost::Ratio,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta,
            created_at: t.created_at,
        }
    }
}

impl From<v3::Event> for Event {
    fn from(t: v3::Event) -> Self {
        Self {
            id: t.id,
            hash: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v3::List> for List {
    fn from(t: v3::List) -> Self {
        Self {
            id: format!("{}:to-read", t.id),
            title: t.title,
        }
    }
}

impl From<v3::ListItem> for ListItem {
    fn from(t: v3::ListItem) -> Self {
        Self {
            guid: (format!("{}:to-read", t.list_id), t.guid.1),
            list_id: format!("{}:to-read", t.list_id),
            title: t.title,
            authors: t.authors,
            series: t.series,
            cover_url: t.cover_url,
            book_url: t.book_url,
            isbn: t.isbn,
            prefer_format: t.prefer_format,
            allow_audio: true,
            audio_torrent: t
                .audio_torrent
                .map(|t| ListItemTorrent {
                    mam_id: t.0,
                    status: TorrentStatus::Selected,
                    at: t.1,
                })
                .or_else(|| {
                    t.wanted_audio_torrent.map(|t| ListItemTorrent {
                        mam_id: t.0,
                        status: TorrentStatus::Wanted,
                        at: t.1,
                    })
                }),
            allow_ebook: true,
            ebook_torrent: t
                .ebook_torrent
                .map(|t| ListItemTorrent {
                    mam_id: t.0,
                    status: TorrentStatus::Selected,
                    at: t.1,
                })
                .or_else(|| {
                    t.wanted_ebook_torrent.map(|t| ListItemTorrent {
                        mam_id: t.0,
                        status: TorrentStatus::Wanted,
                        at: t.1,
                    })
                }),
            created_at: t.created_at,
        }
    }
}

impl From<v3::EventType> for EventType {
    fn from(t: v3::EventType) -> Self {
        match t {
            v3::EventType::Grabbed => Self::Grabbed {
                cost: None,
                wedged: false,
            },
            v3::EventType::Linked { library_path } => Self::Linked { library_path },
            v3::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
        }
    }
}

impl From<v5::List> for List {
    fn from(t: v5::List) -> Self {
        Self {
            id: t.id,
            title: t.title,
        }
    }
}

impl From<v5::ListItem> for ListItem {
    fn from(t: v5::ListItem) -> Self {
        Self {
            guid: t.guid,
            list_id: t.list_id,
            title: t.title,
            authors: t.authors,
            series: t
                .series
                .into_iter()
                .map(|(name, num)| (name, num as u64))
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
        }
    }
}

impl From<v6::SelectedTorrent> for SelectedTorrent {
    fn from(t: v6::SelectedTorrent) -> Self {
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
            v7::EventType::Updated { .. } => unimplemented!(),
        }
    }
}
