use super::{v03, v04, v06, v08, v09, v10, v11, v12, v13, v14, v16};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::UtcDateTime;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 15, from = v14::Torrent)]
#[native_db]
pub struct Torrent {
    #[primary_key]
    pub id: String,
    pub id_is_hash: bool,
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
#[native_model(id = 3, version = 15, from = v14::SelectedTorrent)]
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
#[native_model(id = 4, version = 15, from = v14::DuplicateTorrent)]
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
#[native_model(id = 5, version = 15, from = v14::ErroredTorrent)]
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
    pub media_type: v13::MediaType,
    pub main_cat: Option<v12::MainCat>,
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
    pub uploaded_at: v03::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    Dramatization,
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
    ProgressionFantasy,
    ContemporaryFiction,
    DramaPlays,
    Unknown(u8),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 6, version = 15, from = v12::Event)]
#[native_db]
pub struct Event {
    #[primary_key]
    pub id: v03::Uuid,
    #[secondary_key]
    pub torrent_id: Option<String>,
    #[secondary_key]
    pub mam_id: Option<u64>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
    pub event: v12::EventType,
}

impl From<v14::Torrent> for Torrent {
    fn from(t: v14::Torrent) -> Self {
        let id_is_hash = Uuid::parse_str(&t.hash).is_err();
        Self {
            id: t.hash,
            id_is_hash,
            mam_id: t.mam_id,
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

impl From<v14::SelectedTorrent> for SelectedTorrent {
    fn from(t: v14::SelectedTorrent) -> Self {
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

impl From<v14::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v14::DuplicateTorrent) -> Self {
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

impl From<v14::ErroredTorrent> for ErroredTorrent {
    fn from(t: v14::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v14::TorrentMeta> for TorrentMeta {
    fn from(t: v14::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            vip_status: t.vip_status,
            cat: t.cat,
            media_type: t.media_type,
            main_cat: t.main_cat,
            categories: t.categories.into_iter().map(|c| c.into()).collect(),
            language: t.language,
            flags: t.flags,
            filetypes: t.filetypes,
            size: t.size,
            title: t.title,
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
            source: t.source,
            uploaded_at: v03::Timestamp(UtcDateTime::UNIX_EPOCH),
        }
    }
}

impl From<v14::Category> for Category {
    fn from(value: v14::Category) -> Self {
        match value {
            v14::Category::Action => Category::Action,
            v14::Category::Art => Category::Art,
            v14::Category::Biographical => Category::Biographical,
            v14::Category::Business => Category::Business,
            v14::Category::Comedy => Category::Comedy,
            v14::Category::CompleteEditionsMusic => Category::CompleteEditionsMusic,
            v14::Category::Computer => Category::Computer,
            v14::Category::Crafts => Category::Crafts,
            v14::Category::Crime => Category::Crime,
            v14::Category::Dramatization => Category::Dramatization,
            v14::Category::Education => Category::Education,
            v14::Category::FactualNews => Category::FactualNews,
            v14::Category::Fantasy => Category::Fantasy,
            v14::Category::Food => Category::Food,
            v14::Category::Guitar => Category::Guitar,
            v14::Category::Health => Category::Health,
            v14::Category::Historical => Category::Historical,
            v14::Category::Home => Category::Home,
            v14::Category::Horror => Category::Horror,
            v14::Category::Humor => Category::Humor,
            v14::Category::IndividualSheet => Category::IndividualSheet,
            v14::Category::Instructional => Category::Instructional,
            v14::Category::Juvenile => Category::Juvenile,
            v14::Category::Language => Category::Language,
            v14::Category::Lgbt => Category::Lgbt,
            v14::Category::LickLibraryLTP => Category::LickLibraryLTP,
            v14::Category::LickLibraryTechniques => Category::LickLibraryTechniques,
            v14::Category::LiteraryClassics => Category::LiteraryClassics,
            v14::Category::LitRPG => Category::LitRPG,
            v14::Category::Math => Category::Math,
            v14::Category::Medicine => Category::Medicine,
            v14::Category::Music => Category::Music,
            v14::Category::MusicBook => Category::MusicBook,
            v14::Category::Mystery => Category::Mystery,
            v14::Category::Nature => Category::Nature,
            v14::Category::Paranormal => Category::Paranormal,
            v14::Category::Philosophy => Category::Philosophy,
            v14::Category::Poetry => Category::Poetry,
            v14::Category::Politics => Category::Politics,
            v14::Category::Reference => Category::Reference,
            v14::Category::Religion => Category::Religion,
            v14::Category::Romance => Category::Romance,
            v14::Category::Rpg => Category::Rpg,
            v14::Category::Science => Category::Science,
            v14::Category::ScienceFiction => Category::ScienceFiction,
            v14::Category::SelfHelp => Category::SelfHelp,
            v14::Category::SheetCollection => Category::SheetCollection,
            v14::Category::SheetCollectionMP3 => Category::SheetCollectionMP3,
            v14::Category::Sports => Category::Sports,
            v14::Category::Technology => Category::Technology,
            v14::Category::Thriller => Category::Thriller,
            v14::Category::Travel => Category::Travel,
            v14::Category::UrbanFantasy => Category::UrbanFantasy,
            v14::Category::Western => Category::Western,
            v14::Category::YoungAdult => Category::YoungAdult,
            v14::Category::Superheroes => Category::Superheroes,
            v14::Category::LiteraryFiction => Category::LiteraryFiction,
            v14::Category::ProgressionFantasy => Category::ProgressionFantasy,
            v14::Category::Unknown(59) => Category::ContemporaryFiction,
            v14::Category::DramaPlays => Category::DramaPlays,
            v14::Category::Unknown(id) => Category::Unknown(id),
        }
    }
}

impl From<v12::Event> for Event {
    fn from(t: v12::Event) -> Self {
        Self {
            id: t.id,
            torrent_id: t.hash,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event,
        }
    }
}

impl From<v16::Torrent> for Torrent {
    fn from(t: v16::Torrent) -> Self {
        Self {
            id: t.id,
            id_is_hash: t.id_is_hash,
            mam_id: t.mam_id,
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

impl From<v16::SelectedTorrent> for SelectedTorrent {
    fn from(t: v16::SelectedTorrent) -> Self {
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

impl From<v16::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v16::DuplicateTorrent) -> Self {
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

impl From<v16::ErroredTorrent> for ErroredTorrent {
    fn from(t: v16::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v16::TorrentMeta> for TorrentMeta {
    fn from(t: v16::TorrentMeta) -> Self {
        Self {
            mam_id: t.mam_id,
            vip_status: t.vip_status,
            cat: t.cat.map(|c| c.into()),
            media_type: t.media_type,
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
            uploaded_at: t.uploaded_at,
        }
    }
}
