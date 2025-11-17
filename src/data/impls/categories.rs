use crate::{
    config::Config,
    data::{Category, MainCat, MediaType, OldMainCat},
};

impl MediaType {
    pub fn from_id(id: u8) -> Option<MediaType> {
        match id {
            1 => Some(MediaType::Audiobook),
            2 => Some(MediaType::Ebook),
            3 => Some(MediaType::Musicology),
            4 => Some(MediaType::Radio),
            5 => Some(MediaType::Manga),
            6 => Some(MediaType::ComicBook),
            7 => Some(MediaType::PeriodicalEbook),
            8 => Some(MediaType::PeriodicalAudiobook),
            _ => None,
        }
    }

    pub(crate) fn from_main_cat_id(main_cat: u8) -> Option<MediaType> {
        match main_cat {
            13 => Some(MediaType::Audiobook),
            14 => Some(MediaType::Ebook),
            15 => Some(MediaType::Musicology),
            16 => Some(MediaType::Radio),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Audiobook => "Audiobook",
            MediaType::Ebook => "Ebook",
            MediaType::Musicology => "Musicology",
            MediaType::Radio => "Radio",
            MediaType::Manga => "Manga",
            MediaType::ComicBook => "Comic Book / Graphic Novel",
            MediaType::PeriodicalEbook => "Periodical Ebook",
            MediaType::PeriodicalAudiobook => "Periodical Audiobook",
        }
    }

    pub fn as_id(&self) -> u8 {
        match self {
            MediaType::Audiobook => 1,
            MediaType::Ebook => 2,
            MediaType::Musicology => 3,
            MediaType::Radio => 4,
            MediaType::Manga => 5,
            MediaType::ComicBook => 6,
            MediaType::PeriodicalEbook => 7,
            MediaType::PeriodicalAudiobook => 8,
        }
    }

    pub fn preferred_types<'a>(&self, config: &'a Config) -> &'a [String] {
        match self {
            MediaType::Audiobook => &config.audio_types,
            MediaType::Ebook => &config.ebook_types,
            MediaType::Musicology => &config.music_types,
            MediaType::Radio => &config.radio_types,
            MediaType::Manga => &config.ebook_types,
            MediaType::ComicBook => &config.ebook_types,
            MediaType::PeriodicalEbook => &config.ebook_types,
            MediaType::PeriodicalAudiobook => &config.audio_types,
        }
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<OldMainCat> for MediaType {
    fn from(value: OldMainCat) -> Self {
        match value {
            OldMainCat::Audio => MediaType::Audiobook,
            OldMainCat::Ebook => MediaType::Ebook,
        }
    }
}

impl MainCat {
    pub fn from_id(id: u8) -> Option<MainCat> {
        match id {
            1 => Some(MainCat::Fiction),
            2 => Some(MainCat::Nonfiction),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MainCat::Fiction => "Fiction",
            MainCat::Nonfiction => "Nonfiction",
        }
    }

    pub fn as_id(&self) -> u8 {
        match self {
            MainCat::Fiction => 1,
            MainCat::Nonfiction => 2,
        }
    }
}

impl std::fmt::Display for MainCat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Category {
    pub fn from_id(id: u64) -> Option<Category> {
        match id {
            1 => Some(Category::Action),
            2 => Some(Category::Art),
            3 => Some(Category::Biographical),
            4 => Some(Category::Business),
            5 => Some(Category::Comedy),
            6 => Some(Category::CompleteEditionsMusic),
            7 => Some(Category::Computer),
            8 => Some(Category::Crafts),
            9 => Some(Category::Crime),
            10 => Some(Category::Dramatization),
            11 => Some(Category::Education),
            12 => Some(Category::FactualNews),
            13 => Some(Category::Fantasy),
            14 => Some(Category::Food),
            15 => Some(Category::Guitar),
            16 => Some(Category::Health),
            17 => Some(Category::Historical),
            18 => Some(Category::Home),
            19 => Some(Category::Horror),
            20 => Some(Category::Humor),
            21 => Some(Category::IndividualSheet),
            22 => Some(Category::Instructional),
            23 => Some(Category::Juvenile),
            24 => Some(Category::Language),
            25 => Some(Category::Lgbt),
            26 => Some(Category::LickLibraryLTP),
            27 => Some(Category::LickLibraryTechniques),
            28 => Some(Category::LiteraryClassics),
            29 => Some(Category::LitRPG),
            30 => Some(Category::Math),
            31 => Some(Category::Medicine),
            32 => Some(Category::Music),
            33 => Some(Category::MusicBook),
            34 => Some(Category::Mystery),
            35 => Some(Category::Nature),
            36 => Some(Category::Paranormal),
            37 => Some(Category::Philosophy),
            38 => Some(Category::Poetry),
            39 => Some(Category::Politics),
            40 => Some(Category::Reference),
            41 => Some(Category::Religion),
            42 => Some(Category::Romance),
            43 => Some(Category::Rpg),
            44 => Some(Category::Science),
            45 => Some(Category::ScienceFiction),
            46 => Some(Category::SelfHelp),
            47 => Some(Category::SheetCollection),
            48 => Some(Category::SheetCollectionMP3),
            49 => Some(Category::Sports),
            50 => Some(Category::Technology),
            51 => Some(Category::Thriller),
            52 => Some(Category::Travel),
            53 => Some(Category::UrbanFantasy),
            54 => Some(Category::Western),
            55 => Some(Category::YoungAdult),
            56 => Some(Category::Superheroes),
            57 => Some(Category::LiteraryFiction),
            58 => Some(Category::ProgressionFantasy),
            // 59 is already removed?
            60 => Some(Category::DramaPlays),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Action => "Action/Adventure",
            Category::Art => "Art/Photography",
            Category::Biographical => "Biographical",
            Category::Business => "Business/Money",
            Category::Comedy => "Comedy",
            Category::CompleteEditionsMusic => "Complete Editions - Music",
            Category::Computer => "Computer/Internet",
            Category::Crafts => "Crafts",
            Category::Crime => "Crime",
            Category::Dramatization => "Dramatization/Full Cast",
            Category::Education => "Education/Textbook",
            Category::FactualNews => "Factual News/Current Events",
            Category::Fantasy => "Fantasy",
            Category::Food => "Food/Wine",
            Category::Guitar => "Guitar/Bass Tabs",
            Category::Health => "Health/Fitness/Diet",
            Category::Historical => "Historical",
            Category::Home => "Home/Garden",
            Category::Horror => "Horror",
            Category::Humor => "Humor",
            Category::IndividualSheet => "Individual Sheet",
            Category::Instructional => "Instructional",
            Category::Juvenile => "Juvenile",
            Category::Language => "Language",
            Category::Lgbt => "LGBTQIA+",
            Category::LickLibraryLTP => "Lick Library - LTP/Jam With",
            Category::LickLibraryTechniques => "Lick Library - Techniques/QL",
            Category::LiteraryClassics => "Literary Classics",
            Category::LitRPG => "LitRPG",
            Category::Math => "Math",
            Category::Medicine => "Medicine/Psychology",
            Category::Music => "Music",
            Category::MusicBook => "Music Book",
            Category::Mystery => "Mystery",
            Category::Nature => "Nature",
            Category::Paranormal => "Paranormal",
            Category::Philosophy => "Philosophy",
            Category::Poetry => "Poetry",
            Category::Politics => "Politics/Sociology",
            Category::Reference => "Reference",
            Category::Religion => "Religion/Spirituality",
            Category::Romance => "Romance",
            Category::Rpg => "RPG",
            Category::Science => "Science",
            Category::ScienceFiction => "Science Fiction",
            Category::SelfHelp => "Self-Help",
            Category::SheetCollection => "Sheet Collection",
            Category::SheetCollectionMP3 => "Sheet Collection MP3",
            Category::Sports => "Sports/Hobbies",
            Category::Technology => "Technology",
            Category::Thriller => "Thriller/Suspense",
            Category::Travel => "Travel",
            Category::UrbanFantasy => "Urban Fantasy",
            Category::Western => "Western",
            Category::YoungAdult => "Young Adult",
            Category::Superheroes => "Superheroes",
            // Since there is no replacement for general fiction, literary fiction is being used
            // instead.
            Category::LiteraryFiction => "General Fiction",
            Category::ProgressionFantasy => "Progression Fantasy",
            Category::DramaPlays => "Drama/Plays",
            Category::Unknown(_) => "Unknown Category",
        }
    }

    pub fn as_id(&self) -> u8 {
        match self {
            Category::Action => 1,
            Category::Art => 2,
            Category::Biographical => 3,
            Category::Business => 4,
            Category::Comedy => 5,
            Category::CompleteEditionsMusic => 6,
            Category::Computer => 7,
            Category::Crafts => 8,
            Category::Crime => 9,
            Category::Dramatization => 10,
            Category::Education => 11,
            Category::FactualNews => 12,
            Category::Fantasy => 13,
            Category::Food => 14,
            Category::Guitar => 15,
            Category::Health => 16,
            Category::Historical => 17,
            Category::Home => 18,
            Category::Horror => 19,
            Category::Humor => 20,
            Category::IndividualSheet => 21,
            Category::Instructional => 22,
            Category::Juvenile => 23,
            Category::Language => 24,
            Category::Lgbt => 25,
            Category::LickLibraryLTP => 26,
            Category::LickLibraryTechniques => 27,
            Category::LiteraryClassics => 28,
            Category::LitRPG => 29,
            Category::Math => 30,
            Category::Medicine => 31,
            Category::Music => 32,
            Category::MusicBook => 33,
            Category::Mystery => 34,
            Category::Nature => 35,
            Category::Paranormal => 36,
            Category::Philosophy => 37,
            Category::Poetry => 38,
            Category::Politics => 39,
            Category::Reference => 40,
            Category::Religion => 41,
            Category::Romance => 42,
            Category::Rpg => 43,
            Category::Science => 44,
            Category::ScienceFiction => 45,
            Category::SelfHelp => 46,
            Category::SheetCollection => 47,
            Category::SheetCollectionMP3 => 48,
            Category::Sports => 49,
            Category::Technology => 50,
            Category::Thriller => 51,
            Category::Travel => 52,
            Category::UrbanFantasy => 53,
            Category::Western => 54,
            Category::YoungAdult => 55,
            Category::Superheroes => 56,
            Category::LiteraryFiction => 57,
            Category::ProgressionFantasy => 58,
            Category::DramaPlays => 60,
            Category::Unknown(id) => *id,
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
