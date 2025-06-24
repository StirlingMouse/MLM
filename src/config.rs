use std::path::PathBuf;

use serde::Deserialize;

use crate::mam_enums::{Categories, Language, SearchIn, Size};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub mam_id: String,
    #[serde(default = "default_unsat_buffer")]
    pub unsat_buffer: u8,

    #[serde(rename = "autograb")]
    pub autograbs: Vec<TorrentFilter>,

    #[serde(rename = "tag")]
    pub tags: Vec<TagFilter>,

    #[serde(default = "default_audio_types")]
    pub audio_types: Vec<String>,
    #[serde(default = "default_ebook_types")]
    pub ebook_types: Vec<String>,

    pub qbittorrent: Vec<QbitConfig>,
    #[serde(rename = "library")]
    pub libraries: Vec<Library>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TorrentFilter {
    #[serde(flatten)]
    pub filter: Filter,
    pub unsat_buffer: Option<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TagFilter {
    #[serde(flatten)]
    pub filter: Filter,
    pub category: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Filter {
    #[serde(rename = "type")]
    pub kind: Type,
    #[serde(default)]
    pub cost: Cost,
    #[serde(default)]
    pub max_size: Size,
    pub query: Option<String>,
    #[serde(default)]
    pub search_in: Vec<SearchIn>,
    #[serde(default)]
    pub categories: Categories,
    #[serde(default)]
    pub languages: Vec<Language>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Bookmarks,
    Freeleech,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cost {
    #[default]
    Free,
    All,
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

fn default_unsat_buffer() -> u8 {
    10
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
