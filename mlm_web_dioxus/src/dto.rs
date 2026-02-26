use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Series {
    pub name: String,
    pub entries: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentMetaDiff {
    pub field: String,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TorrentCost {
    GlobalFreeleech,
    PersonalFreeleech,
    Vip,
    UseWedge,
    TryWedge,
    Ratio,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MetadataSource {
    Mam,
    Manual,
    File,
    Match,
}

impl std::fmt::Display for MetadataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataSource::Mam => write!(f, "MaM"),
            MetadataSource::Manual => write!(f, "Manual"),
            MetadataSource::File => write!(f, "File"),
            MetadataSource::Match => write!(f, "Match"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    Grabbed {
        grabber: Option<String>,
        cost: Option<TorrentCost>,
        wedged: bool,
    },
    Linked {
        linker: Option<String>,
        library_path: PathBuf,
    },
    Cleaned {
        library_path: PathBuf,
        files: Vec<PathBuf>,
    },
    Updated {
        fields: Vec<TorrentMetaDiff>,
        source: (MetadataSource, String),
    },
    RemovedFromTracker,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: String,
    pub created_at: String,
    pub event: EventType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentMeta {
    pub title: String,
    pub media_type: String,
    pub size: u64,
    pub filetypes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Torrent {
    pub id: String,
    pub meta: TorrentMeta,
    pub library_path: Option<PathBuf>,
    pub library_files: Vec<PathBuf>,
    pub linker: Option<String>,
    pub category: Option<String>,
}

#[cfg(feature = "server")]
impl From<&mlm_core::TorrentCost> for TorrentCost {
    fn from(cost: &mlm_core::TorrentCost) -> Self {
        match cost {
            mlm_core::TorrentCost::Vip => TorrentCost::Vip,
            mlm_core::TorrentCost::GlobalFreeleech => TorrentCost::GlobalFreeleech,
            mlm_core::TorrentCost::PersonalFreeleech => TorrentCost::PersonalFreeleech,
            mlm_core::TorrentCost::UseWedge => TorrentCost::UseWedge,
            mlm_core::TorrentCost::TryWedge => TorrentCost::TryWedge,
            mlm_core::TorrentCost::Ratio => TorrentCost::Ratio,
        }
    }
}

#[cfg(feature = "server")]
impl From<&mlm_core::MetadataSource> for MetadataSource {
    fn from(source: &mlm_core::MetadataSource) -> Self {
        match source {
            mlm_core::MetadataSource::Mam => MetadataSource::Mam,
            mlm_core::MetadataSource::Manual => MetadataSource::Manual,
            mlm_core::MetadataSource::File => MetadataSource::File,
            mlm_core::MetadataSource::Match => MetadataSource::Match,
        }
    }
}

#[cfg(feature = "server")]
pub fn convert_torrent(db_torrent: &mlm_core::Torrent) -> Torrent {
    Torrent {
        id: db_torrent.id.clone(),
        meta: TorrentMeta {
            title: db_torrent.meta.title.clone(),
            media_type: db_torrent.meta.media_type.as_str().to_string(),
            size: db_torrent.meta.size.bytes(),
            filetypes: db_torrent.meta.filetypes.clone(),
        },
        library_path: db_torrent.library_path.clone(),
        library_files: db_torrent.library_files.clone(),
        linker: db_torrent.linker.clone(),
        category: db_torrent.category.clone(),
    }
}

/// Convert a `mlm_core::EventType` to the DTO `EventType`. Used by both the
/// events page and the torrent-detail page so it lives here rather than in
/// either module.
#[cfg(feature = "server")]
pub fn convert_event_type(event: &mlm_core::EventType) -> EventType {
    match event {
        mlm_core::EventType::Grabbed {
            grabber,
            cost,
            wedged,
        } => EventType::Grabbed {
            grabber: grabber.clone(),
            cost: cost.as_ref().map(|c| c.into()),
            wedged: *wedged,
        },
        mlm_core::EventType::Linked {
            linker,
            library_path,
        } => EventType::Linked {
            linker: linker.clone(),
            library_path: library_path.clone(),
        },
        mlm_core::EventType::Cleaned {
            library_path,
            files,
        } => EventType::Cleaned {
            library_path: library_path.clone(),
            files: files.clone(),
        },
        mlm_core::EventType::Updated { fields, source } => EventType::Updated {
            fields: fields
                .iter()
                .map(|f| TorrentMetaDiff {
                    field: f.field.to_string(),
                    from: f.from.clone(),
                    to: f.to.clone(),
                })
                .collect(),
            source: (MetadataSource::from(&source.0), source.1.clone()),
        },
        mlm_core::EventType::RemovedFromTracker => EventType::RemovedFromTracker,
    }
}
