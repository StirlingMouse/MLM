use std::str::FromStr;

use itertools::Itertools as _;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer};
use time::Date;

use crate::{
    data::{
        Category, Language, ListItem, MainCat, Size, Torrent, TorrentCost, TorrentMeta,
        TorrentMetaDiff, TorrentMetaField, TorrentStatus,
    },
    mam::DATE_FORMAT,
    mam_enums::Flags,
};

use super::{AudiobookCategory, EbookCategory, FlagBits};

pub fn parse<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr<Err = String>,
    D: Deserializer<'de>,
{
    let v: String = Deserialize::deserialize(deserializer)?;
    v.parse().map_err(serde::de::Error::custom)
}

pub fn parse_opt<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: FromStr<Err = String>,
    D: Deserializer<'de>,
{
    let v: Option<String> = Deserialize::deserialize(deserializer)?;
    v.map(|v| v.parse().map_err(serde::de::Error::custom))
        .transpose()
}

pub fn parse_vec<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: FromStr<Err = String>,
    D: Deserializer<'de>,
{
    let v: Vec<String> = Deserialize::deserialize(deserializer)?;
    v.into_iter()
        .map(|v| v.parse().map_err(serde::de::Error::custom))
        .collect()
}

pub fn parse_opt_date<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<String> = Deserialize::deserialize(deserializer)?;
    v.map(|v| Date::parse(&v, &DATE_FORMAT).map_err(serde::de::Error::custom))
        .transpose()
}

impl Torrent {
    pub fn matches(&self, other: &Torrent) -> bool {
        // if self.hash == other.hash { return true };
        if self.title_search != other.title_search {
            return false;
        };
        self.meta.matches(&other.meta)
    }
}

impl TorrentMeta {
    pub(crate) fn matches(&self, other: &TorrentMeta) -> bool {
        self.main_cat == other.main_cat
            && self.authors.iter().any(|a| other.authors.contains(a))
            && ((self.narrators.is_empty() && other.narrators.is_empty())
                || self.narrators.iter().any(|a| other.narrators.contains(a)))
    }

    pub(crate) fn diff(&self, other: &TorrentMeta) -> Vec<TorrentMetaDiff> {
        let mut diff = vec![];
        if self.mam_id != other.mam_id {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::MamId,
                from: self.mam_id.to_string(),
                to: other.mam_id.to_string(),
            });
        }
        if self.main_cat != other.main_cat {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::MainCat,
                from: self.main_cat.as_str().to_string(),
                to: other.main_cat.as_str().to_string(),
            });
        }
        if self.cat != other.cat {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Cat,
                from: self
                    .cat
                    .as_ref()
                    .map(|cat| cat.as_str().to_string())
                    .unwrap_or_default(),
                to: other
                    .cat
                    .as_ref()
                    .map(|cat| cat.as_str().to_string())
                    .unwrap_or_default(),
            });
        }
        if self.language != other.language {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Language,
                from: self
                    .language
                    .map(|language| language.to_str().to_string())
                    .unwrap_or_default(),
                to: other
                    .language
                    .map(|language| language.to_str().to_string())
                    .unwrap_or_default(),
            });
        }
        if self.flags != other.flags {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Flags,
                from: self
                    .flags
                    .map(|flags| format!("{}", Flags::from(flags)))
                    .unwrap_or_default(),
                to: other
                    .flags
                    .map(|flags| format!("{}", Flags::from(flags)))
                    .unwrap_or_default(),
            });
        }
        if self.filetypes != other.filetypes {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Filetypes,
                from: self.filetypes.join(", ").to_string(),
                to: other.filetypes.join(", ").to_string(),
            });
        }
        if self.size != other.size {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Size,
                from: self.size.to_string(),
                to: other.size.to_string(),
            });
        }
        if self.title != other.title {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Title,
                from: self.title.to_string(),
                to: other.title.to_string(),
            });
        }
        if self.authors != other.authors {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Authors,
                from: self.authors.join(", ").to_string(),
                to: other.authors.join(", ").to_string(),
            });
        }
        if self.narrators != other.narrators {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Narrators,
                from: self.narrators.join(", ").to_string(),
                to: other.narrators.join(", ").to_string(),
            });
        }
        if self.series != other.series {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Series,
                from: self.series.iter().map(format_serie).join(", ").to_string(),
                to: other.series.iter().map(format_serie).join(", ").to_string(),
            });
        }
        diff
    }
}

impl std::fmt::Display for TorrentMetaField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TorrentMetaField::MamId => write!(f, "mam_id"),
            TorrentMetaField::MainCat => write!(f, "main_cat"),
            TorrentMetaField::Cat => write!(f, "cat"),
            TorrentMetaField::Language => write!(f, "language"),
            TorrentMetaField::Flags => write!(f, "flags"),
            TorrentMetaField::Filetypes => write!(f, "filetypes"),
            TorrentMetaField::Size => write!(f, "size"),
            TorrentMetaField::Title => write!(f, "title"),
            TorrentMetaField::Authors => write!(f, "authors"),
            TorrentMetaField::Narrators => write!(f, "narrators"),
            TorrentMetaField::Series => write!(f, "series"),
        }
    }
}

impl ListItem {
    pub fn want_audio(&self) -> bool {
        let have_audio = self
            .audio_torrent
            .as_ref()
            .is_some_and(|t| t.status != TorrentStatus::Wanted);
        let have_ebook = self
            .ebook_torrent
            .as_ref()
            .is_some_and(|t| t.status != TorrentStatus::Wanted);

        self.allow_audio
            && !have_audio
            && self
                .prefer_format
                .is_none_or(|f| f == MainCat::Audio || !have_ebook)
    }

    pub fn want_ebook(&self) -> bool {
        let have_audio = self
            .audio_torrent
            .as_ref()
            .is_some_and(|t| t.status != TorrentStatus::Wanted);
        let have_ebook = self
            .ebook_torrent
            .as_ref()
            .is_some_and(|t| t.status != TorrentStatus::Wanted);

        self.allow_ebook
            && !have_ebook
            && self
                .prefer_format
                .is_none_or(|f| f == MainCat::Ebook || !have_audio)
    }
}

impl Size {
    pub fn unit(self) -> u64 {
        if self.bytes() > 0 { 1 } else { 0 }
    }
}

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value = self.bytes() as f64;
        let mut unit = "B";
        if value > 1024_f64.powf(3.0) {
            value /= 1024_f64.powf(3.0);
            unit = "GiB";
        } else if value > 1024_f64.powf(2.0) {
            value /= 1024_f64.powf(2.0);
            unit = "MiB";
        } else if value > 1024.0 {
            value /= 1024.0;
            unit = "KiB";
        }
        let value = ((value * 1000.0).round() as u64) / 1000;
        write!(f, "{} {}", value, unit)
    }
}

pub static SIZE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^((?:\d{1,3},)?\d{1,6}(?:\.\d{1,3})?) ([kKMG]?)(i)?B$").unwrap());

impl FromStr for Size {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Some((Some(value), Some(unit), i)) = SIZE_PATTERN
            .captures(value)
            .map(|c| (c.get(1), c.get(2), c.get(3)))
        {
            let value: f64 = value.as_str().replace(",", "").parse().unwrap();
            let base: u64 = if i.is_some() { 1024 } else { 1000 };
            let multiplier = match unit.as_str() {
                "" => 1,
                "k" | "K" => base,
                "M" => base.pow(2),
                "G" => base.pow(3),
                _ => unreachable!("unknown unit: {}", unit.as_str()),
            } as f64;
            Ok(Size::from_bytes((value * multiplier).round() as u64))
        } else {
            Err(format!("invalid size value {value}"))
        }
    }
}

impl TryFrom<String> for Size {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl MainCat {
    pub(crate) fn from_id(main_cat: u64) -> Result<MainCat, String> {
        match main_cat {
            13 => Ok(MainCat::Audio),
            14 => Ok(MainCat::Ebook),
            15 => Err("Unsupported main_cat Musicology".to_string()),
            16 => Err("Unsupported main_cat Radio".to_string()),
            id => Err(format!("Unknown main_cat {id}")),
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        match self {
            MainCat::Audio => "Audiobook",
            MainCat::Ebook => "Ebook",
        }
    }

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
    pub fn from_id(main_cat: MainCat, category: u64) -> Option<Category> {
        match main_cat {
            MainCat::Audio => AudiobookCategory::from_id(category).map(Category::Audio),
            MainCat::Ebook => EbookCategory::from_id(category).map(Category::Ebook),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Audio(cat) => cat.to_str(),
            Category::Ebook(cat) => cat.to_str(),
        }
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

impl TorrentCost {
    pub fn as_str(&self) -> &'static str {
        match self {
            TorrentCost::GlobalFreeleech => "free",
            TorrentCost::PersonalFreeleech => "PF",
            TorrentCost::Vip => "VIP",
            TorrentCost::UseWedge => "wedge",
            TorrentCost::TryWedge => "try wedge",
            TorrentCost::Ratio => "ratio",
        }
    }
}

impl Language {
    pub fn from_id(id: u8) -> Option<Language> {
        match id {
            1 => Some(Language::English),
            17 => Some(Language::Afrikaans),
            32 => Some(Language::Arabic),
            35 => Some(Language::Bengali),
            51 => Some(Language::Bosnian),
            18 => Some(Language::Bulgarian),
            6 => Some(Language::Burmese),
            44 => Some(Language::Cantonese),
            19 => Some(Language::Catalan),
            2 => Some(Language::Chinese),
            49 => Some(Language::Croatian),
            20 => Some(Language::Czech),
            21 => Some(Language::Danish),
            22 => Some(Language::Dutch),
            61 => Some(Language::Estonian),
            39 => Some(Language::Farsi),
            23 => Some(Language::Finnish),
            36 => Some(Language::French),
            37 => Some(Language::German),
            26 => Some(Language::Greek),
            59 => Some(Language::GreekAncient),
            3 => Some(Language::Gujarati),
            27 => Some(Language::Hebrew),
            8 => Some(Language::Hindi),
            28 => Some(Language::Hungarian),
            63 => Some(Language::Icelandic),
            53 => Some(Language::Indonesian),
            56 => Some(Language::Irish),
            43 => Some(Language::Italian),
            38 => Some(Language::Japanese),
            12 => Some(Language::Javanese),
            5 => Some(Language::Kannada),
            41 => Some(Language::Korean),
            50 => Some(Language::Lithuanian),
            46 => Some(Language::Latin),
            62 => Some(Language::Latvian),
            33 => Some(Language::Malay),
            58 => Some(Language::Malayalam),
            57 => Some(Language::Manx),
            9 => Some(Language::Marathi),
            48 => Some(Language::Norwegian),
            45 => Some(Language::Polish),
            34 => Some(Language::Portuguese),
            52 => Some(Language::BrazilianPortuguese),
            14 => Some(Language::Punjabi),
            30 => Some(Language::Romanian),
            16 => Some(Language::Russian),
            24 => Some(Language::ScottishGaelic),
            60 => Some(Language::Sanskrit),
            31 => Some(Language::Serbian),
            54 => Some(Language::Slovenian),
            4 => Some(Language::Spanish),
            55 => Some(Language::CastilianSpanish),
            40 => Some(Language::Swedish),
            29 => Some(Language::Tagalog),
            11 => Some(Language::Tamil),
            10 => Some(Language::Telugu),
            7 => Some(Language::Thai),
            42 => Some(Language::Turkish),
            25 => Some(Language::Ukrainian),
            15 => Some(Language::Urdu),
            13 => Some(Language::Vietnamese),
            47 => Some(Language::Other),
            _ => None,
        }
    }

    pub fn to_id(self) -> u8 {
        match self {
            Language::English => 1,
            Language::Afrikaans => 17,
            Language::Arabic => 32,
            Language::Bengali => 35,
            Language::Bosnian => 51,
            Language::Bulgarian => 18,
            Language::Burmese => 6,
            Language::Cantonese => 44,
            Language::Catalan => 19,
            Language::Chinese => 2,
            Language::Croatian => 49,
            Language::Czech => 20,
            Language::Danish => 21,
            Language::Dutch => 22,
            Language::Estonian => 61,
            Language::Farsi => 39,
            Language::Finnish => 23,
            Language::French => 36,
            Language::German => 37,
            Language::Greek => 26,
            Language::GreekAncient => 59,
            Language::Gujarati => 3,
            Language::Hebrew => 27,
            Language::Hindi => 8,
            Language::Hungarian => 28,
            Language::Icelandic => 63,
            Language::Indonesian => 53,
            Language::Irish => 56,
            Language::Italian => 43,
            Language::Japanese => 38,
            Language::Javanese => 12,
            Language::Kannada => 5,
            Language::Korean => 41,
            Language::Lithuanian => 50,
            Language::Latin => 46,
            Language::Latvian => 62,
            Language::Malay => 33,
            Language::Malayalam => 58,
            Language::Manx => 57,
            Language::Marathi => 9,
            Language::Norwegian => 48,
            Language::Polish => 45,
            Language::Portuguese => 34,
            Language::BrazilianPortuguese => 52,
            Language::Punjabi => 14,
            Language::Romanian => 30,
            Language::Russian => 16,
            Language::ScottishGaelic => 24,
            Language::Sanskrit => 60,
            Language::Serbian => 31,
            Language::Slovenian => 54,
            Language::Spanish => 4,
            Language::CastilianSpanish => 55,
            Language::Swedish => 40,
            Language::Tagalog => 29,
            Language::Tamil => 11,
            Language::Telugu => 10,
            Language::Thai => 7,
            Language::Turkish => 42,
            Language::Ukrainian => 25,
            Language::Urdu => 15,
            Language::Vietnamese => 13,
            Language::Other => 47,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            Language::English => "english",
            Language::Afrikaans => "afrikaans",
            Language::Arabic => "arabic",
            Language::Bengali => "bengali",
            Language::Bosnian => "bosnian",
            Language::Bulgarian => "bulgarian",
            Language::Burmese => "burmese",
            Language::Cantonese => "cantonese",
            Language::Catalan => "catalan",
            Language::Chinese => "chinese",
            Language::Croatian => "croatian",
            Language::Czech => "czech",
            Language::Danish => "danish",
            Language::Dutch => "dutch",
            Language::Estonian => "estonian",
            Language::Farsi => "farsi",
            Language::Finnish => "finnish",
            Language::French => "french",
            Language::German => "german",
            Language::Greek => "greek",
            Language::GreekAncient => "ancient greek",
            Language::Gujarati => "gujarati",
            Language::Hebrew => "hebrew",
            Language::Hindi => "hindi",
            Language::Hungarian => "hungarian",
            Language::Icelandic => "icelandic",
            Language::Indonesian => "indonesian",
            Language::Irish => "irish",
            Language::Italian => "italian",
            Language::Japanese => "japanese",
            Language::Javanese => "javanese",
            Language::Kannada => "kannada",
            Language::Korean => "korean",
            Language::Lithuanian => "lithuanian",
            Language::Latin => "latin",
            Language::Latvian => "latvian",
            Language::Malay => "malay",
            Language::Malayalam => "malayalam",
            Language::Manx => "manx",
            Language::Marathi => "marathi",
            Language::Norwegian => "norwegian",
            Language::Polish => "polish",
            Language::Portuguese => "portuguese",
            Language::BrazilianPortuguese => "brazilian",
            Language::Punjabi => "punjabi",
            Language::Romanian => "romanian",
            Language::Russian => "russian",
            Language::ScottishGaelic => "scottish",
            Language::Sanskrit => "sanskrit",
            Language::Serbian => "serbian",
            Language::Slovenian => "slovenian",
            Language::Spanish => "spanish",
            Language::CastilianSpanish => "castilian",
            Language::Swedish => "swedish",
            Language::Tagalog => "tagalog",
            Language::Tamil => "tamil",
            Language::Telugu => "telugu",
            Language::Thai => "thai",
            Language::Turkish => "turkish",
            Language::Ukrainian => "ukrainian",
            Language::Urdu => "urdu",
            Language::Vietnamese => "vietnamese",
            Language::Other => "other",
        }
    }
}

impl FromStr for Language {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let l = match value.to_lowercase().as_str() {
            "english" => Some(Language::English),
            "afrikaans" => Some(Language::Afrikaans),
            "arabic" => Some(Language::Arabic),
            "bengali" => Some(Language::Bengali),
            "bosnian" => Some(Language::Bosnian),
            "bulgarian" => Some(Language::Bulgarian),
            "burmese" => Some(Language::Burmese),
            "cantonese" => Some(Language::Cantonese),
            "catalan" => Some(Language::Catalan),
            "chinese" => Some(Language::Chinese),
            "croatian" => Some(Language::Croatian),
            "czech" => Some(Language::Czech),
            "danish" => Some(Language::Danish),
            "dutch" => Some(Language::Dutch),
            "estonian" => Some(Language::Estonian),
            "farsi" => Some(Language::Farsi),
            "finnish" => Some(Language::Finnish),
            "french" => Some(Language::French),
            "german" => Some(Language::German),
            "greek" => Some(Language::Greek),
            "ancient greek" => Some(Language::GreekAncient),
            "greek ancient" => Some(Language::GreekAncient),
            "greek, ancient" => Some(Language::GreekAncient),
            "gujarati" => Some(Language::Gujarati),
            "hebrew" => Some(Language::Hebrew),
            "hindi" => Some(Language::Hindi),
            "hungarian" => Some(Language::Hungarian),
            "icelandic" => Some(Language::Icelandic),
            "indonesian" => Some(Language::Indonesian),
            "irish" => Some(Language::Irish),
            "italian" => Some(Language::Italian),
            "japanese" => Some(Language::Japanese),
            "javanese" => Some(Language::Javanese),
            "kannada" => Some(Language::Kannada),
            "korean" => Some(Language::Korean),
            "lithuanian" => Some(Language::Lithuanian),
            "latin" => Some(Language::Latin),
            "latvian" => Some(Language::Latvian),
            "malay" => Some(Language::Malay),
            "malayalam" => Some(Language::Malayalam),
            "manx" => Some(Language::Manx),
            "marathi" => Some(Language::Marathi),
            "norwegian" => Some(Language::Norwegian),
            "polish" => Some(Language::Polish),
            "portuguese" => Some(Language::Portuguese),
            "bp" => Some(Language::BrazilianPortuguese),
            "brazilian" => Some(Language::BrazilianPortuguese),
            "brazilian portuguese" => Some(Language::BrazilianPortuguese),
            "brazilian portuguese (bp)" => Some(Language::BrazilianPortuguese),
            "punjabi" => Some(Language::Punjabi),
            "romanian" => Some(Language::Romanian),
            "russian" => Some(Language::Russian),
            "scottish" => Some(Language::ScottishGaelic),
            "scottish gaelic" => Some(Language::ScottishGaelic),
            "gaelic" => Some(Language::ScottishGaelic),
            "sanskrit" => Some(Language::Sanskrit),
            "serbian" => Some(Language::Serbian),
            "slovenian" => Some(Language::Slovenian),
            "spanish" => Some(Language::Spanish),
            "castilian" => Some(Language::CastilianSpanish),
            "castilian spanish" => Some(Language::CastilianSpanish),
            "swedish" => Some(Language::Swedish),
            "tagalog" => Some(Language::Tagalog),
            "tamil" => Some(Language::Tamil),
            "telugu" => Some(Language::Telugu),
            "thai" => Some(Language::Thai),
            "turkish" => Some(Language::Turkish),
            "ukrainian" => Some(Language::Ukrainian),
            "urdu" => Some(Language::Urdu),
            "vietnamese" => Some(Language::Vietnamese),
            "other" => Some(Language::Other),
            _ => None,
        };
        match l {
            Some(l) => Ok(l),
            None => Err(format!("invalid language {value}")),
        }
    }
}

impl TryFrom<String> for Language {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<Flags> for FlagBits {
    fn from(value: Flags) -> Self {
        FlagBits::new(value.as_bitfield())
    }
}

impl From<FlagBits> for Flags {
    fn from(value: FlagBits) -> Self {
        Flags::from_bitfield(value.0)
    }
}

pub fn format_serie(series: &(String, String)) -> String {
    let (name, num) = series;
    if num.is_empty() {
        name.clone()
    } else {
        format!("{name} #{num}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_thousands_divider() {
        assert_eq!(
            Size::from_str("1,016.2 KiB"),
            Ok(Size::from_bytes(1_040_589))
        );
    }
}
