use std::{collections::HashMap, fmt, marker::PhantomData};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, SeqAccess, Visitor},
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchIn {
    Author,
    Description,
    Filenames,
    FileTypes,
    Narrator,
    Series,
    Tags,
    Title,
}

impl SearchIn {
    pub fn as_str(&self) -> &str {
        match self {
            SearchIn::Author => "author",
            SearchIn::Description => "description",
            SearchIn::Filenames => "filenames",
            SearchIn::FileTypes => "fileTypes",
            SearchIn::Narrator => "narrator",
            SearchIn::Series => "series",
            SearchIn::Tags => "tags",
            SearchIn::Title => "title",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Categories {
    #[serde(deserialize_with = "categories_parser")]
    audio: Option<Vec<AudiobookCategory>>,
    #[serde(deserialize_with = "categories_parser")]
    ebook: Option<Vec<EbookCategory>>,
}

impl Categories {
    pub fn get_main_cats(&self) -> Vec<u8> {
        [
            self.audio
                .as_ref()
                .is_none_or(|c| !c.is_empty())
                .then_some(13),
            self.ebook
                .as_ref()
                .is_none_or(|c| !c.is_empty())
                .then_some(14),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    pub fn get_cats(&self) -> Vec<u8> {
        if self.audio.is_none() && self.ebook.is_none() {
            return vec![];
        }

        self.audio
            .clone()
            .unwrap_or_else(AudiobookCategory::all)
            .iter()
            .map(|c| c.to_id())
            .chain(
                self.ebook
                    .clone()
                    .unwrap_or_else(EbookCategory::all)
                    .iter()
                    .map(|c| c.to_id()),
            )
            .collect()
    }

    pub fn matches(&self, category: u64) -> bool {
        if let Some(cat) = AudiobookCategory::from_id(category) {
            self.audio.as_ref().is_none_or(|cats| cats.contains(&cat))
        } else if let Some(cat) = EbookCategory::from_id(category) {
            self.ebook.as_ref().is_none_or(|cats| cats.contains(&cat))
        } else {
            false
        }
    }
}

fn categories_parser<'de, T, D>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct CategoriesParser<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for CategoriesParser<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Option<Vec<T>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("bool or array")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(if value { None } else { Some(vec![]) })
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            Ok(Some(Deserialize::deserialize(
                de::value::SeqAccessDeserializer::new(seq),
            )?))
        }
    }

    deserializer.deserialize_any(CategoriesParser(PhantomData))
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "String")]
pub enum AudiobookCategory {
    ActionAdventure,
    Art,
    Biographical,
    Business,
    ComputerInternet,
    Crafts,
    CrimeThriller,
    Fantasy,
    Food,
    GeneralFiction,
    GeneralNonFic,
    HistoricalFiction,
    History,
    HomeGarden,
    Horror,
    Humor,
    Instructional,
    Juvenile,
    Language,
    LiteraryClassics,
    MathScienceTech,
    Medical,
    Mystery,
    Nature,
    Philosophy,
    PolSocRelig,
    Recreation,
    Romance,
    ScienceFiction,
    SelfHelp,
    TravelAdventure,
    TrueCrime,
    UrbanFantasy,
    Western,
    YoungAdult,
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

    fn from_id(category: u64) -> Option<AudiobookCategory> {
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "String")]
pub enum EbookCategory {
    ActionAdventure,
    Art,
    Biographical,
    Business,
    ComicsGraphicnovels,
    ComputerInternet,
    Crafts,
    CrimeThriller,
    Fantasy,
    Food,
    GeneralFiction,
    GeneralNonFiction,
    HistoricalFiction,
    History,
    HomeGarden,
    Horror,
    Humor,
    IllusionMagic,
    Instructional,
    Juvenile,
    Language,
    LiteraryClassics,
    MagazinesNewspapers,
    MathScienceTech,
    Medical,
    MixedCollections,
    Mystery,
    Nature,
    Philosophy,
    PolSocRelig,
    Recreation,
    Romance,
    ScienceFiction,
    SelfHelp,
    TravelAdventure,
    TrueCrime,
    UrbanFantasy,
    Western,
    YoungAdult,
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

    fn from_id(category: u64) -> Option<EbookCategory> {
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

#[derive(Clone, Default, Debug, Deserialize)]
#[serde(try_from = "HashMap<String, bool>")]
pub struct Flags {
    pub crude_language: Option<bool>,
    pub violence: Option<bool>,
    pub some_explicit: Option<bool>,
    pub explicit: Option<bool>,
    pub abridged: Option<bool>,
    pub lgbt: Option<bool>,
}

impl Flags {
    pub fn from_bitfield(field: u8) -> Flags {
        Flags {
            crude_language: Some(field & (1 << 1) > 0),
            violence: Some(field & (1 << 2) > 0),
            some_explicit: Some(field & (1 << 3) > 0),
            explicit: Some(field & (1 << 4) > 0),
            abridged: Some(field & (1 << 5) > 0),
            lgbt: Some(field & (1 << 6) > 0),
        }
    }

    pub fn as_search_bitfield(&self) -> (bool, Vec<u8>) {
        let shows = self.fields().filter(|f| f.unwrap_or(false)).count();
        let hides = self.fields().filter(|f| f.is_some_and(|f| !f)).count();
        let is_hide = hides > shows;
        let mut field = vec![];
        if self.crude_language.is_some_and(|f| f != is_hide) {
            field.push(1 << 1);
        }
        if self.violence.is_some_and(|f| f != is_hide) {
            field.push(1 << 2);
        }
        if self.some_explicit.is_some_and(|f| f != is_hide) {
            field.push(1 << 3);
        }
        if self.explicit.is_some_and(|f| f != is_hide) {
            field.push(1 << 4);
        }
        if self.abridged.is_some_and(|f| f != is_hide) {
            field.push(1 << 5);
        }
        if self.lgbt.is_some_and(|f| f != is_hide) {
            field.push(1 << 6);
        }
        (is_hide, field)
    }

    pub fn as_bitfield(&self) -> u8 {
        let mut field = 0;
        if self.crude_language.unwrap_or_default() {
            field += 1 << 1;
        }
        if self.violence.unwrap_or_default() {
            field += 1 << 2;
        }
        if self.some_explicit.unwrap_or_default() {
            field += 1 << 3;
        }
        if self.explicit.unwrap_or_default() {
            field += 1 << 4;
        }
        if self.abridged.unwrap_or_default() {
            field += 1 << 5;
        }
        if self.lgbt.unwrap_or_default() {
            field += 1 << 6;
        }
        field
    }

    pub fn matches(&self, other: &Flags) -> bool {
        self.fields()
            .zip(other.fields())
            .all(|(t, o)| t.is_none_or(|_| t == o))
    }

    fn fields(&self) -> impl Iterator<Item = Option<bool>> {
        [
            self.crude_language,
            self.violence,
            self.some_explicit,
            self.explicit,
            self.abridged,
            self.lgbt,
        ]
        .into_iter()
    }
}

impl TryFrom<HashMap<String, bool>> for Flags {
    type Error = String;

    fn try_from(value: HashMap<String, bool>) -> Result<Self, Self::Error> {
        let mut flags = Flags::default();
        for (key, value) in value.into_iter() {
            match key.to_lowercase().as_str() {
                "crude" => flags.crude_language = Some(value),
                "language" => flags.crude_language = Some(value),
                "crude language" => flags.crude_language = Some(value),
                "violence" => flags.violence = Some(value),
                "some explicit" => flags.some_explicit = Some(value),
                "explicit" => flags.explicit = Some(value),
                "abridged" => flags.abridged = Some(value),
                "lgbt" => flags.lgbt = Some(value),
                _ => return Err(format!("invalid flag {key}")),
            }
        }
        Ok(flags)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub enum UserClass {
    Dev,
    SysOp,
    SrAdministrator,
    Administrator,
    UploaderCoordinator,
    SrModerator,
    Moderator,
    TorrentMod,
    ForumMod,
    SupportStaff,
    EntryLevelStaff,
    Uploader,
    Mouseketeer,
    Supporter,
    Elite,
    EliteVip,
    Vip,
    PowerUser,
    User,
    Mouse,
}

impl UserClass {
    pub fn from_str(class: &str) -> Option<UserClass> {
        match class {
            "Dev" => Some(UserClass::Dev),
            "SysOp" => Some(UserClass::SysOp),
            "SR Administrator" => Some(UserClass::SrAdministrator),
            "Administrator" => Some(UserClass::Administrator),
            "Uploader Coordinator" => Some(UserClass::UploaderCoordinator),
            "SR Moderator" => Some(UserClass::SrModerator),
            "Moderator" => Some(UserClass::Moderator),
            "Torrent Mod" => Some(UserClass::TorrentMod),
            "Forum Mod" => Some(UserClass::ForumMod),
            "Support Staff" => Some(UserClass::SupportStaff),
            "Entry Level Staff" => Some(UserClass::EntryLevelStaff),
            "Uploader" => Some(UserClass::Uploader),
            "Mouseketeer" => Some(UserClass::Mouseketeer),
            "Supporter" => Some(UserClass::Supporter),
            "Elite" => Some(UserClass::Elite),
            "Elite VIP" => Some(UserClass::EliteVip),
            "VIP" => Some(UserClass::Vip),
            "Power User" => Some(UserClass::PowerUser),
            "User" => Some(UserClass::User),
            "Mous" => Some(UserClass::Mouse),
            _ => None,
        }
    }

    pub fn unsats(&self) -> u64 {
        match self {
            UserClass::Dev
            | UserClass::SysOp
            | UserClass::SrAdministrator
            | UserClass::Administrator
            | UserClass::UploaderCoordinator
            | UserClass::SrModerator
            | UserClass::Moderator
            | UserClass::TorrentMod
            | UserClass::ForumMod
            | UserClass::SupportStaff
            | UserClass::EntryLevelStaff
            | UserClass::Uploader
            | UserClass::Mouseketeer
            | UserClass::Supporter
            | UserClass::Elite
            | UserClass::EliteVip => 200,
            UserClass::Vip => 150,
            UserClass::PowerUser => 100,
            UserClass::User => 20,
            UserClass::Mouse => 0,
        }
    }
}

impl TryFrom<String> for UserClass {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let v = Self::from_str(&value);
        match v {
            Some(v) => Ok(v),
            None => Err(format!("invalid category {value}")),
        }
    }
}
