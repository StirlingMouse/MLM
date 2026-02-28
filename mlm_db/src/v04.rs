use super::{v01, v03, v05, v06, v07, v18};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 4, from = v03::SelectedTorrent)]
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
    pub meta: v03::TorrentMeta,
    pub created_at: v03::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 6, version = 4, from = v03::Event)]
#[native_db]
pub struct Event {
    #[primary_key]
    pub id: v03::Uuid,
    #[secondary_key]
    pub hash: Option<String>,
    #[secondary_key]
    pub mam_id: Option<u64>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
    pub event: EventType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 7, version = 4, from = v03::List)]
#[native_db]
pub struct List {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 8, version = 4, from = v03::ListItem)]
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
    pub prefer_format: Option<v01::MainCat>,
    pub allow_audio: bool,
    pub audio_torrent: Option<ListItemTorrent>,
    pub allow_ebook: bool,
    pub ebook_torrent: Option<ListItemTorrent>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
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
    pub at: v03::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum TorrentStatus {
    Selected,
    Wanted,
    NotWanted,
    Existing,
}

impl From<v03::SelectedTorrent> for SelectedTorrent {
    fn from(t: v03::SelectedTorrent) -> Self {
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

impl From<v03::Event> for Event {
    fn from(t: v03::Event) -> Self {
        Self {
            id: t.id,
            hash: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v03::List> for List {
    fn from(t: v03::List) -> Self {
        Self {
            id: format!("{}:to-read", t.id),
            title: t.title,
        }
    }
}

impl From<v03::ListItem> for ListItem {
    fn from(t: v03::ListItem) -> Self {
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

impl From<v03::EventType> for EventType {
    fn from(t: v03::EventType) -> Self {
        match t {
            v03::EventType::Grabbed => Self::Grabbed {
                cost: None,
                wedged: false,
            },
            v03::EventType::Linked { library_path } => Self::Linked { library_path },
            v03::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
        }
    }
}

impl From<v05::List> for List {
    fn from(t: v05::List) -> Self {
        Self {
            id: t.id,
            title: t.title,
        }
    }
}

impl From<v05::ListItem> for ListItem {
    fn from(t: v05::ListItem) -> Self {
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

impl From<v06::SelectedTorrent> for SelectedTorrent {
    fn from(t: v06::SelectedTorrent) -> Self {
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

impl From<v07::Event> for Event {
    fn from(t: v07::Event) -> Self {
        Self {
            id: t.id,
            hash: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v07::EventType> for EventType {
    fn from(t: v07::EventType) -> Self {
        match t {
            v07::EventType::Grabbed { cost, wedged } => Self::Grabbed { cost, wedged },
            v07::EventType::Linked { library_path } => Self::Linked { library_path },
            v07::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
            v07::EventType::Updated { .. } => unimplemented!(),
        }
    }
}

impl From<v18::ListItemTorrent> for ListItemTorrent {
    fn from(t: v18::ListItemTorrent) -> Self {
        Self {
            mam_id: t.mam_id.unwrap(),
            status: t.status,
            at: t.at,
        }
    }
}
