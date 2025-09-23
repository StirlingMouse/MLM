use anyhow::Result;
use native_db::transaction::RwTransaction;
use native_db::{Database, Key, ToInput, db_type};
use native_db::{Models, ToKey, native_db};
use native_model::{Model, native_model};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::OffsetDateTime;
use tracing::{info, instrument};

pub static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<v1::Config>().unwrap();

    models.define::<v7::SelectedTorrent>().unwrap();
    models.define::<v7::DuplicateTorrent>().unwrap();

    models.define::<v6::Torrent>().unwrap();
    models.define::<v6::SelectedTorrent>().unwrap();
    models.define::<v6::DuplicateTorrent>().unwrap();
    models.define::<v6::ErroredTorrent>().unwrap();

    models.define::<v5::Torrent>().unwrap();
    models.define::<v5::List>().unwrap();
    models.define::<v5::ListItem>().unwrap();

    models.define::<v4::SelectedTorrent>().unwrap();
    models.define::<v4::Event>().unwrap();
    models.define::<v4::List>().unwrap();
    models.define::<v4::ListItem>().unwrap();

    models.define::<v3::Torrent>().unwrap();
    models.define::<v3::SelectedTorrent>().unwrap();
    models.define::<v3::DuplicateTorrent>().unwrap();
    models.define::<v3::ErroredTorrent>().unwrap();
    models.define::<v3::Event>().unwrap();
    models.define::<v3::List>().unwrap();
    models.define::<v3::ListItem>().unwrap();

    models.define::<v2::Torrent>().unwrap();
    models.define::<v2::SelectedTorrent>().unwrap();
    models.define::<v2::DuplicateTorrent>().unwrap();
    models.define::<v2::ErroredTorrent>().unwrap();

    models.define::<v1::Torrent>().unwrap();
    models.define::<v1::SelectedTorrent>().unwrap();
    models.define::<v1::DuplicateTorrent>().unwrap();
    models.define::<v1::ErroredTorrent>().unwrap();

    models
});

pub type Config = v1::Config;
pub type Torrent = v6::Torrent;
pub type TorrentKey = v6::TorrentKey;
pub type SelectedTorrent = v7::SelectedTorrent;
pub type SelectedTorrentKey = v7::SelectedTorrentKey;
pub type DuplicateTorrent = v7::DuplicateTorrent;
pub type ErroredTorrent = v6::ErroredTorrent;
pub type ErroredTorrentKey = v6::ErroredTorrentKey;
pub type ErroredTorrentId = v1::ErroredTorrentId;
pub type Event = v4::Event;
pub type EventKey = v4::EventKey;
pub type EventType = v4::EventType;
pub type List = v5::List;
pub type ListKey = v5::ListKey;
pub type ListItem = v5::ListItem;
pub type ListItemKey = v5::ListItemKey;
pub type ListItemTorrent = v4::ListItemTorrent;
pub type TorrentMeta = v6::TorrentMeta;
pub type MainCat = v1::MainCat;
pub type Uuid = v3::Uuid;
pub type Timestamp = v3::Timestamp;
pub type Language = v3::Language;
pub type Size = v3::Size;
pub type TorrentCost = v4::TorrentCost;
pub type TorrentStatus = v4::TorrentStatus;
pub type LibraryMismatch = v5::LibraryMismatch;
pub type AudiobookCategory = v6::AudiobookCategory;
pub type EbookCategory = v6::EbookCategory;
pub type Category = v6::Category;

#[instrument(skip_all)]
pub fn migrate(db: &Database<'_>) -> Result<()> {
    let rw = db.rw_transaction()?;

    rw.migrate::<Torrent>()?;
    rw.migrate::<SelectedTorrent>()?;
    rw.migrate::<DuplicateTorrent>()?;
    // recover_migrate::<v2::ErroredTorrent, v3::ErroredTorrent>(&rw)?;
    rw.migrate::<ErroredTorrent>()?;
    recover_migrate::<v3::Event, Event>(&rw)?;
    rw.migrate::<Event>()?;
    rw.migrate::<List>()?;
    rw.migrate::<ListItem>()?;
    rw.commit()?;
    info!("Migrations done");

    Ok(())
}

fn recover_migrate<Old, New>(rw: &RwTransaction<'_>) -> Result<(), db_type::Error>
where
    Old: From<New> + Clone + ToInput,
    New: From<Old> + ToInput,
{
    let old_data = rw
        .scan()
        .primary::<Old>()?
        .all()?
        .collect::<Result<Vec<_>, _>>()?;

    for old in old_data {
        let new: New = old.clone().into();
        rw.insert(new).or_else(|err| match err {
            db_type::Error::DuplicateKey { .. } => Ok(()),
            err => Err(err),
        })?;
        rw.remove(old)?;
    }

    Ok(())
}

pub mod v1 {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    #[native_model(id = 1, version = 1)]
    #[native_db]
    pub struct Config {
        #[primary_key]
        pub key: String,
        pub value: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 2, version = 1)]
    #[native_db]
    pub struct Torrent {
        #[primary_key]
        pub hash: String,
        pub library_path: Option<PathBuf>,
        pub library_files: Vec<PathBuf>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        pub replaced_with: Option<String>,
        pub request_matadata_update: bool,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 3, version = 1)]
    #[native_db]
    pub struct SelectedTorrent {
        #[primary_key]
        pub mam_id: u64,
        pub dl_link: String,
        pub unsat_buffer: Option<u64>,
        pub category: Option<String>,
        pub tags: Vec<String>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 4, version = 1)]
    #[native_db]
    pub struct DuplicateTorrent {
        #[primary_key]
        pub mam_id: u64,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        pub duplicate_of: Option<String>,
        pub request_replace: bool,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 5, version = 1)]
    #[native_db]
    pub struct ErroredTorrent {
        #[primary_key]
        pub id: ErroredTorrentId,
        pub title: String,
        pub error: String,
        pub meta: Option<TorrentMeta>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct TorrentMeta {
        pub mam_id: u64,
        pub main_cat: MainCat,
        pub filetypes: Vec<String>,
        pub title: String,
        pub authors: Vec<String>,
        pub narrators: Vec<String>,
        pub series: Vec<(String, String)>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum MainCat {
        Audio,
        Ebook,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum ErroredTorrentId {
        Grabber(/* mam_id */ u64),
        Linker(/* hash */ String),
        Cleaner(/* hash */ String),
    }

    impl ToKey for ErroredTorrentId {
        fn to_key(&self) -> Key {
            match self {
                ErroredTorrentId::Grabber(mam_id) => {
                    Key::new([&[0u8] as &[u8], &mam_id.to_le_bytes()].concat())
                }
                ErroredTorrentId::Linker(hash) => {
                    Key::new([&[0u8] as &[u8], hash.as_bytes()].concat())
                }
                ErroredTorrentId::Cleaner(hash) => {
                    Key::new([&[0u8] as &[u8], hash.as_bytes()].concat())
                }
            }
        }

        fn key_names() -> Vec<String> {
            vec!["ErroredTorrentHash".to_string()]
        }
    }

    impl From<v2::Torrent> for Torrent {
        fn from(t: v2::Torrent) -> Self {
            Self {
                hash: t.hash,
                library_path: t.library_path,
                library_files: t.library_files,
                title_search: t.title_search,
                meta: t.meta,
                replaced_with: t.replaced_with.map(|(r, _)| r),
                request_matadata_update: t.request_matadata_update,
            }
        }
    }

    impl From<v2::SelectedTorrent> for SelectedTorrent {
        fn from(t: v2::SelectedTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                dl_link: t.dl_link,
                unsat_buffer: t.unsat_buffer,
                category: t.category,
                tags: t.tags,
                title_search: t.title_search,
                meta: t.meta,
            }
        }
    }

    impl From<v2::DuplicateTorrent> for DuplicateTorrent {
        fn from(t: v2::DuplicateTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                title_search: t.title_search,
                meta: t.meta,
                duplicate_of: t.duplicate_of,
                request_replace: t.request_replace,
            }
        }
    }

    impl From<v2::ErroredTorrent> for ErroredTorrent {
        fn from(t: v2::ErroredTorrent) -> Self {
            Self {
                id: t.id,
                title: t.title,
                error: t.error,
                meta: t.meta,
            }
        }
    }
}

pub mod v2 {
    use time::UtcOffset;

    use super::*;
    use v1::TorrentMeta;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 2, version = 2, from = v1::Torrent)]
    #[native_db]
    pub struct Torrent {
        #[primary_key]
        pub hash: String,
        pub library_path: Option<PathBuf>,
        pub library_files: Vec<PathBuf>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        pub created_at: OffsetDateTime,
        pub replaced_with: Option<(String, OffsetDateTime)>,
        pub request_matadata_update: bool,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 3, version = 2, from = v1::SelectedTorrent)]
    #[native_db]
    pub struct SelectedTorrent {
        #[primary_key]
        pub mam_id: u64,
        pub dl_link: String,
        pub unsat_buffer: Option<u64>,
        pub category: Option<String>,
        pub tags: Vec<String>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        pub created_at: OffsetDateTime,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 4, version = 2, from = v1::DuplicateTorrent)]
    #[native_db]
    pub struct DuplicateTorrent {
        #[primary_key]
        pub mam_id: u64,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        pub created_at: OffsetDateTime,
        pub duplicate_of: Option<String>,
        pub request_replace: bool,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 5, version = 2, from = v1::ErroredTorrent)]
    #[native_db]
    pub struct ErroredTorrent {
        #[primary_key]
        pub id: ErroredTorrentId,
        pub title: String,
        pub error: String,
        pub meta: Option<TorrentMeta>,
        pub created_at: OffsetDateTime,
    }

    impl From<v1::Torrent> for Torrent {
        fn from(t: v1::Torrent) -> Self {
            Self {
                hash: t.hash,
                library_path: t.library_path,
                library_files: t.library_files,
                title_search: t.title_search,
                meta: t.meta,
                created_at: OffsetDateTime::now_utc(),
                replaced_with: t.replaced_with.map(|r| (r, OffsetDateTime::now_utc())),
                request_matadata_update: t.request_matadata_update,
            }
        }
    }

    impl From<v1::SelectedTorrent> for SelectedTorrent {
        fn from(t: v1::SelectedTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                dl_link: t.dl_link,
                unsat_buffer: t.unsat_buffer,
                category: t.category,
                tags: t.tags,
                title_search: t.title_search,
                meta: t.meta,
                created_at: OffsetDateTime::now_utc(),
            }
        }
    }

    impl From<v1::DuplicateTorrent> for DuplicateTorrent {
        fn from(t: v1::DuplicateTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                title_search: t.title_search,
                meta: t.meta,
                duplicate_of: t.duplicate_of,
                request_replace: t.request_replace,
                created_at: OffsetDateTime::now_utc(),
            }
        }
    }

    impl From<v1::ErroredTorrent> for ErroredTorrent {
        fn from(t: v1::ErroredTorrent) -> Self {
            Self {
                id: t.id,
                title: t.title,
                error: t.error,
                meta: t.meta,
                created_at: OffsetDateTime::now_utc(),
            }
        }
    }

    impl From<v3::Torrent> for Torrent {
        fn from(t: v3::Torrent) -> Self {
            Self {
                hash: t.hash,
                library_path: t.library_path,
                library_files: t.library_files,
                title_search: t.title_search,
                meta: t.meta.into(),
                created_at: t.created_at.0.to_offset(UtcOffset::UTC),
                replaced_with: t
                    .replaced_with
                    .map(|(with, when)| (with, when.0.to_offset(UtcOffset::UTC))),
                request_matadata_update: t.request_matadata_update,
            }
        }
    }

    impl From<v3::SelectedTorrent> for SelectedTorrent {
        fn from(t: v3::SelectedTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                dl_link: t.dl_link,
                unsat_buffer: t.unsat_buffer,
                category: t.category,
                tags: t.tags,
                title_search: t.title_search,
                meta: t.meta.into(),
                created_at: t.created_at.0.to_offset(UtcOffset::UTC),
            }
        }
    }

    impl From<v3::DuplicateTorrent> for DuplicateTorrent {
        fn from(t: v3::DuplicateTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                title_search: t.title_search,
                meta: t.meta.into(),
                duplicate_of: t.duplicate_of,
                request_replace: t.request_replace,
                created_at: t.created_at.0.to_offset(UtcOffset::UTC),
            }
        }
    }

    impl From<v3::ErroredTorrent> for ErroredTorrent {
        fn from(t: v3::ErroredTorrent) -> Self {
            Self {
                id: t.id,
                title: t.title,
                error: t.error,
                meta: t.meta.map(Into::into),
                created_at: t.created_at.0.to_offset(UtcOffset::UTC),
            }
        }
    }

    impl From<v3::TorrentMeta> for TorrentMeta {
        fn from(t: v3::TorrentMeta) -> Self {
            Self {
                mam_id: t.mam_id,
                main_cat: t.main_cat,
                filetypes: t.filetypes,
                title: t.title,
                authors: t.authors,
                narrators: t.narrators,
                series: t.series,
            }
        }
    }
}

pub mod v3 {
    use time::UtcDateTime;

    use super::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 2, version = 3, from = v2::Torrent)]
    #[native_db]
    pub struct Torrent {
        #[primary_key]
        pub hash: String,
        pub library_path: Option<PathBuf>,
        pub library_files: Vec<PathBuf>,
        pub selected_audio_format: Option<String>,
        pub selected_ebook_format: Option<String>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        #[secondary_key]
        pub created_at: Timestamp,
        pub replaced_with: Option<(String, Timestamp)>,
        pub request_matadata_update: bool,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 3, version = 3, from = v2::SelectedTorrent)]
    #[native_db]
    pub struct SelectedTorrent {
        #[primary_key]
        pub mam_id: u64,
        pub dl_link: String,
        pub unsat_buffer: Option<u64>,
        pub category: Option<String>,
        pub tags: Vec<String>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        pub created_at: Timestamp,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 4, version = 3, from = v2::DuplicateTorrent)]
    #[native_db]
    pub struct DuplicateTorrent {
        #[primary_key]
        pub mam_id: u64,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        pub created_at: Timestamp,
        pub duplicate_of: Option<String>,
        pub request_replace: bool,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 5, version = 3, from = v2::ErroredTorrent)]
    #[native_db]
    pub struct ErroredTorrent {
        #[primary_key]
        pub id: ErroredTorrentId,
        pub title: String,
        pub error: String,
        pub meta: Option<TorrentMeta>,
        #[secondary_key]
        pub created_at: Timestamp,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 6, version = 3)]
    #[native_db]
    pub struct Event {
        #[primary_key]
        pub id: Uuid,
        #[secondary_key]
        pub hash: Option<String>,
        #[secondary_key]
        pub mam_id: Option<u64>,
        #[secondary_key]
        pub created_at: Timestamp,
        pub event: EventType,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 7, version = 3)]
    #[native_db]
    pub struct List {
        #[primary_key]
        pub id: u64,
        #[secondary_key]
        pub title: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 8, version = 3)]
    #[native_db]
    pub struct ListItem {
        #[primary_key]
        pub guid: (u64, String),
        #[secondary_key]
        pub list_id: u64,
        pub title: String,
        pub authors: Vec<String>,
        pub series: Vec<(String, u64)>,
        pub cover_url: String,
        pub book_url: Option<String>,
        pub isbn: Option<u64>,
        pub prefer_format: Option<MainCat>,
        pub audio_torrent: Option<(u64, Timestamp)>,
        pub wanted_audio_torrent: Option<(u64, Timestamp)>,
        pub ebook_torrent: Option<(u64, Timestamp)>,
        pub wanted_ebook_torrent: Option<(u64, Timestamp)>,
        #[secondary_key]
        pub created_at: Timestamp,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum EventType {
        Grabbed,
        Linked {
            library_path: PathBuf,
        },
        Cleaned {
            library_path: PathBuf,
            files: Vec<PathBuf>,
        },
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct TorrentMeta {
        pub mam_id: u64,
        pub main_cat: MainCat,
        pub language: Option<Language>,
        pub filetypes: Vec<String>,
        pub size: Size,
        pub title: String,
        pub authors: Vec<String>,
        pub narrators: Vec<String>,
        pub series: Vec<(String, String)>,
    }

    #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Language {
        English,
        Afrikaans,
        Arabic,
        Bengali,
        Bosnian,
        Bulgarian,
        Burmese,
        Cantonese,
        Catalan,
        Chinese,
        Croatian,
        Czech,
        Danish,
        Dutch,
        Estonian,
        Farsi,
        Finnish,
        French,
        German,
        Greek,
        GreekAncient,
        Gujarati,
        Hebrew,
        Hindi,
        Hungarian,
        Icelandic,
        Indonesian,
        Irish,
        Italian,
        Japanese,
        Javanese,
        Kannada,
        Korean,
        Lithuanian,
        Latin,
        Latvian,
        Malay,
        Malayalam,
        Manx,
        Marathi,
        Norwegian,
        Polish,
        Portuguese,
        BrazilianPortuguese,
        Punjabi,
        Romanian,
        Russian,
        ScottishGaelic,
        Sanskrit,
        Serbian,
        Slovenian,
        Spanish,
        CastilianSpanish,
        Swedish,
        Tagalog,
        Tamil,
        Telugu,
        Thai,
        Turkish,
        Ukrainian,
        Urdu,
        Vietnamese,
        Other,
    }

    #[derive(
        Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord,
    )]
    pub struct Size(u64);
    impl Size {
        pub fn from_bytes(bytes: u64) -> Size {
            Size(bytes)
        }

        pub fn bytes(self) -> u64 {
            self.0
        }
    }

    #[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Hash)]
    pub struct Timestamp(pub UtcDateTime);
    impl Timestamp {
        pub fn now() -> Self {
            Self(UtcDateTime::now())
        }
    }

    impl From<UtcDateTime> for Timestamp {
        fn from(value: UtcDateTime) -> Self {
            Self(value)
        }
    }
    impl From<OffsetDateTime> for Timestamp {
        fn from(value: OffsetDateTime) -> Self {
            Self(value.to_utc())
        }
    }

    impl ToKey for Timestamp {
        fn to_key(&self) -> Key {
            Key::new(self.0.unix_timestamp().to_be_bytes().into())
        }

        fn key_names() -> Vec<String> {
            vec!["Timestamp".to_string()]
        }
    }

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone, Hash)]
    pub struct Uuid(uuid::Uuid);
    impl Uuid {
        pub fn new() -> Self {
            Self(uuid::Uuid::new_v4())
        }
    }

    impl ToKey for Uuid {
        fn to_key(&self) -> Key {
            Key::new(self.0.as_bytes().to_vec())
        }

        fn key_names() -> Vec<String> {
            vec!["Uuid".to_string()]
        }
    }

    impl From<v2::Torrent> for Torrent {
        fn from(t: v2::Torrent) -> Self {
            Self {
                hash: t.hash,
                library_path: t.library_path,
                library_files: t.library_files,
                selected_audio_format: None,
                selected_ebook_format: None,
                title_search: t.title_search,
                meta: t.meta.into(),
                created_at: t.created_at.into(),
                replaced_with: t.replaced_with.map(|(with, when)| (with, when.into())),
                request_matadata_update: t.request_matadata_update,
            }
        }
    }

    impl From<v2::SelectedTorrent> for SelectedTorrent {
        fn from(t: v2::SelectedTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                dl_link: t.dl_link,
                unsat_buffer: t.unsat_buffer,
                category: t.category,
                tags: t.tags,
                title_search: t.title_search,
                meta: t.meta.into(),
                created_at: t.created_at.into(),
            }
        }
    }

    impl From<v2::DuplicateTorrent> for DuplicateTorrent {
        fn from(t: v2::DuplicateTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                title_search: t.title_search,
                meta: t.meta.into(),
                duplicate_of: t.duplicate_of,
                request_replace: t.request_replace,
                created_at: t.created_at.into(),
            }
        }
    }

    impl From<v2::ErroredTorrent> for ErroredTorrent {
        fn from(t: v2::ErroredTorrent) -> Self {
            Self {
                id: t.id,
                title: t.title,
                error: t.error,
                meta: t.meta.map(Into::into),
                created_at: t.created_at.into(),
            }
        }
    }

    impl From<v1::TorrentMeta> for TorrentMeta {
        fn from(t: v1::TorrentMeta) -> Self {
            Self {
                mam_id: t.mam_id,
                main_cat: t.main_cat,
                language: None,
                filetypes: t.filetypes,
                size: Size(0),
                title: t.title,
                authors: t.authors,
                narrators: t.narrators,
                series: t.series,
            }
        }
    }

    impl From<v5::Torrent> for Torrent {
        fn from(t: v5::Torrent) -> Self {
            Self {
                hash: t.hash,
                library_path: t.library_path,
                library_files: t.library_files,
                selected_audio_format: t.selected_audio_format,
                selected_ebook_format: t.selected_ebook_format,
                title_search: t.title_search,
                meta: t.meta,
                created_at: t.created_at,
                replaced_with: t.replaced_with,
                request_matadata_update: t.request_matadata_update,
            }
        }
    }

    impl From<v4::SelectedTorrent> for SelectedTorrent {
        fn from(t: v4::SelectedTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                dl_link: t.dl_link,
                unsat_buffer: t.unsat_buffer,
                category: t.category,
                tags: t.tags,
                title_search: t.title_search,
                meta: t.meta,
                created_at: t.created_at,
            }
        }
    }

    impl From<v4::Event> for Event {
        fn from(t: v4::Event) -> Self {
            Self {
                id: t.id,
                hash: t.hash,
                mam_id: t.mam_id,
                created_at: t.created_at,
                event: t.event.into(),
            }
        }
    }

    impl From<v4::List> for List {
        fn from(t: v4::List) -> Self {
            Self {
                id: t.id.split(':').next().unwrap().parse().unwrap(),
                title: t.title,
            }
        }
    }

    impl From<v4::ListItem> for ListItem {
        fn from(t: v4::ListItem) -> Self {
            let list_id = t.list_id.split(':').next().unwrap().parse().unwrap();

            Self {
                guid: (list_id, t.guid.1),
                list_id,
                title: t.title,
                authors: t.authors,
                series: t.series,
                cover_url: t.cover_url,
                book_url: t.book_url,
                isbn: t.isbn,
                prefer_format: t.prefer_format,
                audio_torrent: t.audio_torrent.as_ref().and_then(|t| {
                    if t.status == TorrentStatus::Selected {
                        Some((t.mam_id, t.at))
                    } else {
                        None
                    }
                }),
                wanted_audio_torrent: t.audio_torrent.as_ref().and_then(|t| {
                    if t.status == TorrentStatus::Wanted {
                        Some((t.mam_id, t.at))
                    } else {
                        None
                    }
                }),
                ebook_torrent: t.ebook_torrent.as_ref().and_then(|t| {
                    if t.status == TorrentStatus::Selected {
                        Some((t.mam_id, t.at))
                    } else {
                        None
                    }
                }),
                wanted_ebook_torrent: t.ebook_torrent.as_ref().and_then(|t| {
                    if t.status == TorrentStatus::Wanted {
                        Some((t.mam_id, t.at))
                    } else {
                        None
                    }
                }),
                created_at: t.created_at,
            }
        }
    }

    impl From<v4::EventType> for EventType {
        fn from(t: v4::EventType) -> Self {
            match t {
                v4::EventType::Grabbed { .. } => Self::Grabbed,
                v4::EventType::Linked { library_path } => Self::Linked { library_path },
                v4::EventType::Cleaned {
                    library_path,
                    files,
                } => Self::Cleaned {
                    library_path,
                    files,
                },
            }
        }
    }

    impl From<v6::DuplicateTorrent> for DuplicateTorrent {
        fn from(t: v6::DuplicateTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                title_search: t.title_search,
                meta: t.meta.into(),
                created_at: t.created_at,
                duplicate_of: t.duplicate_of,
                request_replace: t.request_replace,
            }
        }
    }

    impl From<v6::ErroredTorrent> for ErroredTorrent {
        fn from(t: v6::ErroredTorrent) -> Self {
            Self {
                id: t.id,
                title: t.title,
                error: t.error,
                meta: t.meta.map(|t| t.into()),
                created_at: t.created_at,
            }
        }
    }

    impl From<v6::TorrentMeta> for TorrentMeta {
        fn from(t: v6::TorrentMeta) -> Self {
            Self {
                mam_id: t.mam_id,
                main_cat: t.main_cat,
                language: t.language,
                filetypes: t.filetypes,
                size: t.size,
                title: t.title,
                authors: t.authors,
                narrators: t.narrators,
                series: t.series,
            }
        }
    }
}

pub mod v4 {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 3, version = 4, from = v3::SelectedTorrent)]
    #[native_db]
    pub struct SelectedTorrent {
        #[primary_key]
        pub mam_id: u64,
        pub dl_link: String,
        pub unsat_buffer: Option<u64>,
        pub cost: TorrentCost,
        pub category: Option<String>,
        pub tags: Vec<String>,
        #[secondary_key]
        pub title_search: String,
        pub meta: v3::TorrentMeta,
        pub created_at: Timestamp,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 6, version = 4, from = v3::Event)]
    #[native_db]
    pub struct Event {
        #[primary_key]
        pub id: Uuid,
        #[secondary_key]
        pub hash: Option<String>,
        #[secondary_key]
        pub mam_id: Option<u64>,
        #[secondary_key]
        pub created_at: Timestamp,
        pub event: EventType,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 7, version = 4, from = v3::List)]
    #[native_db]
    pub struct List {
        #[primary_key]
        pub id: String,
        #[secondary_key]
        pub title: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 8, version = 4, from = v3::ListItem)]
    #[native_db]
    pub struct ListItem {
        #[primary_key]
        pub guid: (String, String),
        #[secondary_key]
        pub list_id: String,
        pub title: String,
        pub authors: Vec<String>,
        pub series: Vec<(String, u64)>,
        pub cover_url: String,
        pub book_url: Option<String>,
        pub isbn: Option<u64>,
        pub prefer_format: Option<MainCat>,
        pub allow_audio: bool,
        pub audio_torrent: Option<ListItemTorrent>,
        pub allow_ebook: bool,
        pub ebook_torrent: Option<ListItemTorrent>,
        #[secondary_key]
        pub created_at: Timestamp,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum EventType {
        Grabbed {
            cost: Option<TorrentCost>,
            wedged: bool,
        },
        Linked {
            library_path: PathBuf,
        },
        Cleaned {
            library_path: PathBuf,
            files: Vec<PathBuf>,
        },
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum TorrentCost {
        GlobalFreeleech,
        PersonalFreeleech,
        Vip,
        UseWedge,
        TryWedge,
        Ratio,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ListItemTorrent {
        pub mam_id: u64,
        pub status: TorrentStatus,
        pub at: Timestamp,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub enum TorrentStatus {
        Selected,
        Wanted,
        NotWanted,
        Existing,
    }

    impl From<v3::SelectedTorrent> for SelectedTorrent {
        fn from(t: v3::SelectedTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                dl_link: t.dl_link,
                unsat_buffer: t.unsat_buffer,
                cost: TorrentCost::Ratio,
                category: t.category,
                tags: t.tags,
                title_search: t.title_search,
                meta: t.meta,
                created_at: t.created_at,
            }
        }
    }

    impl From<v3::Event> for Event {
        fn from(t: v3::Event) -> Self {
            Self {
                id: t.id,
                hash: t.hash,
                mam_id: t.mam_id,
                created_at: t.created_at,
                event: t.event.into(),
            }
        }
    }

    impl From<v3::List> for List {
        fn from(t: v3::List) -> Self {
            Self {
                id: format!("{}:to-read", t.id),
                title: t.title,
            }
        }
    }

    impl From<v3::ListItem> for ListItem {
        fn from(t: v3::ListItem) -> Self {
            Self {
                guid: (format!("{}:to-read", t.list_id), t.guid.1),
                list_id: format!("{}:to-read", t.list_id),
                title: t.title,
                authors: t.authors,
                series: t.series,
                cover_url: t.cover_url,
                book_url: t.book_url,
                isbn: t.isbn,
                prefer_format: t.prefer_format,
                allow_audio: true,
                audio_torrent: t
                    .audio_torrent
                    .map(|t| ListItemTorrent {
                        mam_id: t.0,
                        status: TorrentStatus::Selected,
                        at: t.1,
                    })
                    .or_else(|| {
                        t.wanted_audio_torrent.map(|t| ListItemTorrent {
                            mam_id: t.0,
                            status: TorrentStatus::Wanted,
                            at: t.1,
                        })
                    }),
                allow_ebook: true,
                ebook_torrent: t
                    .ebook_torrent
                    .map(|t| ListItemTorrent {
                        mam_id: t.0,
                        status: TorrentStatus::Selected,
                        at: t.1,
                    })
                    .or_else(|| {
                        t.wanted_ebook_torrent.map(|t| ListItemTorrent {
                            mam_id: t.0,
                            status: TorrentStatus::Wanted,
                            at: t.1,
                        })
                    }),
                created_at: t.created_at,
            }
        }
    }

    impl From<v3::EventType> for EventType {
        fn from(t: v3::EventType) -> Self {
            match t {
                v3::EventType::Grabbed => Self::Grabbed {
                    cost: None,
                    wedged: false,
                },
                v3::EventType::Linked { library_path } => Self::Linked { library_path },
                v3::EventType::Cleaned {
                    library_path,
                    files,
                } => Self::Cleaned {
                    library_path,
                    files,
                },
            }
        }
    }

    impl From<v5::List> for List {
        fn from(t: v5::List) -> Self {
            Self {
                id: t.id,
                title: t.title,
            }
        }
    }

    impl From<v5::ListItem> for ListItem {
        fn from(t: v5::ListItem) -> Self {
            Self {
                guid: t.guid,
                list_id: t.list_id,
                title: t.title,
                authors: t.authors,
                series: t
                    .series
                    .into_iter()
                    .map(|(name, num)| (name, num as u64))
                    .collect(),
                cover_url: t.cover_url,
                book_url: t.book_url,
                isbn: t.isbn,
                prefer_format: t.prefer_format,
                allow_audio: t.allow_audio,
                audio_torrent: t.audio_torrent,
                allow_ebook: t.allow_ebook,
                ebook_torrent: t.ebook_torrent,
                created_at: t.created_at,
            }
        }
    }

    impl From<v6::SelectedTorrent> for SelectedTorrent {
        fn from(t: v6::SelectedTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                dl_link: t.dl_link,
                unsat_buffer: t.unsat_buffer,
                cost: t.cost,
                category: t.category,
                tags: t.tags,
                title_search: t.title_search,
                meta: t.meta.into(),
                created_at: t.created_at,
            }
        }
    }
}

pub mod v5 {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 2, version = 5, from = v3::Torrent)]
    #[native_db]
    pub struct Torrent {
        #[primary_key]
        pub hash: String,
        pub library_path: Option<PathBuf>,
        pub library_files: Vec<PathBuf>,
        pub selected_audio_format: Option<String>,
        pub selected_ebook_format: Option<String>,
        #[secondary_key]
        pub title_search: String,
        pub meta: v3::TorrentMeta,
        #[secondary_key]
        pub created_at: Timestamp,
        pub replaced_with: Option<(String, Timestamp)>,
        pub request_matadata_update: bool,
        pub library_mismatch: Option<LibraryMismatch>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub enum LibraryMismatch {
        NewPath(PathBuf),
        NoLibrary,
        TorrentRemoved,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 7, version = 5, from = v4::List)]
    #[native_db]
    pub struct List {
        #[primary_key]
        pub id: String,
        #[secondary_key]
        pub title: String,
        pub updated_at: Option<Timestamp>,
        pub build_date: Option<Timestamp>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 8, version = 5, from = v4::ListItem)]
    #[native_db]
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
        pub prefer_format: Option<MainCat>,
        pub allow_audio: bool,
        pub audio_torrent: Option<ListItemTorrent>,
        pub allow_ebook: bool,
        pub ebook_torrent: Option<ListItemTorrent>,
        #[secondary_key]
        pub created_at: Timestamp,
        pub marked_done_at: Option<Timestamp>,
    }

    impl From<v3::Torrent> for Torrent {
        fn from(t: v3::Torrent) -> Self {
            Self {
                hash: t.hash,
                library_path: t.library_path,
                library_files: t.library_files,
                selected_audio_format: t.selected_audio_format,
                selected_ebook_format: t.selected_ebook_format,
                title_search: t.title_search,
                meta: t.meta,
                created_at: t.created_at,
                replaced_with: t.replaced_with,
                request_matadata_update: t.request_matadata_update,
                library_mismatch: None,
            }
        }
    }

    impl From<v4::List> for List {
        fn from(t: v4::List) -> Self {
            Self {
                id: t.id,
                title: t.title,
                updated_at: None,
                build_date: None,
            }
        }
    }

    impl From<v4::ListItem> for ListItem {
        fn from(t: v4::ListItem) -> Self {
            Self {
                guid: t.guid,
                list_id: t.list_id,
                title: t.title,
                authors: t.authors,
                series: t
                    .series
                    .into_iter()
                    .map(|(name, num)| (name, num as f64))
                    .collect(),
                cover_url: t.cover_url,
                book_url: t.book_url,
                isbn: t.isbn,
                prefer_format: t.prefer_format,
                allow_audio: t.allow_audio,
                audio_torrent: t.audio_torrent,
                allow_ebook: t.allow_ebook,
                ebook_torrent: t.ebook_torrent,
                created_at: t.created_at,
                marked_done_at: None,
            }
        }
    }

    impl From<v6::Torrent> for Torrent {
        fn from(t: v6::Torrent) -> Self {
            Self {
                hash: t.hash,
                library_path: t.library_path,
                library_files: t.library_files,
                selected_audio_format: t.selected_audio_format,
                selected_ebook_format: t.selected_ebook_format,
                title_search: t.title_search,
                meta: t.meta.into(),
                created_at: t.created_at,
                replaced_with: t.replaced_with,
                request_matadata_update: t.request_matadata_update,
                library_mismatch: t.library_mismatch,
            }
        }
    }
}

pub mod v6 {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 2, version = 6, from = v5::Torrent)]
    #[native_db]
    pub struct Torrent {
        #[primary_key]
        pub hash: String,
        pub library_path: Option<PathBuf>,
        pub library_files: Vec<PathBuf>,
        pub selected_audio_format: Option<String>,
        pub selected_ebook_format: Option<String>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        #[secondary_key]
        pub created_at: Timestamp,
        pub replaced_with: Option<(String, Timestamp)>,
        pub request_matadata_update: bool,
        pub library_mismatch: Option<LibraryMismatch>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 3, version = 6, from = v4::SelectedTorrent)]
    #[native_db]
    pub struct SelectedTorrent {
        #[primary_key]
        pub mam_id: u64,
        pub dl_link: String,
        pub unsat_buffer: Option<u64>,
        pub cost: TorrentCost,
        pub category: Option<String>,
        pub tags: Vec<String>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        pub created_at: Timestamp,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 4, version = 6, from = v3::DuplicateTorrent)]
    #[native_db]
    pub struct DuplicateTorrent {
        #[primary_key]
        pub mam_id: u64,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        pub created_at: Timestamp,
        pub duplicate_of: Option<String>,
        pub request_replace: bool,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 5, version = 6, from = v3::ErroredTorrent)]
    #[native_db]
    pub struct ErroredTorrent {
        #[primary_key]
        pub id: ErroredTorrentId,
        pub title: String,
        pub error: String,
        pub meta: Option<TorrentMeta>,
        #[secondary_key]
        pub created_at: Timestamp,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct TorrentMeta {
        pub mam_id: u64,
        pub main_cat: MainCat,
        pub cat: Option<Category>,
        pub language: Option<Language>,
        pub filetypes: Vec<String>,
        pub size: Size,
        pub title: String,
        pub authors: Vec<String>,
        pub narrators: Vec<String>,
        pub series: Vec<(String, String)>,
    }

    #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

    #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub enum Category {
        Audio(AudiobookCategory),
        Ebook(EbookCategory),
    }

    impl From<v5::Torrent> for Torrent {
        fn from(t: v5::Torrent) -> Self {
            Self {
                hash: t.hash,
                library_path: t.library_path,
                library_files: t.library_files,
                selected_audio_format: t.selected_audio_format,
                selected_ebook_format: t.selected_ebook_format,
                title_search: t.title_search,
                meta: t.meta.into(),
                created_at: t.created_at,
                replaced_with: t.replaced_with,
                request_matadata_update: t.request_matadata_update,
                library_mismatch: t.library_mismatch,
            }
        }
    }

    impl From<v4::SelectedTorrent> for SelectedTorrent {
        fn from(t: v4::SelectedTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                dl_link: t.dl_link,
                unsat_buffer: t.unsat_buffer,
                cost: t.cost,
                category: t.category,
                tags: t.tags,
                title_search: t.title_search,
                meta: t.meta.into(),
                created_at: t.created_at,
            }
        }
    }

    impl From<v3::DuplicateTorrent> for DuplicateTorrent {
        fn from(t: v3::DuplicateTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                title_search: t.title_search,
                meta: t.meta.into(),
                created_at: t.created_at,
                duplicate_of: t.duplicate_of,
                request_replace: t.request_replace,
            }
        }
    }

    impl From<v3::ErroredTorrent> for ErroredTorrent {
        fn from(t: v3::ErroredTorrent) -> Self {
            Self {
                id: t.id,
                title: t.title,
                error: t.error,
                meta: t.meta.map(|t| t.into()),
                created_at: t.created_at,
            }
        }
    }

    impl From<v3::TorrentMeta> for TorrentMeta {
        fn from(t: v3::TorrentMeta) -> Self {
            Self {
                mam_id: t.mam_id,
                main_cat: t.main_cat,
                cat: None,
                language: t.language,
                filetypes: t.filetypes,
                size: t.size,
                title: t.title,
                authors: t.authors,
                narrators: t.narrators,
                series: t.series,
            }
        }
    }

    impl From<v7::SelectedTorrent> for SelectedTorrent {
        fn from(t: v7::SelectedTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                dl_link: t.dl_link,
                unsat_buffer: t.unsat_buffer,
                cost: t.cost,
                category: t.category,
                tags: t.tags,
                title_search: t.title_search,
                meta: t.meta,
                created_at: t.created_at,
            }
        }
    }

    impl From<v7::DuplicateTorrent> for DuplicateTorrent {
        fn from(t: v7::DuplicateTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                title_search: t.title_search,
                meta: t.meta.into(),
                created_at: t.created_at,
                duplicate_of: t.duplicate_of,
                request_replace: false,
            }
        }
    }
}

pub mod v7 {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 3, version = 7, from = v6::SelectedTorrent)]
    #[native_db]
    pub struct SelectedTorrent {
        #[primary_key]
        pub mam_id: u64,
        pub dl_link: String,
        pub unsat_buffer: Option<u64>,
        pub cost: TorrentCost,
        pub category: Option<String>,
        pub tags: Vec<String>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        pub created_at: Timestamp,
        pub removed_at: Option<Timestamp>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 4, version = 7, from = v6::DuplicateTorrent)]
    #[native_db]
    pub struct DuplicateTorrent {
        #[primary_key]
        pub mam_id: u64,
        pub dl_link: Option<String>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
        pub created_at: Timestamp,
        pub duplicate_of: Option<String>,
    }

    impl From<v6::SelectedTorrent> for SelectedTorrent {
        fn from(t: v6::SelectedTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                dl_link: t.dl_link,
                unsat_buffer: t.unsat_buffer,
                cost: t.cost,
                category: t.category,
                tags: t.tags,
                title_search: t.title_search,
                meta: t.meta,
                created_at: t.created_at,
                removed_at: None,
            }
        }
    }

    impl From<v6::DuplicateTorrent> for DuplicateTorrent {
        fn from(t: v6::DuplicateTorrent) -> Self {
            Self {
                mam_id: t.mam_id,
                dl_link: None,
                title_search: t.title_search,
                meta: t.meta,
                created_at: t.created_at,
                duplicate_of: t.duplicate_of,
            }
        }
    }
}
