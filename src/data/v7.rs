use super::{v3, v4, v5, v6, v8};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 7, from = v6::Torrent)]
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
    pub meta: v6::TorrentMeta,
    #[secondary_key]
    pub created_at: v3::Timestamp,
    pub replaced_with: Option<(String, v3::Timestamp)>,
    pub request_matadata_update: bool,
    pub library_mismatch: Option<v5::LibraryMismatch>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 7, from = v6::SelectedTorrent)]
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
    pub meta: v6::TorrentMeta,
    pub created_at: v3::Timestamp,
    pub removed_at: Option<v3::Timestamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 4, version = 7, from = v6::DuplicateTorrent)]
#[native_db]
pub struct DuplicateTorrent {
    #[primary_key]
    pub mam_id: u64,
    pub dl_link: Option<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: v6::TorrentMeta,
    pub created_at: v3::Timestamp,
    pub duplicate_of: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 6, version = 7, from = v4::Event)]
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
pub enum EventType {
    Grabbed {
        cost: Option<v4::TorrentCost>,
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

impl From<v6::Torrent> for Torrent {
    fn from(t: v6::Torrent) -> Self {
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
            meta: t.meta,
            created_at: t.created_at,
            removed_at: None,
        }
    }
}

impl From<v6::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v6::DuplicateTorrent) -> Self {
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

impl From<v4::Event> for Event {
    fn from(t: v4::Event) -> Self {
        Self {
            id: t.id,
            hash: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v4::EventType> for EventType {
    fn from(t: v4::EventType) -> Self {
        match t {
            v4::EventType::Grabbed { cost, wedged } => Self::Grabbed { cost, wedged },
            v4::EventType::Linked { library_path } => Self::Linked { library_path },
            v4::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
        }
    }
}

impl From<v8::Torrent> for Torrent {
    fn from(t: v8::Torrent) -> Self {
        let library_mismatch = match (t.library_mismatch, t.client_status) {
            (Some(v8::LibraryMismatch::NewLibraryDir(path_buf)), _) => {
                Some(v5::LibraryMismatch::NewPath(path_buf))
            }
            (Some(v8::LibraryMismatch::NoLibrary), _) => Some(v5::LibraryMismatch::NoLibrary),
            (_, Some(v8::ClientStatus::NotInClient)) => Some(v5::LibraryMismatch::TorrentRemoved),
            (None, _) => None,
            (Some(v8::LibraryMismatch::NewPath(_)), _) => unimplemented!(),
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

impl From<v8::SelectedTorrent> for SelectedTorrent {
    fn from(t: v8::SelectedTorrent) -> Self {
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

impl From<v8::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v8::DuplicateTorrent) -> Self {
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

impl From<v8::Event> for Event {
    fn from(t: v8::Event) -> Self {
        Self {
            id: t.id,
            hash: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v8::EventType> for EventType {
    fn from(t: v8::EventType) -> Self {
        match t {
            v8::EventType::Grabbed { cost, wedged } => Self::Grabbed { cost, wedged },
            v8::EventType::Linked { library_path } => Self::Linked { library_path },
            v8::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
            v8::EventType::Updated { fields } => Self::Updated {
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
            v8::EventType::RemovedFromMam => unimplemented!(),
        }
    }
}

impl TryFrom<v8::TorrentMetaField> for TorrentMetaField {
    type Error = ();

    fn try_from(value: v8::TorrentMetaField) -> Result<Self, Self::Error> {
        match value {
            v8::TorrentMetaField::MamId => Ok(TorrentMetaField::MamId),
            v8::TorrentMetaField::MainCat => Ok(TorrentMetaField::MainCat),
            v8::TorrentMetaField::Cat => Ok(TorrentMetaField::Cat),
            v8::TorrentMetaField::Language => Ok(TorrentMetaField::Language),
            v8::TorrentMetaField::Flags => Err(()),
            v8::TorrentMetaField::Filetypes => Ok(TorrentMetaField::Filetypes),
            v8::TorrentMetaField::Size => Ok(TorrentMetaField::Size),
            v8::TorrentMetaField::Title => Ok(TorrentMetaField::Title),
            v8::TorrentMetaField::Authors => Ok(TorrentMetaField::Authors),
            v8::TorrentMetaField::Narrators => Ok(TorrentMetaField::Narrators),
            v8::TorrentMetaField::Series => Ok(TorrentMetaField::Series),
        }
    }
}
