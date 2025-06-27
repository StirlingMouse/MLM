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
    #[serde(default = "default_unsat_buffer")]
    pub unsat_buffer: u64,
    #[serde(default)]
    pub add_torrents_stopped: bool,

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

        if self.max_size.bytes() > 0 {
            match Size::try_from(torrent.size.clone()) {
                Ok(size) => {
                    if size > self.max_size {
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
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Library {
    pub download_dir: PathBuf,
    pub library_dir: PathBuf,
}

fn default_unsat_buffer() -> u64 {
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
