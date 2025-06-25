use native_db::{Models, ToKey, native_db};
use native_model::{Model, native_model};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<v1::Config>().unwrap();
    models.define::<v1::Torrent>().unwrap();
    models.define::<v1::SelectedTorrent>().unwrap();
    models
});

pub type Config = v1::Config;
pub type Torrent = v1::Torrent;
pub type TorrentKey = v1::TorrentKey;
pub type SelectedTorrent = v1::SelectedTorrent;
pub type SelectedTorrentKey = v1::SelectedTorrentKey;
pub type TorrentMeta = v1::TorrentMeta;
pub type MainCat = v1::MainCat;

pub mod v1 {
    use std::path::PathBuf;

    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    #[native_model(id = 1, version = 1)]
    #[native_db]
    pub struct Config {
        #[primary_key]
        pub key: String,
        pub value: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[native_model(id = 2, version = 1)]
    #[native_db]
    pub struct Torrent {
        #[primary_key]
        pub hash: String,
        pub library_path: Option<PathBuf>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[native_model(id = 3, version = 1)]
    #[native_db]
    pub struct SelectedTorrent {
        #[primary_key]
        pub mam_id: u64,
        pub dl_link: String,
        pub unsat_buffer: Option<u8>,
        pub category: Option<String>,
        pub tags: Vec<String>,
        #[secondary_key]
        pub title_search: String,
        pub meta: TorrentMeta,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct TorrentMeta {
        pub mam_id: u64,
        pub main_cat: MainCat,
        pub filetype: String,
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
}
