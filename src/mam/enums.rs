use std::{collections::HashMap, fmt, marker::PhantomData};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, SeqAccess, Visitor},
};

use crate::data::{AudiobookCategory, EbookCategory};

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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Categories {
    #[serde(deserialize_with = "categories_parser")]
    pub audio: Option<Vec<AudiobookCategory>>,
    #[serde(deserialize_with = "categories_parser")]
    pub ebook: Option<Vec<EbookCategory>>,
}

impl Categories {
    pub fn get_main_cats(&self) -> Vec<u8> {
        if self.audio.as_ref().is_some_and(|c| !c.is_empty())
            || self.ebook.as_ref().is_some_and(|c| !c.is_empty())
        {
            return vec![];
        }

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
        if self.audio.as_ref().is_none_or(|c| c.is_empty())
            && self.ebook.as_ref().is_none_or(|c| c.is_empty())
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

impl std::fmt::Display for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut flags = vec![];
        if self.crude_language == Some(true) {
            flags.push("crude language");
        }
        if self.violence == Some(true) {
            flags.push("violence");
        }
        if self.some_explicit == Some(true) {
            flags.push("some explicit");
        }
        if self.explicit == Some(true) {
            flags.push("explicit");
        }
        if self.abridged == Some(true) {
            flags.push("abridged");
        }
        if self.lgbt == Some(true) {
            flags.push("lgbt");
        }
        write!(f, "{}", flags.join(", "))?;
        Ok(())
    }
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
