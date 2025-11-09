use super::{v01, v03, v04, v06, v08, v09, v10, v11, v13};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 12, from = v11::Torrent)]
#[native_db]
pub struct Torrent {
    #[primary_key]
    pub hash: String,
    #[secondary_key(unique)]
    pub mam_id: u64,
    pub abs_id: Option<String>,
    pub goodreads_id: Option<u64>,
    pub library_path: Option<PathBuf>,
    pub library_files: Vec<PathBuf>,
    pub linker: Option<String>,
    pub category: Option<String>,
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
#[native_model(id = 3, version = 12, from = v11::SelectedTorrent)]
#[native_db]
pub struct SelectedTorrent {
    #[primary_key]
    pub mam_id: u64,
    pub goodreads_id: Option<u64>,
    #[secondary_key(unique, optional)]
    pub hash: Option<String>,
    pub dl_link: String,
    pub unsat_buffer: Option<u64>,
    pub cost: v04::TorrentCost,
    pub category: Option<String>,
    pub tags: Vec<String>,
    #[secondary_key]
    pub title_search: String,
    pub meta: TorrentMeta,
    pub grabber: Option<String>,
    pub created_at: v03::Timestamp,
    pub started_at: Option<v03::Timestamp>,
    pub removed_at: Option<v03::Timestamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 4, version = 12, from = v11::DuplicateTorrent)]
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
#[native_model(id = 5, version = 12, from = v11::ErroredTorrent)]
#[native_db]
pub struct ErroredTorrent {
    #[primary_key]
    pub id: v11::ErroredTorrentId,
    pub title: String,
    pub error: String,
    pub meta: Option<TorrentMeta>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TorrentMeta {
    pub mam_id: u64,
    pub vip_status: Option<v11::VipStatus>,
    pub cat: Option<v06::Category>,
    pub media_type: MediaType,
    pub main_cat: Option<MainCat>,
    pub categories: Vec<Category>,
    pub language: Option<v03::Language>,
    pub flags: Option<v08::FlagBits>,
    pub filetypes: Vec<String>,
    pub size: v03::Size,
    pub title: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<v09::Series>,
    pub source: v10::MetadataSource,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MediaType {
    Audiobook,
    Ebook,
    Musicology,
    Radio,
    Manga,
    #[doc(alias = "GraphicNovel")]
    ComicBook,
    Periodical,
    // PeriodicalEbook,
    // PeriodicalAudiobook
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainCat {
    Fiction,
    Nonfiction,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Action,
    Art,
    Biographical,
    Business,
    Comedy,
    CompleteEditionsMusic,
    Computer,
    Crafts,
    Crime,
    Drama,
    Education,
    FactualNews,
    Fantasy,
    Food,
    Guitar,
    Health,
    Historical,
    Home,
    Horror,
    Humor,
    IndividualSheet,
    Instructional,
    Juvenile,
    Language,
    Lgbt,
    LickLibraryLTP,
    LickLibraryTechniques,
    LiteraryClassics,
    LitRPG,
    Math,
    Medicine,
    Music,
    MusicBook,
    Mystery,
    Nature,
    Paranormal,
    Philosophy,
    Poetry,
    Politics,
    Reference,
    Religion,
    Romance,
    Rpg,
    Science,
    ScienceFiction,
    SelfHelp,
    SheetCollection,
    SheetCollectionMP3,
    Sports,
    Technology,
    Thriller,
    Travel,
    UrbanFantasy,
    Western,
    YoungAdult,
    Superheroes,
    LiteraryFiction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 6, version = 12, from = v11::Event)]
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
        grabber: Option<String>,
        cost: Option<v04::TorrentCost>,
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
    },
    RemovedFromMam,
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
    Vip,
    Cat,
    MediaType,
    MainCat,
    Categories,
    Language,
    Flags,
    Filetypes,
    Size,
    Title,
    Authors,
    Narrators,
    Series,
    Source,
}

impl From<v11::Torrent> for Torrent {
    fn from(t: v11::Torrent) -> Self {
        Self {
            hash: t.hash,
            mam_id: t.meta.mam_id,
            abs_id: t.abs_id,
            goodreads_id: None,
            library_path: t.library_path,
            library_files: t.library_files,
            linker: t.linker,
            category: t.category,
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

impl From<v11::SelectedTorrent> for SelectedTorrent {
    fn from(t: v11::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            goodreads_id: None,
            hash: t.hash,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            cost: t.cost,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta.into(),
            grabber: t.grabber,
            created_at: t.created_at,
            started_at: t.started_at,
            removed_at: t.removed_at,
        }
    }
}

impl From<v11::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v11::DuplicateTorrent) -> Self {
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

impl From<v11::ErroredTorrent> for ErroredTorrent {
    fn from(t: v11::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v11::TorrentMeta> for TorrentMeta {
    fn from(t: v11::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            vip_status: t.vip_status,
            cat: t.cat,
            media_type: t.main_cat.into(),
            main_cat: None,
            categories: vec![],
            language: t.language,
            flags: t.flags,
            filetypes: t.filetypes,
            size: t.size,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
            source: t.source,
        }
    }
}

impl From<v01::MainCat> for MediaType {
    fn from(value: v01::MainCat) -> Self {
        match value {
            v01::MainCat::Audio => MediaType::Audiobook,
            v01::MainCat::Ebook => MediaType::Ebook,
        }
    }
}

impl From<v11::Event> for Event {
    fn from(t: v11::Event) -> Self {
        Self {
            id: t.id,
            hash: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v11::EventType> for EventType {
    fn from(t: v11::EventType) -> Self {
        match t {
            v11::EventType::Grabbed {
                grabber,
                cost,
                wedged,
            } => Self::Grabbed {
                grabber,
                cost,
                wedged,
            },
            v11::EventType::Linked {
                linker,
                library_path,
            } => Self::Linked {
                linker,
                library_path,
            },
            v11::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
            v11::EventType::Updated { fields } => Self::Updated {
                fields: fields.into_iter().map(Into::into).collect(),
            },
            v11::EventType::RemovedFromMam => Self::RemovedFromMam,
        }
    }
}

impl From<v11::TorrentMetaDiff> for TorrentMetaDiff {
    fn from(value: v11::TorrentMetaDiff) -> Self {
        Self {
            field: value.field.into(),
            from: value.from,
            to: value.to,
        }
    }
}

impl From<v11::TorrentMetaField> for TorrentMetaField {
    fn from(value: v11::TorrentMetaField) -> Self {
        match value {
            v11::TorrentMetaField::MamId => TorrentMetaField::MamId,
            v11::TorrentMetaField::Vip => TorrentMetaField::Vip,
            v11::TorrentMetaField::MainCat => TorrentMetaField::MediaType,
            v11::TorrentMetaField::Cat => TorrentMetaField::Cat,
            v11::TorrentMetaField::Language => TorrentMetaField::Language,
            v11::TorrentMetaField::Flags => TorrentMetaField::Flags,
            v11::TorrentMetaField::Filetypes => TorrentMetaField::Filetypes,
            v11::TorrentMetaField::Size => TorrentMetaField::Size,
            v11::TorrentMetaField::Title => TorrentMetaField::Title,
            v11::TorrentMetaField::Authors => TorrentMetaField::Authors,
            v11::TorrentMetaField::Narrators => TorrentMetaField::Narrators,
            v11::TorrentMetaField::Series => TorrentMetaField::Series,
            v11::TorrentMetaField::Source => TorrentMetaField::Source,
        }
    }
}

impl From<v13::Torrent> for Torrent {
    fn from(t: v13::Torrent) -> Self {
        Self {
            hash: t.hash,
            mam_id: t.meta.mam_id,
            abs_id: t.abs_id,
            goodreads_id: t.goodreads_id,
            library_path: t.library_path,
            library_files: t.library_files,
            linker: t.linker,
            category: t.category,
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

impl From<v13::SelectedTorrent> for SelectedTorrent {
    fn from(t: v13::SelectedTorrent) -> Self {
        Self {
            mam_id: t.mam_id,
            goodreads_id: t.goodreads_id,
            hash: t.hash,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            cost: t.cost,
            category: t.category,
            tags: t.tags,
            title_search: t.title_search,
            meta: t.meta.into(),
            grabber: t.grabber,
            created_at: t.created_at,
            started_at: t.started_at,
            removed_at: t.removed_at,
        }
    }
}

impl From<v13::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v13::DuplicateTorrent) -> Self {
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

impl From<v13::ErroredTorrent> for ErroredTorrent {
    fn from(t: v13::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v13::TorrentMeta> for TorrentMeta {
    fn from(t: v13::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            vip_status: t.vip_status,
            cat: t.cat,
            media_type: t.media_type.into(),
            main_cat: t.main_cat,
            categories: t.categories,
            language: t.language,
            flags: t.flags,
            filetypes: t.filetypes,
            size: t.size,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
            source: t.source,
        }
    }
}

impl From<v13::MediaType> for MediaType {
    fn from(value: v13::MediaType) -> Self {
        match value {
            v13::MediaType::Audiobook => MediaType::Audiobook,
            v13::MediaType::Ebook => MediaType::Ebook,
            v13::MediaType::Musicology => MediaType::Musicology,
            v13::MediaType::Radio => MediaType::Radio,
            v13::MediaType::Manga => MediaType::Manga,
            v13::MediaType::ComicBook => MediaType::ComicBook,
            v13::MediaType::PeriodicalEbook => MediaType::Periodical,
            v13::MediaType::PeriodicalAudiobook => MediaType::Periodical,
        }
    }
}
