use std::path::PathBuf;

use serde::Deserialize;

use crate::{
    mam::MaMTorrent,
    mam_enums::{Categories, Flags, Language, SearchIn, Size},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub mam_id: String,
    #[serde(default = "default_host")]
    pub web_host: String,
    #[serde(default = "default_port")]
    pub web_port: u16,
    #[serde(default = "default_unsat_buffer")]
    pub unsat_buffer: u64,
    #[serde(default)]
    pub add_torrents_stopped: bool,
    #[serde(default)]
    pub exclude_narrator_in_library_dir: bool,
    #[serde(default = "default_search_interval")]
    pub search_interval: u64,
    #[serde(default = "default_link_interval")]
    pub link_interval: u64,

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
#[serde(deny_unknown_fields)]
pub struct TorrentFilter {
    #[serde(rename = "type")]
    pub kind: Type,
    #[serde(default)]
    pub cost: Cost,
    pub query: Option<String>,
    #[serde(default)]
    pub search_in: Vec<SearchIn>,
    #[serde(flatten)]
    pub filter: Filter,
    pub unsat_buffer: Option<u64>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TagFilter {
    #[serde(flatten)]
    pub filter: Filter,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    #[serde(default)]
    pub categories: Categories,
    #[serde(default)]
    pub languages: Vec<Language>,
    #[serde(default)]
    pub flags: Flags,
    #[serde(default)]
    pub min_size: Size,
    #[serde(default)]
    pub max_size: Size,
    #[serde(default)]
    pub exclude_uploader: Vec<String>,
}

impl Filter {
    pub fn matches(&self, torrent: &MaMTorrent) -> bool {
        if !self.categories.matches(torrent.category) {
            return false;
        }

        if !self.languages.is_empty() {
            if let Some(language) = Language::from_id(torrent.language) {
                if !self.languages.contains(&language) {
                    return false;
                }
            } else {
                eprintln!(
                    "Failed parsing language \"{}\" for torrent \"{}\"",
                    torrent.language, torrent.title
                );
                return false;
            }
        }

        let torrent_flags = Flags::from_bitfield(torrent.browseflags);
        if !self.flags.matches(&torrent_flags) {
            return false;
        }

        if self.min_size.bytes() > 0 || self.max_size.bytes() > 0 {
            match Size::try_from(torrent.size.clone()) {
                Ok(size) => {
                    if self.min_size.bytes() > 0 && size < self.min_size {
                        return false;
                    }
                    if self.max_size.bytes() > 0 && size > self.max_size {
                        return false;
                    }
                }
                Err(_) => {
                    eprintln!(
                        "Failed parsing size \"{}\" for torrent \"{}\"",
                        torrent.size, torrent.title
                    );
                    return false;
                }
            };
        }

        if self.exclude_uploader.contains(&torrent.owner_name) {
            return false;
        }

        true
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Bookmarks,
    Freeleech,
    New,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cost {
    #[default]
    Free,
    All,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QbitConfig {
    pub url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    pub tags: Option<Vec<String>>,
    pub on_cleaned: Option<QbitOnCleaned>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QbitOnCleaned {
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Library {
    ByDir(LibraryByDir),
    ByCategory(LibraryByCategory),
}

impl Library {
    pub fn library_dir(&self) -> &PathBuf {
        match self {
            Library::ByDir(l) => &l.library_dir,
            Library::ByCategory(l) => &l.library_dir,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryByDir {
    pub download_dir: PathBuf,
    pub library_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryByCategory {
    pub category: String,
    pub library_dir: PathBuf,
}

fn default_host() -> String {
    "0.0.0.0".to_owned()
}

fn default_port() -> u16 {
    3157
}

fn default_unsat_buffer() -> u64 {
    10
}

fn default_search_interval() -> u64 {
    30
}

fn default_link_interval() -> u64 {
    10
}

fn default_audio_types() -> Vec<String> {
    ["m4b", "m4a", "mp4", "mp3", "ogg"]
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn default_ebook_types() -> Vec<String> {
    ["cbz", "epub", "pdf", "mobi", "azw3", "azw", "cbr"]
        .iter()
        .map(ToString::to_string)
        .collect()
}
