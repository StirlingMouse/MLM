use std::{fmt, marker::PhantomData};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, SeqAccess, Visitor},
};

use mlm_db::{AudiobookCategory, EbookCategory, MusicologyCategory, RadioCategory};

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SearchTarget {
    Bookmarks,
    New,
    Mine,
    AllReseed,
    MyReseed,
    Uploader(u64),
}

impl Serialize for SearchTarget {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            SearchTarget::Bookmarks => serializer.serialize_str("bookmarks"),
            SearchTarget::New => serializer.serialize_str("torrents"),
            SearchTarget::Mine => serializer.serialize_str("mine"),
            SearchTarget::AllReseed => serializer.serialize_str("allRseed"),
            SearchTarget::MyReseed => serializer.serialize_str("myReseed"),
            SearchTarget::Uploader(id) => serializer.serialize_str(&format!("u{id}")),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SearchKind {
    /// Last update had 1+ seeders
    #[serde(rename = "active")]
    Active,
    /// Last update has 0 seeders
    #[serde(rename = "inactive")]
    Inactive,
    /// Freeleech torrents
    #[serde(rename = "fl")]
    Freeleech,
    /// Freeleech or VIP torrents
    #[serde(rename = "fl-VIP")]
    Free,
    /// VIP torrents
    #[serde(rename = "VIP")]
    Vip,
    /// Torrents not VIP
    #[serde(rename = "nVIP")]
    NotVip,
    /// Torrents missing meta data (old torrents)
    #[serde(rename = "nMeta")]
    NoMeta,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Categories {
    #[serde(default = "default_categories_field")]
    #[serde(deserialize_with = "categories_parser")]
    pub audio: Option<Vec<AudiobookCategory>>,
    #[serde(default = "default_categories_field")]
    #[serde(deserialize_with = "categories_parser")]
    pub ebook: Option<Vec<EbookCategory>>,
    #[serde(default = "default_categories_field")]
    #[serde(deserialize_with = "categories_parser")]
    pub musicology: Option<Vec<MusicologyCategory>>,
    #[serde(default = "default_categories_field")]
    #[serde(deserialize_with = "categories_parser")]
    pub radio: Option<Vec<RadioCategory>>,
}

impl Categories {
    pub fn get_main_cats(&self) -> Vec<u8> {
        if self.audio.as_ref().is_some_and(|c| !c.is_empty())
            || self.ebook.as_ref().is_some_and(|c| !c.is_empty())
            || self.musicology.as_ref().is_some_and(|c| !c.is_empty())
            || self.radio.as_ref().is_some_and(|c| !c.is_empty())
        {
            return vec![];
        }

        let main_cats = [
            self.audio
                .as_ref()
                .is_none_or(|c| !c.is_empty())
                .then_some(13),
            self.ebook
                .as_ref()
                .is_none_or(|c| !c.is_empty())
                .then_some(14),
            self.musicology
                .as_ref()
                .is_none_or(|c| !c.is_empty())
                .then_some(15),
            self.radio
                .as_ref()
                .is_none_or(|c| !c.is_empty())
                .then_some(16),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if main_cats.len() == 4 {
            return vec![];
        }

        main_cats
    }

    pub fn get_cats(&self) -> Vec<u8> {
        if self.audio.as_ref().is_none_or(|c| c.is_empty())
            && self.ebook.as_ref().is_none_or(|c| c.is_empty())
            && self.musicology.as_ref().is_none_or(|c| c.is_empty())
            && self.radio.as_ref().is_none_or(|c| c.is_empty())
        {
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
            .chain(
                self.musicology
                    .clone()
                    .unwrap_or_else(MusicologyCategory::all)
                    .iter()
                    .map(|c| c.to_id()),
            )
            .chain(
                self.radio
                    .clone()
                    .unwrap_or_else(RadioCategory::all)
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
        } else if let Some(cat) = MusicologyCategory::from_id(category) {
            self.musicology
                .as_ref()
                .is_none_or(|cats| cats.contains(&cat))
        } else if let Some(cat) = RadioCategory::from_id(category) {
            self.radio.as_ref().is_none_or(|cats| cats.contains(&cat))
        } else {
            false
        }
    }
}

impl Default for Categories {
    fn default() -> Self {
        Self {
            audio: None,
            ebook: None,
            musicology: Some(vec![]),
            radio: Some(vec![]),
        }
    }
}

fn categories_parser<'de, T, D>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
where
    T: Deserialize<'de>,
    T: TryFrom<String, Error = String>,
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
        T: TryFrom<String, Error = String>,
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

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut values = vec![];
            loop {
                let elm: Option<String> = seq.next_element()?;
                let Some(elm) = elm else {
                    return Ok(Some(values));
                };
                values.push(T::try_from(elm).map_err(serde::de::Error::custom)?);
            }
        }
    }

    deserializer.deserialize_any(CategoriesParser(PhantomData))
}

fn default_categories_field<T>() -> Option<Vec<T>> {
    Some(vec![])
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(try_from = "String")]
#[allow(unused)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categories() {
        let mut categories = Categories::default();
        assert_eq!(categories.get_main_cats(), vec![13, 14]);
        assert_eq!(categories.get_cats(), Vec::<u8>::new());

        categories.audio = Some(vec![]);
        assert_eq!(categories.get_main_cats(), vec![14]);
        assert_eq!(categories.get_cats(), Vec::<u8>::new());

        categories.audio = None;
        categories.ebook = Some(vec![]);
        assert_eq!(categories.get_main_cats(), vec![13]);
        assert_eq!(categories.get_cats(), Vec::<u8>::new());

        categories.audio = None;
        categories.ebook = Some(vec![EbookCategory::Food]);
        assert_eq!(categories.get_main_cats(), Vec::<u8>::new());
        assert_eq!(
            categories.get_cats(),
            [
                AudiobookCategory::all()
                    .into_iter()
                    .map(AudiobookCategory::to_id)
                    .collect::<Vec<u8>>(),
                vec![EbookCategory::Food.to_id()]
            ]
            .concat()
        );

        categories.audio = Some(vec![AudiobookCategory::Food]);
        categories.ebook = Some(vec![EbookCategory::Food]);
        assert_eq!(categories.get_main_cats(), Vec::<u8>::new());
        assert_eq!(
            categories.get_cats(),
            vec![AudiobookCategory::Food.to_id(), EbookCategory::Food.to_id()]
        );

        categories.audio = Some(vec![]);
        categories.ebook = Some(vec![EbookCategory::Food]);
        assert_eq!(categories.get_main_cats(), Vec::<u8>::new());
        assert_eq!(categories.get_cats(), vec![EbookCategory::Food.to_id()]);
    }
}
