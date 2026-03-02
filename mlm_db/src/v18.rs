use crate::ids;

use super::{v01, v03, v04, v05, v06, v08, v09, v10, v11, v12, v13, v15, v16, v17};
use mlm_parse::{normalize_title, parse_edition};
use native_db::{ToKey, native_db};
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 18, from = v17::Torrent)]
#[native_db(export_keys = true)]
pub struct Torrent {
    #[primary_key]
    pub id: String,
    pub id_is_hash: bool,
    #[secondary_key(unique, optional)]
    pub mam_id: Option<u64>,
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
    pub library_mismatch: Option<v08::LibraryMismatch>,
    pub client_status: Option<ClientStatus>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ClientStatus {
    NotInClient,
    RemovedFromTracker,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 18, from = v17::SelectedTorrent)]
#[native_db(export_keys = true)]
pub struct SelectedTorrent {
    #[primary_key]
    pub mam_id: u64,
    #[secondary_key(unique, optional)]
    pub hash: Option<String>,
    pub dl_link: String,
    pub unsat_buffer: Option<u64>,
    pub wedge_buffer: Option<u64>,
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
#[native_model(id = 4, version = 18, from = v17::DuplicateTorrent)]
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
#[native_model(id = 5, version = 18, from = v17::ErroredTorrent)]
#[native_db(export_keys = true)]
pub struct ErroredTorrent {
    #[primary_key]
    pub id: v11::ErroredTorrentId,
    pub title: String,
    pub error: String,
    pub meta: Option<TorrentMeta>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct TorrentMeta {
    pub ids: BTreeMap<String, String>,
    pub vip_status: Option<v11::VipStatus>,
    pub cat: Option<v16::OldCategory>,
    pub media_type: v13::MediaType,
    pub main_cat: Option<v12::MainCat>,
    pub categories: Vec<Category>,
    pub tags: Vec<String>,
    pub language: Option<v03::Language>,
    pub flags: Option<v08::FlagBits>,
    pub filetypes: Vec<String>,
    pub num_files: u64,
    pub size: v03::Size,
    pub title: String,
    pub edition: Option<(String, u64)>,
    pub description: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<v09::Series>,
    pub source: MetadataSource,
    pub uploaded_at: v03::Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    // Fiction (Genres)
    Fantasy,
    ScienceFiction,
    Romance,
    Historical,
    ContemporaryRealist,
    Mystery,
    Thriller,
    Crime,
    Horror,
    ActionAdventure,
    Dystopian,
    PostApocalyptic,
    MagicalRealism,
    Western,
    Military,

    // SFF Subtypes
    EpicFantasy,
    UrbanFantasy,
    SwordAndSorcery,
    HardSciFi,
    SpaceOpera,
    Cyberpunk,
    TimeTravel,
    AlternateHistory,
    ProgressionFantasy,

    // Romance Subtypes
    RomanticComedy,
    RomanticSuspense,
    ParanormalRomance,
    DarkRomance,
    WhyChoose,
    Erotica,

    // Crime & Mystery Subtypes
    Detective,
    Noir,
    LegalThriller,
    PsychologicalThriller,
    CozyMystery,

    // Horror Subtypes
    BodyHorror,
    GothicHorror,
    CosmicHorror,
    ParanormalHorror,

    // Identity & Representation
    Lgbtqia,
    TransRepresentation,
    DisabilityRepresentation,
    NeurodivergentRepresentation,
    PocRepresentation,

    // Themes & Tropes
    ComingOfAge,
    FoundFamily,
    EnemiesToLovers,
    FriendsToLovers,
    FakeDating,
    SecondChance,
    SlowBurn,
    PoliticalIntrigue,
    Revenge,
    Redemption,
    Survival,
    Retelling,

    // Setting & Time
    Ancient,
    Medieval,
    EarlyModern,
    NineteenthCentury,
    TwentiethCentury,
    Contemporary,
    Future,
    AlternateTimeline,
    AlternateUniverse,
    SmallTown,
    Urban,
    Rural,
    AcademySchool,
    Space,

    // Region
    Africa,
    EastAsia,
    SouthAsia,
    SoutheastAsia,
    MiddleEast,
    Europe,
    NorthAmerica,
    LatinAmerica,
    Caribbean,
    Oceania,

    // Tone & Vibe
    Cozy,
    Dark,
    Gritty,
    Wholesome,
    Funny,
    Satire,
    Emotional,
    CharacterDriven,

    // Audience
    Children,
    MiddleGrade,
    YoungAdult,
    NewAdult,
    Adult,

    // Format
    Audiobook,
    Ebook,
    GraphicNovelsComics,
    Manga,
    Novella,
    LightNovel,
    ShortStories,
    Anthology,
    Poetry,
    Essays,
    Epistolary,
    DramaPlays,

    // Audio & Performance
    FullCast,
    DualNarration,
    DuetNarration,
    DramatizedAdaptation,
    AuthorNarrated,
    Abridged,

    // Non-Fiction (Subjects)
    Biography,
    Memoir,
    History,
    TrueCrime,
    Philosophy,
    ReligionSpirituality,
    MythologyFolklore,
    OccultEsotericism,
    PoliticsSociety,
    Business,
    PersonalFinance,
    ParentingFamily,
    SelfHelp,
    Psychology,
    HealthWellness,
    Science,
    Technology,
    Travel,

    // STEM & Technical
    Mathematics,
    ComputerScience,
    DataAi,
    Medicine,
    NatureEnvironment,
    Engineering,

    // Arts & Culture
    ArtPhotography,
    Music,
    SheetMusicScores,
    FilmTelevision,
    PopCulture,
    Humor,
    LiteraryCriticism,

    // Lifestyle & Hobbies
    CookingFood,
    HomeGarden,
    CraftsDiy,
    SportsOutdoors,

    // Education & Reference
    Textbook,
    Reference,
    Workbook,
    GuideManual,
    LanguageLinguistics,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub enum MetadataSource {
    #[default]
    Mam,
    Manual,
    File,
    Match,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[native_model(id = 6, version = 18, from = v17::Event)]
#[native_db(export_keys = true)]
pub struct Event {
    #[primary_key]
    pub id: v03::Uuid,
    #[secondary_key]
    pub torrent_id: Option<String>,
    #[secondary_key]
    pub mam_id: Option<u64>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
    pub event: EventType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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
        source: (MetadataSource, String),
    },
    RemovedFromTracker,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TorrentMetaDiff {
    pub field: TorrentMetaField,
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum TorrentMetaField {
    Ids,
    Vip,
    Cat,
    MediaType,
    MainCat,
    Categories,
    Tags,
    Language,
    Flags,
    Filetypes,
    Size,
    Title,
    Edition,
    Authors,
    Narrators,
    Series,
    Source,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 8, version = 18, from = v05::ListItem)]
#[native_db(export_keys = true)]
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
    pub prefer_format: Option<v01::MainCat>,
    pub allow_audio: bool,
    pub audio_torrent: Option<ListItemTorrent>,
    pub allow_ebook: bool,
    pub ebook_torrent: Option<ListItemTorrent>,
    #[secondary_key]
    pub created_at: v03::Timestamp,
    pub marked_done_at: Option<v03::Timestamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListItemTorrent {
    pub torrent_id: Option<String>,
    pub mam_id: Option<u64>,
    pub status: v04::TorrentStatus,
    pub at: v03::Timestamp,
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn add_mapped_category(categories: &mut Vec<Category>, category: Category) {
    push_unique(categories, category);
}

fn add_freeform_tag(categories: &[Category], tags: &mut Vec<String>, tag: &str) {
    let tag = tag.trim();
    if tag.is_empty() || categories.iter().any(|category| category.as_str() == tag) {
        return;
    }
    if !tags.iter().any(|existing| existing == tag) {
        tags.push(tag.to_string());
    }
}

fn add_categories(categories: &mut Vec<Category>, mapped: &[Category]) {
    for category in mapped {
        add_mapped_category(categories, *category);
    }
}

fn migrate_audio_category(
    cat: v06::AudiobookCategory,
    categories: &mut Vec<Category>,
    _tags: &mut Vec<String>,
) {
    match cat {
        v06::AudiobookCategory::ActionAdventure => {
            add_categories(categories, &[Category::ActionAdventure])
        }
        v06::AudiobookCategory::Art => add_categories(categories, &[Category::ArtPhotography]),
        v06::AudiobookCategory::Biographical => add_categories(categories, &[Category::Biography]),
        v06::AudiobookCategory::Business => add_categories(categories, &[Category::Business]),
        v06::AudiobookCategory::ComputerInternet => {
            add_categories(categories, &[Category::ComputerScience])
        }
        v06::AudiobookCategory::Crafts => add_categories(categories, &[Category::CraftsDiy]),
        v06::AudiobookCategory::CrimeThriller => {
            add_categories(categories, &[Category::Crime, Category::Thriller])
        }
        v06::AudiobookCategory::Fantasy => add_categories(categories, &[Category::Fantasy]),
        v06::AudiobookCategory::Food => add_categories(categories, &[Category::CookingFood]),
        v06::AudiobookCategory::GeneralFiction => add_categories(
            categories,
            &[Category::ContemporaryRealist, Category::CharacterDriven],
        ),
        v06::AudiobookCategory::GeneralNonFic => add_categories(categories, &[Category::Reference]),
        v06::AudiobookCategory::HistoricalFiction => {
            add_categories(categories, &[Category::Historical])
        }
        v06::AudiobookCategory::History => add_categories(categories, &[Category::History]),
        v06::AudiobookCategory::HomeGarden => add_categories(categories, &[Category::HomeGarden]),
        v06::AudiobookCategory::Horror => add_categories(categories, &[Category::Horror]),
        v06::AudiobookCategory::Humor => {
            add_categories(categories, &[Category::Funny, Category::Humor])
        }
        v06::AudiobookCategory::Instructional => {
            add_categories(categories, &[Category::GuideManual])
        }
        v06::AudiobookCategory::Juvenile => add_categories(categories, &[Category::Children]),
        v06::AudiobookCategory::Language => {
            add_categories(categories, &[Category::LanguageLinguistics])
        }
        v06::AudiobookCategory::LiteraryClassics => {
            add_categories(categories, &[Category::CharacterDriven])
        }
        v06::AudiobookCategory::MathScienceTech => {
            add_categories(categories, &[Category::Science, Category::Technology])
        }
        v06::AudiobookCategory::Medical => add_categories(categories, &[Category::Medicine]),
        v06::AudiobookCategory::Mystery => add_categories(categories, &[Category::Mystery]),
        v06::AudiobookCategory::Nature => {
            add_categories(categories, &[Category::NatureEnvironment])
        }
        v06::AudiobookCategory::Philosophy => add_categories(categories, &[Category::Philosophy]),
        v06::AudiobookCategory::PolSocRelig => add_categories(
            categories,
            &[Category::PoliticsSociety, Category::ReligionSpirituality],
        ),
        v06::AudiobookCategory::Recreation => {
            add_categories(categories, &[Category::SportsOutdoors])
        }
        v06::AudiobookCategory::Romance => add_categories(categories, &[Category::Romance]),
        v06::AudiobookCategory::ScienceFiction => {
            add_categories(categories, &[Category::ScienceFiction])
        }
        v06::AudiobookCategory::SelfHelp => add_categories(categories, &[Category::SelfHelp]),
        v06::AudiobookCategory::TravelAdventure => add_categories(categories, &[Category::Travel]),
        v06::AudiobookCategory::TrueCrime => add_categories(categories, &[Category::Crime]),
        v06::AudiobookCategory::UrbanFantasy => {
            add_categories(categories, &[Category::Fantasy, Category::UrbanFantasy])
        }
        v06::AudiobookCategory::Western => add_categories(categories, &[Category::Western]),
        v06::AudiobookCategory::YoungAdult => add_categories(categories, &[Category::YoungAdult]),
    }
}

fn migrate_ebook_category(
    cat: v06::EbookCategory,
    categories: &mut Vec<Category>,
    _tags: &mut Vec<String>,
) {
    match cat {
        v06::EbookCategory::ActionAdventure => {
            add_categories(categories, &[Category::ActionAdventure])
        }
        v06::EbookCategory::Art => add_categories(categories, &[Category::ArtPhotography]),
        v06::EbookCategory::Biographical => add_categories(categories, &[Category::Biography]),
        v06::EbookCategory::Business => add_categories(categories, &[Category::Business]),
        v06::EbookCategory::ComicsGraphicnovels => {
            add_categories(categories, &[Category::GraphicNovelsComics])
        }
        v06::EbookCategory::ComputerInternet => {
            add_categories(categories, &[Category::ComputerScience])
        }
        v06::EbookCategory::Crafts => add_categories(categories, &[Category::CraftsDiy]),
        v06::EbookCategory::CrimeThriller => {
            add_categories(categories, &[Category::Crime, Category::Thriller])
        }
        v06::EbookCategory::Fantasy => add_categories(categories, &[Category::Fantasy]),
        v06::EbookCategory::Food => add_categories(categories, &[Category::CookingFood]),
        v06::EbookCategory::GeneralFiction => add_categories(
            categories,
            &[Category::ContemporaryRealist, Category::CharacterDriven],
        ),
        v06::EbookCategory::GeneralNonFiction => add_categories(categories, &[Category::Reference]),
        v06::EbookCategory::HistoricalFiction => {
            add_categories(categories, &[Category::Historical])
        }
        v06::EbookCategory::History => add_categories(categories, &[Category::History]),
        v06::EbookCategory::HomeGarden => add_categories(categories, &[Category::HomeGarden]),
        v06::EbookCategory::Horror => add_categories(categories, &[Category::Horror]),
        v06::EbookCategory::Humor => {
            add_categories(categories, &[Category::Funny, Category::Humor])
        }
        v06::EbookCategory::IllusionMagic => {
            add_categories(categories, &[Category::MythologyFolklore])
        }
        v06::EbookCategory::Instructional => add_categories(categories, &[Category::GuideManual]),
        v06::EbookCategory::Juvenile => add_categories(categories, &[Category::Children]),
        v06::EbookCategory::Language => {
            add_categories(categories, &[Category::LanguageLinguistics])
        }
        v06::EbookCategory::LiteraryClassics => {
            add_categories(categories, &[Category::CharacterDriven])
        }
        v06::EbookCategory::MagazinesNewspapers => {
            add_categories(categories, &[Category::Reference])
        }
        v06::EbookCategory::MathScienceTech => {
            add_categories(categories, &[Category::Science, Category::Technology])
        }
        v06::EbookCategory::Medical => add_categories(categories, &[Category::Medicine]),
        v06::EbookCategory::MixedCollections => add_categories(categories, &[Category::Anthology]),
        v06::EbookCategory::Mystery => add_categories(categories, &[Category::Mystery]),
        v06::EbookCategory::Nature => add_categories(categories, &[Category::NatureEnvironment]),
        v06::EbookCategory::Philosophy => add_categories(categories, &[Category::Philosophy]),
        v06::EbookCategory::PolSocRelig => add_categories(
            categories,
            &[Category::PoliticsSociety, Category::ReligionSpirituality],
        ),
        v06::EbookCategory::Recreation => add_categories(categories, &[Category::SportsOutdoors]),
        v06::EbookCategory::Romance => add_categories(categories, &[Category::Romance]),
        v06::EbookCategory::ScienceFiction => {
            add_categories(categories, &[Category::ScienceFiction])
        }
        v06::EbookCategory::SelfHelp => add_categories(categories, &[Category::SelfHelp]),
        v06::EbookCategory::TravelAdventure => add_categories(categories, &[Category::Travel]),
        v06::EbookCategory::TrueCrime => add_categories(categories, &[Category::Crime]),
        v06::EbookCategory::UrbanFantasy => {
            add_categories(categories, &[Category::Fantasy, Category::UrbanFantasy])
        }
        v06::EbookCategory::Western => add_categories(categories, &[Category::Western]),
        v06::EbookCategory::YoungAdult => add_categories(categories, &[Category::YoungAdult]),
    }
}

fn migrate_old_category(
    cat: &v16::OldCategory,
    categories: &mut Vec<Category>,
    tags: &mut Vec<String>,
) {
    match cat {
        v16::OldCategory::Audio(cat) => {
            add_mapped_category(categories, Category::Audiobook);
            migrate_audio_category(*cat, categories, tags);
        }
        v16::OldCategory::Ebook(cat) => {
            add_mapped_category(categories, Category::Ebook);
            migrate_ebook_category(*cat, categories, tags);
        }
        v16::OldCategory::Musicology(cat) => match cat {
            v16::MusicologyCategory::GuitarBassTabs
            | v16::MusicologyCategory::IndividualSheet
            | v16::MusicologyCategory::IndividualSheetMP3
            | v16::MusicologyCategory::SheetCollection
            | v16::MusicologyCategory::SheetCollectionMP3 => {
                add_categories(categories, &[Category::SheetMusicScores])
            }
            v16::MusicologyCategory::MusicCompleteEditions
            | v16::MusicologyCategory::MusicBook
            | v16::MusicologyCategory::MusicBookMP3 => {
                add_categories(categories, &[Category::Music])
            }
            v16::MusicologyCategory::InstructionalBookWithVideo
            | v16::MusicologyCategory::InstructionalMediaMusic
            | v16::MusicologyCategory::LickLibraryLTPJamWith
            | v16::MusicologyCategory::LickLibraryTechniquesQL => {
                add_categories(categories, &[Category::GuideManual])
            }
        },
        v16::OldCategory::Radio(cat) => match cat {
            v16::RadioCategory::Comedy => add_categories(categories, &[Category::Funny]),
            v16::RadioCategory::Drama => add_categories(categories, &[Category::CharacterDriven]),
            _ => add_freeform_tag(categories, tags, cat.to_str()),
        },
    }
}

fn legacy_v15_label(category: &v15::Category) -> String {
    match category {
        v15::Category::Action => "Action/Adventure".to_string(),
        v15::Category::Art => "Art/Photography".to_string(),
        v15::Category::Biographical => "Biographical".to_string(),
        v15::Category::Business => "Business/Money".to_string(),
        v15::Category::Comedy => "Comedy".to_string(),
        v15::Category::CompleteEditionsMusic => "Complete Editions - Music".to_string(),
        v15::Category::Computer => "Computer/Internet".to_string(),
        v15::Category::Crafts => "Crafts".to_string(),
        v15::Category::Crime => "Crime".to_string(),
        v15::Category::Dramatization => "Dramatization/Full Cast".to_string(),
        v15::Category::Education => "Education/Textbook".to_string(),
        v15::Category::FactualNews => "Factual News/Current Events".to_string(),
        v15::Category::Fantasy => "Fantasy".to_string(),
        v15::Category::Food => "Food/Wine".to_string(),
        v15::Category::Guitar => "Guitar/Bass Tabs".to_string(),
        v15::Category::Health => "Health/Fitness/Diet".to_string(),
        v15::Category::Historical => "Historical".to_string(),
        v15::Category::Home => "Home/Garden".to_string(),
        v15::Category::Horror => "Horror".to_string(),
        v15::Category::Humor => "Humor".to_string(),
        v15::Category::IndividualSheet => "Individual Sheet".to_string(),
        v15::Category::Instructional => "Instructional".to_string(),
        v15::Category::Juvenile => "Juvenile".to_string(),
        v15::Category::Language => "Language".to_string(),
        v15::Category::Lgbt => "LGBTQIA+".to_string(),
        v15::Category::LickLibraryLTP => "Lick Library - LTP/Jam With".to_string(),
        v15::Category::LickLibraryTechniques => "Lick Library - Techniques/QL".to_string(),
        v15::Category::LiteraryClassics => "Literary Classics".to_string(),
        v15::Category::LitRPG => "LitRPG".to_string(),
        v15::Category::Math => "Math".to_string(),
        v15::Category::Medicine => "Medicine/Psychology".to_string(),
        v15::Category::Music => "Music".to_string(),
        v15::Category::MusicBook => "Music Book".to_string(),
        v15::Category::Mystery => "Mystery".to_string(),
        v15::Category::Nature => "Nature".to_string(),
        v15::Category::Paranormal => "Paranormal".to_string(),
        v15::Category::Philosophy => "Philosophy".to_string(),
        v15::Category::Poetry => "Poetry".to_string(),
        v15::Category::Politics => "Politics/Sociology".to_string(),
        v15::Category::Reference => "Reference".to_string(),
        v15::Category::Religion => "Religion/Spirituality".to_string(),
        v15::Category::Romance => "Romance".to_string(),
        v15::Category::Rpg => "RPG".to_string(),
        v15::Category::Science => "Science".to_string(),
        v15::Category::ScienceFiction => "Science Fiction".to_string(),
        v15::Category::SelfHelp => "Self-Help".to_string(),
        v15::Category::SheetCollection => "Sheet Collection".to_string(),
        v15::Category::SheetCollectionMP3 => "Sheet Collection MP3".to_string(),
        v15::Category::Sports => "Sports/Hobbies".to_string(),
        v15::Category::Technology => "Technology".to_string(),
        v15::Category::Thriller => "Thriller/Suspense".to_string(),
        v15::Category::Travel => "Travel".to_string(),
        v15::Category::UrbanFantasy => "Urban Fantasy".to_string(),
        v15::Category::Western => "Western".to_string(),
        v15::Category::YoungAdult => "Young Adult".to_string(),
        v15::Category::Superheroes => "Superheroes".to_string(),
        v15::Category::LiteraryFiction => "Literary Fiction".to_string(),
        v15::Category::ProgressionFantasy => "Progression Fantasy".to_string(),
        v15::Category::ContemporaryFiction => "Contemporary Fiction".to_string(),
        v15::Category::DramaPlays => "Drama/Plays".to_string(),
        v15::Category::Unknown(61) => "Occult / Metaphysical Practices".to_string(),
        v15::Category::Unknown(62) => "Slice of Life".to_string(),
        v15::Category::Unknown(id) => format!("Unknown Category (id: {id})"),
    }
}

fn migrate_legacy_categories(
    cat: Option<&v16::OldCategory>,
    legacy_categories: &[v15::Category],
) -> (Vec<Category>, Vec<String>) {
    let mut categories = Vec::new();
    let mut tags = Vec::new();

    if let Some(cat) = cat {
        migrate_old_category(cat, &mut categories, &mut tags);
    }

    for legacy_category in legacy_categories {
        match legacy_category {
            v15::Category::Action => add_categories(&mut categories, &[Category::ActionAdventure]),
            v15::Category::Art => add_categories(&mut categories, &[Category::ArtPhotography]),
            v15::Category::Biographical => add_categories(&mut categories, &[Category::Biography]),
            v15::Category::Business => add_categories(&mut categories, &[Category::Business]),
            v15::Category::Comedy | v15::Category::Humor => {
                add_categories(&mut categories, &[Category::Funny, Category::Humor])
            }
            v15::Category::CompleteEditionsMusic
            | v15::Category::Music
            | v15::Category::MusicBook => add_categories(&mut categories, &[Category::Music]),
            v15::Category::Computer => {
                add_categories(&mut categories, &[Category::ComputerScience])
            }
            v15::Category::Crafts => add_categories(&mut categories, &[Category::CraftsDiy]),
            v15::Category::Crime => add_categories(&mut categories, &[Category::Crime]),
            v15::Category::Dramatization => add_categories(
                &mut categories,
                &[Category::DramatizedAdaptation, Category::FullCast],
            ),
            v15::Category::Education => add_categories(&mut categories, &[Category::Textbook]),
            v15::Category::FactualNews => {
                add_categories(&mut categories, &[Category::PoliticsSociety])
            }
            v15::Category::Fantasy => add_categories(&mut categories, &[Category::Fantasy]),
            v15::Category::Food => add_categories(&mut categories, &[Category::CookingFood]),
            v15::Category::Guitar
            | v15::Category::IndividualSheet
            | v15::Category::SheetCollection
            | v15::Category::SheetCollectionMP3 => {
                add_categories(&mut categories, &[Category::SheetMusicScores])
            }
            v15::Category::Health => add_categories(&mut categories, &[Category::HealthWellness]),
            v15::Category::Historical => add_categories(&mut categories, &[Category::Historical]),
            v15::Category::Home => add_categories(&mut categories, &[Category::HomeGarden]),
            v15::Category::Horror => add_categories(&mut categories, &[Category::Horror]),
            v15::Category::Lgbt => add_categories(&mut categories, &[Category::Lgbtqia]),
            v15::Category::Instructional
            | v15::Category::LickLibraryLTP
            | v15::Category::LickLibraryTechniques => {
                add_categories(&mut categories, &[Category::GuideManual])
            }
            v15::Category::Juvenile => add_categories(&mut categories, &[Category::Children]),
            v15::Category::Language => {
                add_categories(&mut categories, &[Category::LanguageLinguistics])
            }
            v15::Category::LiteraryClassics => {
                add_categories(&mut categories, &[Category::CharacterDriven])
            }
            v15::Category::LiteraryFiction => {
                add_categories(&mut categories, &[Category::CharacterDriven])
            }
            v15::Category::LitRPG | v15::Category::ProgressionFantasy => add_categories(
                &mut categories,
                &[Category::Fantasy, Category::ProgressionFantasy],
            ),
            v15::Category::Math => add_categories(&mut categories, &[Category::Mathematics]),
            v15::Category::Medicine => add_categories(&mut categories, &[Category::Medicine]),
            v15::Category::Mystery => add_categories(&mut categories, &[Category::Mystery]),
            v15::Category::Nature => {
                add_categories(&mut categories, &[Category::NatureEnvironment])
            }
            v15::Category::Philosophy => add_categories(&mut categories, &[Category::Philosophy]),
            v15::Category::Poetry => add_categories(&mut categories, &[Category::Poetry]),
            v15::Category::Politics => {
                add_categories(&mut categories, &[Category::PoliticsSociety])
            }
            v15::Category::Reference => add_categories(&mut categories, &[Category::Reference]),
            v15::Category::Religion => {
                add_categories(&mut categories, &[Category::ReligionSpirituality])
            }
            v15::Category::Romance => add_categories(&mut categories, &[Category::Romance]),
            v15::Category::Rpg => add_categories(&mut categories, &[Category::Fantasy]),
            v15::Category::Science => add_categories(&mut categories, &[Category::Science]),
            v15::Category::ScienceFiction => {
                add_categories(&mut categories, &[Category::ScienceFiction])
            }
            v15::Category::SelfHelp => add_categories(&mut categories, &[Category::SelfHelp]),
            v15::Category::Sports => add_categories(&mut categories, &[Category::SportsOutdoors]),
            v15::Category::Technology => add_categories(&mut categories, &[Category::Technology]),
            v15::Category::Thriller => add_categories(&mut categories, &[Category::Thriller]),
            v15::Category::Travel => add_categories(&mut categories, &[Category::Travel]),
            v15::Category::UrbanFantasy => add_categories(
                &mut categories,
                &[Category::Fantasy, Category::UrbanFantasy],
            ),
            v15::Category::Western => add_categories(&mut categories, &[Category::Western]),
            v15::Category::YoungAdult => add_categories(&mut categories, &[Category::YoungAdult]),
            v15::Category::Superheroes => {
                add_categories(&mut categories, &[Category::ActionAdventure])
            }
            v15::Category::ContemporaryFiction => {
                add_categories(&mut categories, &[Category::ContemporaryRealist])
            }
            v15::Category::DramaPlays => add_categories(
                &mut categories,
                &[Category::DramaPlays, Category::CharacterDriven],
            ),
            v15::Category::Paranormal => {
                add_categories(&mut categories, &[Category::ParanormalHorror])
            }
            v15::Category::Unknown(61) => {
                add_categories(&mut categories, &[Category::OccultEsotericism])
            }
            v15::Category::Unknown(62) => add_categories(
                &mut categories,
                &[Category::ContemporaryRealist, Category::CharacterDriven],
            ),
            _ => {
                let label = legacy_v15_label(legacy_category);
                add_freeform_tag(&categories, &mut tags, &label);
            }
        }
    }

    (categories, tags)
}

impl From<v17::Torrent> for Torrent {
    fn from(t: v17::Torrent) -> Self {
        let mut meta: TorrentMeta = t.meta.into();
        if let Some(abs_id) = t.abs_id {
            meta.ids.insert(ids::ABS.to_string(), abs_id.to_string());
        }
        if let Some(goodreads_id) = t.goodreads_id {
            meta.ids
                .insert(ids::GOODREADS.to_string(), goodreads_id.to_string());
        }

        Self {
            id: t.id,
            id_is_hash: t.id_is_hash,
            mam_id: Some(t.mam_id),
            library_path: t.library_path,
            library_files: t.library_files,
            linker: t.linker,
            category: t.category,
            selected_audio_format: t.selected_audio_format,
            selected_ebook_format: t.selected_ebook_format,
            title_search: normalize_title(&meta.title),
            meta,
            created_at: t.created_at,
            replaced_with: t.replaced_with,
            library_mismatch: t.library_mismatch,
            client_status: t.client_status.map(Into::into),
        }
    }
}

impl From<v08::ClientStatus> for ClientStatus {
    fn from(value: v08::ClientStatus) -> Self {
        match value {
            v08::ClientStatus::NotInClient => Self::NotInClient,
            v08::ClientStatus::RemovedFromMam => Self::RemovedFromTracker,
        }
    }
}

impl From<v17::SelectedTorrent> for SelectedTorrent {
    fn from(t: v17::SelectedTorrent) -> Self {
        let mut meta: TorrentMeta = t.meta.into();
        if let Some(goodreads_id) = t.goodreads_id {
            meta.ids
                .insert(ids::GOODREADS.to_string(), goodreads_id.to_string());
        }

        Self {
            mam_id: t.mam_id,
            hash: t.hash,
            dl_link: t.dl_link,
            unsat_buffer: t.unsat_buffer,
            wedge_buffer: None,
            cost: t.cost,
            category: t.category,
            tags: t.tags,
            title_search: normalize_title(&meta.title),
            meta,
            grabber: t.grabber,
            created_at: t.created_at,
            started_at: t.started_at,
            removed_at: t.removed_at,
        }
    }
}

impl From<v17::DuplicateTorrent> for DuplicateTorrent {
    fn from(t: v17::DuplicateTorrent) -> Self {
        let meta: TorrentMeta = t.meta.into();
        Self {
            mam_id: t.mam_id,
            dl_link: t.dl_link,
            title_search: normalize_title(&meta.title),
            meta,
            created_at: t.created_at,
            duplicate_of: t.duplicate_of,
        }
    }
}

impl From<v17::ErroredTorrent> for ErroredTorrent {
    fn from(t: v17::ErroredTorrent) -> Self {
        Self {
            id: t.id,
            title: t.title,
            error: t.error,
            meta: t.meta.map(|t| t.into()),
            created_at: t.created_at,
        }
    }
}

impl From<v17::TorrentMeta> for TorrentMeta {
    fn from(t: v17::TorrentMeta) -> Self {
        let (title, edition) = parse_edition(&t.title, "");
        let mut ids = BTreeMap::default();
        ids.insert(ids::MAM.to_string(), t.mam_id.to_string());
        let (categories, tags) = migrate_legacy_categories(t.cat.as_ref(), &t.categories);

        Self {
            ids,
            vip_status: t.vip_status,
            cat: t.cat,
            media_type: t.media_type,
            main_cat: t.main_cat,
            categories,
            tags,
            language: t.language,
            flags: t.flags,
            filetypes: t.filetypes,
            num_files: t.num_files,
            size: t.size,
            title,
            edition,
            description: String::new(),
            authors: t.authors,
            narrators: t.narrators,
            series: t.series,
            source: t.source.into(),
            uploaded_at: t.uploaded_at,
        }
    }
}

impl From<v10::MetadataSource> for MetadataSource {
    fn from(t: v10::MetadataSource) -> Self {
        match t {
            v10::MetadataSource::Mam => Self::Mam,
            v10::MetadataSource::Manual => Self::Manual,
        }
    }
}

impl From<v17::Event> for Event {
    fn from(t: v17::Event) -> Self {
        Self {
            id: t.id,
            torrent_id: t.torrent_id,
            mam_id: t.mam_id,
            created_at: t.created_at,
            event: t.event.into(),
        }
    }
}

impl From<v17::EventType> for EventType {
    fn from(t: v17::EventType) -> Self {
        match t {
            v17::EventType::Grabbed {
                grabber,
                cost,
                wedged,
            } => Self::Grabbed {
                grabber,
                cost,
                wedged,
            },
            v17::EventType::Linked {
                linker,
                library_path,
            } => Self::Linked {
                linker,
                library_path,
            },
            v17::EventType::Cleaned {
                library_path,
                files,
            } => Self::Cleaned {
                library_path,
                files,
            },
            v17::EventType::Updated { fields } => Self::Updated {
                fields: fields.into_iter().map(Into::into).collect(),
                source: (MetadataSource::Mam, String::new()),
            },
            v17::EventType::RemovedFromMam => Self::RemovedFromTracker,
        }
    }
}

impl From<v17::TorrentMetaDiff> for TorrentMetaDiff {
    fn from(value: v17::TorrentMetaDiff) -> Self {
        Self {
            field: value.field.into(),
            from: value.from,
            to: value.to,
        }
    }
}

impl From<v17::TorrentMetaField> for TorrentMetaField {
    fn from(value: v17::TorrentMetaField) -> Self {
        match value {
            v17::TorrentMetaField::MamId => TorrentMetaField::Ids,
            v17::TorrentMetaField::Vip => TorrentMetaField::Vip,
            v17::TorrentMetaField::MediaType => TorrentMetaField::MediaType,
            v17::TorrentMetaField::MainCat => TorrentMetaField::MainCat,
            v17::TorrentMetaField::Categories => TorrentMetaField::Categories,
            v17::TorrentMetaField::Cat => TorrentMetaField::Cat,
            v17::TorrentMetaField::Language => TorrentMetaField::Language,
            v17::TorrentMetaField::Flags => TorrentMetaField::Flags,
            v17::TorrentMetaField::Filetypes => TorrentMetaField::Filetypes,
            v17::TorrentMetaField::Size => TorrentMetaField::Size,
            v17::TorrentMetaField::Title => TorrentMetaField::Title,
            v17::TorrentMetaField::Authors => TorrentMetaField::Authors,
            v17::TorrentMetaField::Narrators => TorrentMetaField::Narrators,
            v17::TorrentMetaField::Series => TorrentMetaField::Series,
            v17::TorrentMetaField::Source => TorrentMetaField::Source,
            v17::TorrentMetaField::Edition => TorrentMetaField::Edition,
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
            series: t.series,
            cover_url: t.cover_url,
            book_url: t.book_url,
            isbn: t.isbn,
            prefer_format: t.prefer_format,
            allow_audio: t.allow_audio,
            audio_torrent: t.audio_torrent.map(Into::into),
            allow_ebook: t.allow_ebook,
            ebook_torrent: t.ebook_torrent.map(Into::into),
            created_at: t.created_at,
            marked_done_at: t.marked_done_at,
        }
    }
}

impl From<v04::ListItemTorrent> for ListItemTorrent {
    fn from(t: v04::ListItemTorrent) -> Self {
        Self {
            torrent_id: None,
            mam_id: Some(t.mam_id),
            status: t.status,
            at: t.at,
        }
    }
}
