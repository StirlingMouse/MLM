use anyhow::Result;
use native_db::{Database, Key};
use native_db::{Models, ToKey, native_db};
use native_model::{Model, native_model};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::OffsetDateTime;

pub static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<v1::Config>().unwrap();

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
pub type Torrent = v2::Torrent;
pub type TorrentKey = v2::TorrentKey;
pub type SelectedTorrent = v2::SelectedTorrent;
pub type SelectedTorrentKey = v2::SelectedTorrentKey;
pub type DuplicateTorrent = v2::DuplicateTorrent;
pub type ErroredTorrent = v2::ErroredTorrent;
pub type ErroredTorrentId = v1::ErroredTorrentId;
pub type TorrentMeta = v1::TorrentMeta;
pub type MainCat = v1::MainCat;

pub fn migrate(db: &Database<'_>) -> Result<()> {
    let rw = db.rw_transaction()?;

    rw.migrate::<v2::Torrent>()?;
    rw.migrate::<v2::SelectedTorrent>()?;
    rw.migrate::<v2::DuplicateTorrent>()?;
    rw.migrate::<v2::ErroredTorrent>()?;
    rw.commit()?;
    println!("Migrations done");

    Ok(())
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
    use super::*;

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
}
