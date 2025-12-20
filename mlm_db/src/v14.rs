use super::{v03, v04, v06, v08, v09, v10, v11, v12, v13, v15};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 14, from = v13::Torrent)]
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
#[native_model(id = 3, version = 14, from = v13::SelectedTorrent)]
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
#[native_model(id = 4, version = 14, from = v13::DuplicateTorrent)]
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
#[native_model(id = 5, version = 14, from = v13::ErroredTorrent)]
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
    DramaPlays,
    Unknown(u8),
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
        }
    }
}

impl From<v12::Category> for Category {
    fn from(value: v12::Category) -> Self {
        match value {
            v12::Category::Action => Category::Action,
            v12::Category::Art => Category::Art,
            v12::Category::Biographical => Category::Biographical,
            v12::Category::Business => Category::Business,
            v12::Category::Comedy => Category::Comedy,
            v12::Category::CompleteEditionsMusic => Category::CompleteEditionsMusic,
            v12::Category::Computer => Category::Computer,
            v12::Category::Crafts => Category::Crafts,
            v12::Category::Crime => Category::Crime,
            v12::Category::Drama => Category::Dramatization,
            v12::Category::Education => Category::Education,
            v12::Category::FactualNews => Category::FactualNews,
            v12::Category::Fantasy => Category::Fantasy,
            v12::Category::Food => Category::Food,
            v12::Category::Guitar => Category::Guitar,
            v12::Category::Health => Category::Health,
            v12::Category::Historical => Category::Historical,
            v12::Category::Home => Category::Home,
            v12::Category::Horror => Category::Horror,
            v12::Category::Humor => Category::Humor,
            v12::Category::IndividualSheet => Category::IndividualSheet,
            v12::Category::Instructional => Category::Instructional,
            v12::Category::Juvenile => Category::Juvenile,
            v12::Category::Language => Category::Language,
            v12::Category::Lgbt => Category::Lgbt,
            v12::Category::LickLibraryLTP => Category::LickLibraryLTP,
            v12::Category::LickLibraryTechniques => Category::LickLibraryTechniques,
            v12::Category::LiteraryClassics => Category::LiteraryClassics,
            v12::Category::LitRPG => Category::LitRPG,
            v12::Category::Math => Category::Math,
            v12::Category::Medicine => Category::Medicine,
            v12::Category::Music => Category::Music,
            v12::Category::MusicBook => Category::MusicBook,
            v12::Category::Mystery => Category::Mystery,
            v12::Category::Nature => Category::Nature,
            v12::Category::Paranormal => Category::Paranormal,
            v12::Category::Philosophy => Category::Philosophy,
            v12::Category::Poetry => Category::Poetry,
            v12::Category::Politics => Category::Politics,
            v12::Category::Reference => Category::Reference,
            v12::Category::Religion => Category::Religion,
            v12::Category::Romance => Category::Romance,
            v12::Category::Rpg => Category::Rpg,
            v12::Category::Science => Category::Science,
            v12::Category::ScienceFiction => Category::ScienceFiction,
            v12::Category::SelfHelp => Category::SelfHelp,
            v12::Category::SheetCollection => Category::SheetCollection,
            v12::Category::SheetCollectionMP3 => Category::SheetCollectionMP3,
            v12::Category::Sports => Category::Sports,
            v12::Category::Technology => Category::Technology,
            v12::Category::Thriller => Category::Thriller,
            v12::Category::Travel => Category::Travel,
            v12::Category::UrbanFantasy => Category::UrbanFantasy,
            v12::Category::Western => Category::Western,
            v12::Category::YoungAdult => Category::YoungAdult,
            v12::Category::Superheroes => Category::Superheroes,
            v12::Category::LiteraryFiction => Category::LiteraryFiction,
        }
    }
}

impl From<v15::Torrent> for Torrent {
    fn from(t: v15::Torrent) -> Self {
        Self {
            hash: t.id,
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

impl From<v15::SelectedTorrent> for SelectedTorrent {
    fn from(t: v15::SelectedTorrent) -> Self {
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

impl From<v15::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v15::DuplicateTorrent) -> Self {
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

impl From<v15::ErroredTorrent> for ErroredTorrent {
    fn from(t: v15::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v15::TorrentMeta> for TorrentMeta {
    fn from(t: v15::TorrentMeta) -> Self {
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
        }
    }
}

impl From<v15::Category> for Category {
    fn from(value: v15::Category) -> Self {
        match value {
            v15::Category::Action => Category::Action,
            v15::Category::Art => Category::Art,
            v15::Category::Biographical => Category::Biographical,
            v15::Category::Business => Category::Business,
            v15::Category::Comedy => Category::Comedy,
            v15::Category::CompleteEditionsMusic => Category::CompleteEditionsMusic,
            v15::Category::Computer => Category::Computer,
            v15::Category::Crafts => Category::Crafts,
            v15::Category::Crime => Category::Crime,
            v15::Category::Dramatization => Category::Dramatization,
            v15::Category::Education => Category::Education,
            v15::Category::FactualNews => Category::FactualNews,
            v15::Category::Fantasy => Category::Fantasy,
            v15::Category::Food => Category::Food,
            v15::Category::Guitar => Category::Guitar,
            v15::Category::Health => Category::Health,
            v15::Category::Historical => Category::Historical,
            v15::Category::Home => Category::Home,
            v15::Category::Horror => Category::Horror,
            v15::Category::Humor => Category::Humor,
            v15::Category::IndividualSheet => Category::IndividualSheet,
            v15::Category::Instructional => Category::Instructional,
            v15::Category::Juvenile => Category::Juvenile,
            v15::Category::Language => Category::Language,
            v15::Category::Lgbt => Category::Lgbt,
            v15::Category::LickLibraryLTP => Category::LickLibraryLTP,
            v15::Category::LickLibraryTechniques => Category::LickLibraryTechniques,
            v15::Category::LiteraryClassics => Category::LiteraryClassics,
            v15::Category::LitRPG => Category::LitRPG,
            v15::Category::Math => Category::Math,
            v15::Category::Medicine => Category::Medicine,
            v15::Category::Music => Category::Music,
            v15::Category::MusicBook => Category::MusicBook,
            v15::Category::Mystery => Category::Mystery,
            v15::Category::Nature => Category::Nature,
            v15::Category::Paranormal => Category::Paranormal,
            v15::Category::Philosophy => Category::Philosophy,
            v15::Category::Poetry => Category::Poetry,
            v15::Category::Politics => Category::Politics,
            v15::Category::Reference => Category::Reference,
            v15::Category::Religion => Category::Religion,
            v15::Category::Romance => Category::Romance,
            v15::Category::Rpg => Category::Rpg,
            v15::Category::Science => Category::Science,
            v15::Category::ScienceFiction => Category::ScienceFiction,
            v15::Category::SelfHelp => Category::SelfHelp,
            v15::Category::SheetCollection => Category::SheetCollection,
            v15::Category::SheetCollectionMP3 => Category::SheetCollectionMP3,
            v15::Category::Sports => Category::Sports,
            v15::Category::Technology => Category::Technology,
            v15::Category::Thriller => Category::Thriller,
            v15::Category::Travel => Category::Travel,
            v15::Category::UrbanFantasy => Category::UrbanFantasy,
            v15::Category::Western => Category::Western,
            v15::Category::YoungAdult => Category::YoungAdult,
            v15::Category::Superheroes => Category::Superheroes,
            v15::Category::LiteraryFiction => Category::LiteraryFiction,
            v15::Category::ProgressionFantasy => Category::ProgressionFantasy,
            v15::Category::ContemporaryFiction => Category::Unknown(59),
            v15::Category::DramaPlays => Category::DramaPlays,
            v15::Category::Unknown(id) => Category::Unknown(id),
        }
    }
}
