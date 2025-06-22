use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub mam_id: String,
    pub qbittorrent: QbitConfig,
    #[serde(default = "default_audio_types")]
    pub audio_types: Vec<String>,
    #[serde(default = "default_ebook_types")]
    pub ebook_types: Vec<String>,
    #[serde(alias = "library")]
    pub libraries: Vec<Library>,
}

#[derive(Debug, Deserialize)]
pub struct QbitConfig {
    pub url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Library {
    pub download_dir: PathBuf,
    pub library_dir: PathBuf,
}

fn default_audio_types() -> Vec<String> {
    ["m4b", "mp3", "ogg"]
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn default_ebook_types() -> Vec<String> {
    ["cbz", "epub", "pdf", "mobi", "azw3", "cbr"]
        .iter()
        .map(ToString::to_string)
        .collect()
}
