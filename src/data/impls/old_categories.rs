use std::str::FromStr;

use crate::data::{
    AudiobookCategory, EbookCategory, OldCategory as Category, OldMainCat as MainCat,
};

impl MainCat {
    pub fn as_id(&self) -> u8 {
        match self {
            MainCat::Audio => 13,
            MainCat::Ebook => 14,
        }
    }
}

impl FromStr for MainCat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let l = match value.to_lowercase().as_str() {
            "audio" => Some(MainCat::Audio),
            "ebook" => Some(MainCat::Ebook),
            _ => None,
        };
        match l {
            Some(l) => Ok(l),
            None => Err(format!("invalid category {value}")),
        }
    }
}

impl TryFrom<String> for MainCat {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Category {
    pub fn from_one_id(category: u64) -> Option<Category> {
        AudiobookCategory::from_id(category)
            .map(Category::Audio)
            .or_else(|| EbookCategory::from_id(category).map(Category::Ebook))
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Audio(cat) => cat.to_str(),
            Category::Ebook(cat) => cat.to_str(),
        }
    }

    pub fn as_main_cat(&self) -> MainCat {
        match self {
            Category::Audio(_) => MainCat::Audio,
            Category::Ebook(_) => MainCat::Ebook,
        }
    }

    pub fn as_id(&self) -> u8 {
        match self {
            Category::Audio(cat) => cat.to_id(),
            Category::Ebook(cat) => cat.to_id(),
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AudiobookCategory {
    pub fn all() -> Vec<AudiobookCategory> {
        vec![
            AudiobookCategory::ActionAdventure,
            AudiobookCategory::Art,
            AudiobookCategory::Biographical,
            AudiobookCategory::Business,
            AudiobookCategory::ComputerInternet,
            AudiobookCategory::Crafts,
            AudiobookCategory::CrimeThriller,
            AudiobookCategory::Fantasy,
            AudiobookCategory::Food,
            AudiobookCategory::GeneralFiction,
            AudiobookCategory::GeneralNonFic,
            AudiobookCategory::HistoricalFiction,
            AudiobookCategory::History,
            AudiobookCategory::HomeGarden,
            AudiobookCategory::Horror,
            AudiobookCategory::Humor,
            AudiobookCategory::Instructional,
            AudiobookCategory::Juvenile,
            AudiobookCategory::Language,
            AudiobookCategory::LiteraryClassics,
            AudiobookCategory::MathScienceTech,
            AudiobookCategory::Medical,
            AudiobookCategory::Mystery,
            AudiobookCategory::Nature,
            AudiobookCategory::Philosophy,
            AudiobookCategory::PolSocRelig,
            AudiobookCategory::Recreation,
            AudiobookCategory::Romance,
            AudiobookCategory::ScienceFiction,
            AudiobookCategory::SelfHelp,
            AudiobookCategory::TravelAdventure,
            AudiobookCategory::TrueCrime,
            AudiobookCategory::UrbanFantasy,
            AudiobookCategory::Western,
            AudiobookCategory::YoungAdult,
        ]
    }

    pub fn from_str(value: &str) -> Option<AudiobookCategory> {
        match value.to_lowercase().as_str() {
            "action" => Some(AudiobookCategory::ActionAdventure),
            "action/adventure" => Some(AudiobookCategory::ActionAdventure),
            "art" => Some(AudiobookCategory::Art),
            "biographical" => Some(AudiobookCategory::Biographical),
            "business" => Some(AudiobookCategory::Business),
            "computer" => Some(AudiobookCategory::ComputerInternet),
            "internet" => Some(AudiobookCategory::ComputerInternet),
            "computer/internet" => Some(AudiobookCategory::ComputerInternet),
            "crafts" => Some(AudiobookCategory::Crafts),
            "crime/thriller" => Some(AudiobookCategory::CrimeThriller),
            "fantasy" => Some(AudiobookCategory::Fantasy),
            "food" => Some(AudiobookCategory::Food),
            "general fiction" => Some(AudiobookCategory::GeneralFiction),
            "general non-fic" => Some(AudiobookCategory::GeneralNonFic),
            "general non fic" => Some(AudiobookCategory::GeneralNonFic),
            "general nonfic" => Some(AudiobookCategory::GeneralNonFic),
            "general non-fiction" => Some(AudiobookCategory::GeneralNonFic),
            "general non fiction" => Some(AudiobookCategory::GeneralNonFic),
            "general nonfiction" => Some(AudiobookCategory::GeneralNonFic),
            "historical fiction" => Some(AudiobookCategory::HistoricalFiction),
            "history" => Some(AudiobookCategory::History),
            "home" => Some(AudiobookCategory::HomeGarden),
            "garden" => Some(AudiobookCategory::HomeGarden),
            "home/garden" => Some(AudiobookCategory::HomeGarden),
            "horror" => Some(AudiobookCategory::Horror),
            "humor" => Some(AudiobookCategory::Humor),
            "instructional" => Some(AudiobookCategory::Instructional),
            "juvenile" => Some(AudiobookCategory::Juvenile),
            "language" => Some(AudiobookCategory::Language),
            "classics" => Some(AudiobookCategory::LiteraryClassics),
            "literary classics" => Some(AudiobookCategory::LiteraryClassics),
            "math" => Some(AudiobookCategory::MathScienceTech),
            "science" => Some(AudiobookCategory::MathScienceTech),
            "tech" => Some(AudiobookCategory::MathScienceTech),
            "math/science/tech" => Some(AudiobookCategory::MathScienceTech),
            "medical" => Some(AudiobookCategory::Medical),
            "mystery" => Some(AudiobookCategory::Mystery),
            "nature" => Some(AudiobookCategory::Nature),
            "philosophy" => Some(AudiobookCategory::Philosophy),
            "pol" => Some(AudiobookCategory::PolSocRelig),
            "soc" => Some(AudiobookCategory::PolSocRelig),
            "relig" => Some(AudiobookCategory::PolSocRelig),
            "pol/soc/relig" => Some(AudiobookCategory::PolSocRelig),
            "recreation" => Some(AudiobookCategory::Recreation),
            "romance" => Some(AudiobookCategory::Romance),
            "sf" => Some(AudiobookCategory::ScienceFiction),
            "science fiction" => Some(AudiobookCategory::ScienceFiction),
            "self help" => Some(AudiobookCategory::SelfHelp),
            "self-help" => Some(AudiobookCategory::SelfHelp),
            "travel" => Some(AudiobookCategory::TravelAdventure),
            "travel/adventure" => Some(AudiobookCategory::TravelAdventure),
            "true crime" => Some(AudiobookCategory::TrueCrime),
            "urban fantasy" => Some(AudiobookCategory::UrbanFantasy),
            "western" => Some(AudiobookCategory::Western),
            "ya" => Some(AudiobookCategory::YoungAdult),
            "young adult" => Some(AudiobookCategory::YoungAdult),
            _ => None,
        }
    }

    pub fn from_id(category: u64) -> Option<AudiobookCategory> {
        match category {
            39 => Some(AudiobookCategory::ActionAdventure),
            49 => Some(AudiobookCategory::Art),
            50 => Some(AudiobookCategory::Biographical),
            83 => Some(AudiobookCategory::Business),
            51 => Some(AudiobookCategory::ComputerInternet),
            97 => Some(AudiobookCategory::Crafts),
            40 => Some(AudiobookCategory::CrimeThriller),
            41 => Some(AudiobookCategory::Fantasy),
            106 => Some(AudiobookCategory::Food),
            42 => Some(AudiobookCategory::GeneralFiction),
            52 => Some(AudiobookCategory::GeneralNonFic),
            98 => Some(AudiobookCategory::HistoricalFiction),
            54 => Some(AudiobookCategory::History),
            55 => Some(AudiobookCategory::HomeGarden),
            43 => Some(AudiobookCategory::Horror),
            99 => Some(AudiobookCategory::Humor),
            84 => Some(AudiobookCategory::Instructional),
            44 => Some(AudiobookCategory::Juvenile),
            56 => Some(AudiobookCategory::Language),
            45 => Some(AudiobookCategory::LiteraryClassics),
            57 => Some(AudiobookCategory::MathScienceTech),
            85 => Some(AudiobookCategory::Medical),
            87 => Some(AudiobookCategory::Mystery),
            119 => Some(AudiobookCategory::Nature),
            88 => Some(AudiobookCategory::Philosophy),
            58 => Some(AudiobookCategory::PolSocRelig),
            59 => Some(AudiobookCategory::Recreation),
            46 => Some(AudiobookCategory::Romance),
            47 => Some(AudiobookCategory::ScienceFiction),
            53 => Some(AudiobookCategory::SelfHelp),
            89 => Some(AudiobookCategory::TravelAdventure),
            100 => Some(AudiobookCategory::TrueCrime),
            108 => Some(AudiobookCategory::UrbanFantasy),
            48 => Some(AudiobookCategory::Western),
            111 => Some(AudiobookCategory::YoungAdult),
            _ => None,
        }
    }

    pub fn to_id(self) -> u8 {
        match self {
            AudiobookCategory::ActionAdventure => 39,
            AudiobookCategory::Art => 49,
            AudiobookCategory::Biographical => 50,
            AudiobookCategory::Business => 83,
            AudiobookCategory::ComputerInternet => 51,
            AudiobookCategory::Crafts => 97,
            AudiobookCategory::CrimeThriller => 40,
            AudiobookCategory::Fantasy => 41,
            AudiobookCategory::Food => 106,
            AudiobookCategory::GeneralFiction => 42,
            AudiobookCategory::GeneralNonFic => 52,
            AudiobookCategory::HistoricalFiction => 98,
            AudiobookCategory::History => 54,
            AudiobookCategory::HomeGarden => 55,
            AudiobookCategory::Horror => 43,
            AudiobookCategory::Humor => 99,
            AudiobookCategory::Instructional => 84,
            AudiobookCategory::Juvenile => 44,
            AudiobookCategory::Language => 56,
            AudiobookCategory::LiteraryClassics => 45,
            AudiobookCategory::MathScienceTech => 57,
            AudiobookCategory::Medical => 85,
            AudiobookCategory::Mystery => 87,
            AudiobookCategory::Nature => 119,
            AudiobookCategory::Philosophy => 88,
            AudiobookCategory::PolSocRelig => 58,
            AudiobookCategory::Recreation => 59,
            AudiobookCategory::Romance => 46,
            AudiobookCategory::ScienceFiction => 47,
            AudiobookCategory::SelfHelp => 53,
            AudiobookCategory::TravelAdventure => 89,
            AudiobookCategory::TrueCrime => 100,
            AudiobookCategory::UrbanFantasy => 108,
            AudiobookCategory::Western => 48,
            AudiobookCategory::YoungAdult => 111,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            AudiobookCategory::ActionAdventure => "Action/Adventure",
            AudiobookCategory::Art => "Art",
            AudiobookCategory::Biographical => "Biographical",
            AudiobookCategory::Business => "Business",
            AudiobookCategory::ComputerInternet => "Computer/Internet",
            AudiobookCategory::Crafts => "Crafts",
            AudiobookCategory::CrimeThriller => "Crime/Thriller",
            AudiobookCategory::Fantasy => "Fantasy",
            AudiobookCategory::Food => "Food",
            AudiobookCategory::GeneralFiction => "General Fiction",
            AudiobookCategory::GeneralNonFic => "General Non-fic",
            AudiobookCategory::HistoricalFiction => "Historical Fiction",
            AudiobookCategory::History => "History",
            AudiobookCategory::HomeGarden => "Home/Garden",
            AudiobookCategory::Horror => "Horror",
            AudiobookCategory::Humor => "Humor",
            AudiobookCategory::Instructional => "Instructional",
            AudiobookCategory::Juvenile => "Juvenile",
            AudiobookCategory::Language => "Language",
            AudiobookCategory::LiteraryClassics => "Literary Classics",
            AudiobookCategory::MathScienceTech => "Math/Science/Tech",
            AudiobookCategory::Medical => "Medical",
            AudiobookCategory::Mystery => "Mystery",
            AudiobookCategory::Nature => "Nature",
            AudiobookCategory::Philosophy => "Philosophy",
            AudiobookCategory::PolSocRelig => "Pol/Soc/Relig",
            AudiobookCategory::Recreation => "Recreation",
            AudiobookCategory::Romance => "Romance",
            AudiobookCategory::ScienceFiction => "Science Fiction",
            AudiobookCategory::SelfHelp => "Self-Help",
            AudiobookCategory::TravelAdventure => "Travel/Adventure",
            AudiobookCategory::TrueCrime => "True Crime",
            AudiobookCategory::UrbanFantasy => "Urban Fantasy",
            AudiobookCategory::Western => "Western",
            AudiobookCategory::YoungAdult => "Young Adult",
        }
    }
}

impl TryFrom<String> for AudiobookCategory {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let l = Self::from_str(&value);
        match l {
            Some(l) => Ok(l),
            None => Err(format!("invalid category {value}")),
        }
    }
}

impl EbookCategory {
    pub fn all() -> Vec<EbookCategory> {
        vec![
            EbookCategory::ActionAdventure,
            EbookCategory::Art,
            EbookCategory::Biographical,
            EbookCategory::Business,
            EbookCategory::ComicsGraphicnovels,
            EbookCategory::ComputerInternet,
            EbookCategory::Crafts,
            EbookCategory::CrimeThriller,
            EbookCategory::Fantasy,
            EbookCategory::Food,
            EbookCategory::GeneralFiction,
            EbookCategory::GeneralNonFiction,
            EbookCategory::HistoricalFiction,
            EbookCategory::History,
            EbookCategory::HomeGarden,
            EbookCategory::Horror,
            EbookCategory::Humor,
            EbookCategory::IllusionMagic,
            EbookCategory::Instructional,
            EbookCategory::Juvenile,
            EbookCategory::Language,
            EbookCategory::LiteraryClassics,
            EbookCategory::MagazinesNewspapers,
            EbookCategory::MathScienceTech,
            EbookCategory::Medical,
            EbookCategory::MixedCollections,
            EbookCategory::Mystery,
            EbookCategory::Nature,
            EbookCategory::Philosophy,
            EbookCategory::PolSocRelig,
            EbookCategory::Recreation,
            EbookCategory::Romance,
            EbookCategory::ScienceFiction,
            EbookCategory::SelfHelp,
            EbookCategory::TravelAdventure,
            EbookCategory::TrueCrime,
            EbookCategory::UrbanFantasy,
            EbookCategory::Western,
            EbookCategory::YoungAdult,
        ]
    }

    pub fn from_str(value: &str) -> Option<EbookCategory> {
        match value.to_lowercase().as_str() {
            "action" => Some(EbookCategory::ActionAdventure),
            "action/adventure" => Some(EbookCategory::ActionAdventure),
            "art" => Some(EbookCategory::Art),
            "biographical" => Some(EbookCategory::Biographical),
            "business" => Some(EbookCategory::Business),
            "comics" => Some(EbookCategory::ComicsGraphicnovels),
            "graphic novels" => Some(EbookCategory::ComicsGraphicnovels),
            "comics/graphic novels" => Some(EbookCategory::ComicsGraphicnovels),
            "computer" => Some(EbookCategory::ComputerInternet),
            "internet" => Some(EbookCategory::ComputerInternet),
            "computer/internet" => Some(EbookCategory::ComputerInternet),
            "crafts" => Some(EbookCategory::Crafts),
            "crime" => Some(EbookCategory::CrimeThriller),
            "thriller" => Some(EbookCategory::CrimeThriller),
            "crime/thriller" => Some(EbookCategory::CrimeThriller),
            "fantasy" => Some(EbookCategory::Fantasy),
            "food" => Some(EbookCategory::Food),
            "general fiction" => Some(EbookCategory::GeneralFiction),
            "general non-fic" => Some(EbookCategory::GeneralNonFiction),
            "general non fic" => Some(EbookCategory::GeneralNonFiction),
            "general nonfic" => Some(EbookCategory::GeneralNonFiction),
            "general non-fiction" => Some(EbookCategory::GeneralNonFiction),
            "general non fiction" => Some(EbookCategory::GeneralNonFiction),
            "general nonfiction" => Some(EbookCategory::GeneralNonFiction),
            "historical fiction" => Some(EbookCategory::HistoricalFiction),
            "history" => Some(EbookCategory::History),
            "home" => Some(EbookCategory::HomeGarden),
            "garden" => Some(EbookCategory::HomeGarden),
            "home/garden" => Some(EbookCategory::HomeGarden),
            "horror" => Some(EbookCategory::Horror),
            "humor" => Some(EbookCategory::Humor),
            "illusion" => Some(EbookCategory::IllusionMagic),
            "magic" => Some(EbookCategory::IllusionMagic),
            "illusion/magic" => Some(EbookCategory::IllusionMagic),
            "instructional" => Some(EbookCategory::Instructional),
            "juvenile" => Some(EbookCategory::Juvenile),
            "language" => Some(EbookCategory::Language),
            "literary classics" => Some(EbookCategory::LiteraryClassics),
            "magazines" => Some(EbookCategory::MagazinesNewspapers),
            "newspapers" => Some(EbookCategory::MagazinesNewspapers),
            "magazines/newspapers" => Some(EbookCategory::MagazinesNewspapers),
            "math" => Some(EbookCategory::MathScienceTech),
            "science" => Some(EbookCategory::MathScienceTech),
            "tech" => Some(EbookCategory::MathScienceTech),
            "math/science/tech" => Some(EbookCategory::MathScienceTech),
            "medical" => Some(EbookCategory::Medical),
            "mixed collections" => Some(EbookCategory::MixedCollections),
            "mystery" => Some(EbookCategory::Mystery),
            "nature" => Some(EbookCategory::Nature),
            "philosophy" => Some(EbookCategory::Philosophy),
            "pol" => Some(EbookCategory::PolSocRelig),
            "soc" => Some(EbookCategory::PolSocRelig),
            "relig" => Some(EbookCategory::PolSocRelig),
            "pol/soc/relig" => Some(EbookCategory::PolSocRelig),
            "recreation" => Some(EbookCategory::Recreation),
            "romance" => Some(EbookCategory::Romance),
            "sf" => Some(EbookCategory::ScienceFiction),
            "science fiction" => Some(EbookCategory::ScienceFiction),
            "self help" => Some(EbookCategory::SelfHelp),
            "self-help" => Some(EbookCategory::SelfHelp),
            "travel" => Some(EbookCategory::TravelAdventure),
            "travel/adventure" => Some(EbookCategory::TravelAdventure),
            "true crime" => Some(EbookCategory::TrueCrime),
            "urban fantasy" => Some(EbookCategory::UrbanFantasy),
            "western" => Some(EbookCategory::Western),
            "ya" => Some(EbookCategory::YoungAdult),
            "young adult" => Some(EbookCategory::YoungAdult),
            _ => None,
        }
    }

    pub fn from_id(category: u64) -> Option<EbookCategory> {
        match category {
            60 => Some(EbookCategory::ActionAdventure),
            71 => Some(EbookCategory::Art),
            72 => Some(EbookCategory::Biographical),
            90 => Some(EbookCategory::Business),
            61 => Some(EbookCategory::ComicsGraphicnovels),
            73 => Some(EbookCategory::ComputerInternet),
            101 => Some(EbookCategory::Crafts),
            62 => Some(EbookCategory::CrimeThriller),
            63 => Some(EbookCategory::Fantasy),
            107 => Some(EbookCategory::Food),
            64 => Some(EbookCategory::GeneralFiction),
            74 => Some(EbookCategory::GeneralNonFiction),
            102 => Some(EbookCategory::HistoricalFiction),
            76 => Some(EbookCategory::History),
            77 => Some(EbookCategory::HomeGarden),
            65 => Some(EbookCategory::Horror),
            103 => Some(EbookCategory::Humor),
            115 => Some(EbookCategory::IllusionMagic),
            91 => Some(EbookCategory::Instructional),
            66 => Some(EbookCategory::Juvenile),
            78 => Some(EbookCategory::Language),
            67 => Some(EbookCategory::LiteraryClassics),
            79 => Some(EbookCategory::MagazinesNewspapers),
            80 => Some(EbookCategory::MathScienceTech),
            92 => Some(EbookCategory::Medical),
            118 => Some(EbookCategory::MixedCollections),
            94 => Some(EbookCategory::Mystery),
            120 => Some(EbookCategory::Nature),
            95 => Some(EbookCategory::Philosophy),
            81 => Some(EbookCategory::PolSocRelig),
            82 => Some(EbookCategory::Recreation),
            68 => Some(EbookCategory::Romance),
            69 => Some(EbookCategory::ScienceFiction),
            75 => Some(EbookCategory::SelfHelp),
            96 => Some(EbookCategory::TravelAdventure),
            104 => Some(EbookCategory::TrueCrime),
            109 => Some(EbookCategory::UrbanFantasy),
            70 => Some(EbookCategory::Western),
            112 => Some(EbookCategory::YoungAdult),
            _ => None,
        }
    }

    pub fn to_id(self) -> u8 {
        match self {
            EbookCategory::ActionAdventure => 60,
            EbookCategory::Art => 71,
            EbookCategory::Biographical => 72,
            EbookCategory::Business => 90,
            EbookCategory::ComicsGraphicnovels => 61,
            EbookCategory::ComputerInternet => 73,
            EbookCategory::Crafts => 101,
            EbookCategory::CrimeThriller => 62,
            EbookCategory::Fantasy => 63,
            EbookCategory::Food => 107,
            EbookCategory::GeneralFiction => 64,
            EbookCategory::GeneralNonFiction => 74,
            EbookCategory::HistoricalFiction => 102,
            EbookCategory::History => 76,
            EbookCategory::HomeGarden => 77,
            EbookCategory::Horror => 65,
            EbookCategory::Humor => 103,
            EbookCategory::IllusionMagic => 115,
            EbookCategory::Instructional => 91,
            EbookCategory::Juvenile => 66,
            EbookCategory::Language => 78,
            EbookCategory::LiteraryClassics => 67,
            EbookCategory::MagazinesNewspapers => 79,
            EbookCategory::MathScienceTech => 80,
            EbookCategory::Medical => 92,
            EbookCategory::MixedCollections => 118,
            EbookCategory::Mystery => 94,
            EbookCategory::Nature => 120,
            EbookCategory::Philosophy => 95,
            EbookCategory::PolSocRelig => 81,
            EbookCategory::Recreation => 82,
            EbookCategory::Romance => 68,
            EbookCategory::ScienceFiction => 69,
            EbookCategory::SelfHelp => 75,
            EbookCategory::TravelAdventure => 96,
            EbookCategory::TrueCrime => 104,
            EbookCategory::UrbanFantasy => 109,
            EbookCategory::Western => 70,
            EbookCategory::YoungAdult => 112,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            EbookCategory::ActionAdventure => "Action/Adventure",
            EbookCategory::Art => "Art",
            EbookCategory::Biographical => "Biographical",
            EbookCategory::Business => "Business",
            EbookCategory::ComicsGraphicnovels => "Comics/Graphic Novels",
            EbookCategory::ComputerInternet => "Computer/Internet",
            EbookCategory::Crafts => "Crafts",
            EbookCategory::CrimeThriller => "Crime/Thriller",
            EbookCategory::Fantasy => "Fantasy",
            EbookCategory::Food => "Food",
            EbookCategory::GeneralFiction => "General Fiction",
            EbookCategory::GeneralNonFiction => "General Non-fic",
            EbookCategory::HistoricalFiction => "Historical Fiction",
            EbookCategory::History => "History",
            EbookCategory::HomeGarden => "Home/Garden",
            EbookCategory::Horror => "Horror",
            EbookCategory::Humor => "Humor",
            EbookCategory::IllusionMagic => "Illusion/Magic",
            EbookCategory::Instructional => "Instructional",
            EbookCategory::Juvenile => "Juvenile",
            EbookCategory::Language => "Language",
            EbookCategory::LiteraryClassics => "Literary Classics",
            EbookCategory::MagazinesNewspapers => "Magazines/Newspapers",
            EbookCategory::MathScienceTech => "Math/Science/Tech",
            EbookCategory::Medical => "Medical",
            EbookCategory::MixedCollections => "Mixed Collections",
            EbookCategory::Mystery => "Mystery",
            EbookCategory::Nature => "Nature",
            EbookCategory::Philosophy => "Philosophy",
            EbookCategory::PolSocRelig => "Pol/Soc/Relig",
            EbookCategory::Recreation => "Recreation",
            EbookCategory::Romance => "Romance",
            EbookCategory::ScienceFiction => "Science Fiction",
            EbookCategory::SelfHelp => "Self-Help",
            EbookCategory::TravelAdventure => "Travel/Adventure",
            EbookCategory::TrueCrime => "True Crime",
            EbookCategory::UrbanFantasy => "Urban Fantasy",
            EbookCategory::Western => "Western",
            EbookCategory::YoungAdult => "Young Adult",
        }
    }
}

impl TryFrom<String> for EbookCategory {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let l = Self::from_str(&value);
        match l {
            Some(l) => Ok(l),
            None => Err(format!("invalid category {value}")),
        }
    }
}
