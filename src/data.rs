use native_db::{Models, ToKey, native_db};
use native_model::{Model, native_model};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<v1::Torrent>().unwrap();
    models.define::<v1::Config>().unwrap();
    models
});

pub type Torrent = v1::Torrent;
pub type Config = v1::Config;

pub mod v1 {
    use std::path::PathBuf;

    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    #[native_model(id = 1, version = 1)]
    #[native_db]
    pub struct Torrent {
        #[primary_key]
        pub hash: String,
        pub library_path: PathBuf,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[native_model(id = 2, version = 1)]
    #[native_db]
    pub struct Config {
        #[primary_key]
        pub key: String,
        pub value: String,
    }
}
