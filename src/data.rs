use native_db::{Models, ToKey, native_db};
use native_model::{Model, native_model};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<v1::Config>().unwrap();
    models.define::<v1::Torrent>().unwrap();
    models.define::<v1::SelectedTorrent>().unwrap();
    models.define::<v1::DuplicateTorrent>().unwrap();
    models.define::<v1::ErroredTorrent>().unwrap();
    models
});

pub type Config = v1::Config;
pub type Torrent = v1::Torrent;
pub type TorrentKey = v1::TorrentKey;
pub type SelectedTorrent = v1::SelectedTorrent;
pub type SelectedTorrentKey = v1::SelectedTorrentKey;
pub type DuplicateTorrent = v1::DuplicateTorrent;
pub type ErroredTorrent = v1::ErroredTorrent;
pub type ErroredTorrentId = v1::ErroredTorrentId;
pub type TorrentMeta = v1::TorrentMeta;
pub type MainCat = v1::MainCat;

pub mod v1 {
    use native_db::Key;

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

    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MainCat {
        Audio,
        Ebook,
    }
    impl MainCat {
        pub(crate) fn from_id(main_cat: u64) -> Option<MainCat> {
            match main_cat {
                13 => Some(MainCat::Audio),
                14 => Some(MainCat::Ebook),
                _ => None,
            }
        }
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
}
