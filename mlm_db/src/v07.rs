use super::{v03, v04, v05, v06, v08};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 7, from = v06::Torrent)]
#[native_db]
pub struct Torrent {
    #[primary_key]
    pub hash: String,
    pub abs_id: Option<String>,
    pub library_path: Option<PathBuf>,
    pub library_files: Vec<PathBuf>,
    pub selected_audio_format: Option<String>,
    pub selected_ebook_format: Option<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: v06::TorrentMeta,
    #[secondary_key]
    pub created_at: v03::Timestamp,
    pub replaced_with: Option<(String, v03::Timestamp)>,
    pub request_matadata_update: bool,
    pub library_mismatch: Option<v05::LibraryMismatch>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 7, from = v06::SelectedTorrent)]
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
    pub meta: v06::TorrentMeta,
    pub created_at: v03::Timestamp,
    pub removed_at: Option<v03::Timestamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 4, version = 7, from = v06::DuplicateTorrent)]
#[native_db]
pub struct DuplicateTorrent {
    #[primary_key]
    pub mam_id: u64,
    pub dl_link: Option<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: v06::TorrentMeta,
    pub created_at: v03::Timestamp,
    pub duplicate_of: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 6, version = 7, from = v04::Event)]
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
pub enum EventType {
    Grabbed {
        cost: Option<v04::TorrentCost>,
        wedged: bool,
    },
    Linked {
        library_path: PathBuf,
    },
    Cleaned {
        library_path: PathBuf,
        files: Vec<PathBuf>,
    },
    Updated {
        fields: Vec<TorrentMetaDiff>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TorrentMetaDiff {
    pub field: TorrentMetaField,
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TorrentMetaField {
    MamId,
    MainCat,
    Cat,
    Language,
    Filetypes,
    Size,
    Title,
    Authors,
    Narrators,
    Series,
}

impl From<v06::Torrent> for Torrent {
    fn from(t: v06::Torrent) -> Self {
        Self {
            hash: t.hash,
            abs_id: None,
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
            meta: t.meta,
            created_at: t.created_at,
            removed_at: None,
        }
    }
}

impl From<v06::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v06::DuplicateTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            dl_link: None,
            title_search: t.title_search,
            meta: t.meta,
            created_at: t.created_at,
            duplicate_of: t.duplicate_of,
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

impl From<v04::EventType> for EventType {
    fn from(t: v04::EventType) -> Self {
        match t {
            v04::EventType::Grabbed { cost, wedged } => Self::Grabbed { cost, wedged },
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

impl From<v08::Torrent> for Torrent {
    fn from(t: v08::Torrent) -> Self {
        let library_mismatch = match (t.library_mismatch, t.client_status) {
            (Some(v08::LibraryMismatch::NewLibraryDir(path_buf)), _) => {
                Some(v05::LibraryMismatch::NewPath(path_buf))
            }
            (Some(v08::LibraryMismatch::NoLibrary), _) => Some(v05::LibraryMismatch::NoLibrary),
            (_, Some(v08::ClientStatus::NotInClient)) => Some(v05::LibraryMismatch::TorrentRemoved),
            (None, _) => None,
            (Some(v08::LibraryMismatch::NewPath(_)), _) => unimplemented!(),
        };

        Self {
            hash: t.hash,
            abs_id: None,
            library_path: t.library_path,
            library_files: t.library_files,
            selected_audio_format: t.selected_audio_format,
            selected_ebook_format: t.selected_ebook_format,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at,
            replaced_with: t.replaced_with,
            request_matadata_update: t.request_matadata_update,
            library_mismatch,
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
            removed_at: None,
        }
    }
}

impl From<v08::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v08::DuplicateTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            dl_link: None,
            title_search: t.title_search,
            meta: t.meta.into(),
            created_at: t.created_at,
            duplicate_of: t.duplicate_of,
        }
    }
}

impl From<v08::Event> for Event {
    fn from(t: v08::Event) -> Self {
        Self {
            id: t.id,
            hash: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v08::EventType> for EventType {
    fn from(t: v08::EventType) -> Self {
        match t {
            v08::EventType::Grabbed { cost, wedged } => Self::Grabbed { cost, wedged },
            v08::EventType::Linked { library_path } => Self::Linked { library_path },
            v08::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
            v08::EventType::Updated { fields } => Self::Updated {
                fields: fields
                    .into_iter()
                    .filter_map(|f| {
                        Some(TorrentMetaDiff {
                            field: f.field.try_into().ok()?,
                            from: f.from,
                            to: f.to,
                        })
                    })
                    .collect(),
            },
            v08::EventType::RemovedFromMam => unimplemented!(),
        }
    }
}

impl TryFrom<v08::TorrentMetaField> for TorrentMetaField {
    type Error = ();

    fn try_from(value: v08::TorrentMetaField) -> Result<Self, Self::Error> {
        match value {
            v08::TorrentMetaField::MamId => Ok(TorrentMetaField::MamId),
            v08::TorrentMetaField::MainCat => Ok(TorrentMetaField::MainCat),
            v08::TorrentMetaField::Cat => Ok(TorrentMetaField::Cat),
            v08::TorrentMetaField::Language => Ok(TorrentMetaField::Language),
            v08::TorrentMetaField::Flags => Err(()),
            v08::TorrentMetaField::Filetypes => Ok(TorrentMetaField::Filetypes),
            v08::TorrentMetaField::Size => Ok(TorrentMetaField::Size),
            v08::TorrentMetaField::Title => Ok(TorrentMetaField::Title),
            v08::TorrentMetaField::Authors => Ok(TorrentMetaField::Authors),
            v08::TorrentMetaField::Narrators => Ok(TorrentMetaField::Narrators),
            v08::TorrentMetaField::Series => Ok(TorrentMetaField::Series),
        }
    }
}
