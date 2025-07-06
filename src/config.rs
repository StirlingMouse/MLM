use std::path::PathBuf;

use serde::Deserialize;
use time::{Date, UtcDateTime};
use tracing::error;

use crate::{
    data::{Language, MainCat, Size},
    data_impl::{parse, parse_opt, parse_opt_date, parse_vec},
    mam::{DATE_TIME_FORMAT, MaMTorrent},
    mam_enums::{Categories, Flags, SearchIn},
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
    #[serde(default = "default_goodreads_interval")]
    pub goodreads_interval: u64,

    #[serde(default = "default_audio_types")]
    pub audio_types: Vec<String>,
    #[serde(default = "default_ebook_types")]
    pub ebook_types: Vec<String>,

    #[serde(default)]
    #[serde(rename = "autograb")]
    pub autograbs: Vec<TorrentFilter>,

    #[serde(default)]
    #[serde(rename = "goodreads_list")]
    pub goodreads_lists: Vec<GoodreadsList>,

    #[serde(default)]
    #[serde(rename = "tag")]
    pub tags: Vec<TagFilter>,

    pub qbittorrent: Vec<QbitConfig>,

    #[serde(default)]
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
pub struct GoodreadsList {
    pub url: String,
    #[serde(default)]
    #[serde(deserialize_with = "parse_opt")]
    pub prefer_format: Option<MainCat>,
    pub grab: Vec<Grab>,

    pub unsat_buffer: Option<u64>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grab {
    #[serde(default)]
    pub cost: Cost,
    #[serde(flatten)]
    pub filter: Filter,
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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    #[serde(default)]
    pub categories: Categories,
    #[serde(default)]
    #[serde(deserialize_with = "parse_vec")]
    pub languages: Vec<Language>,
    #[serde(default)]
    pub flags: Flags,
    #[serde(default)]
    #[serde(deserialize_with = "parse")]
    pub min_size: Size,
    #[serde(default)]
    #[serde(deserialize_with = "parse")]
    pub max_size: Size,
    #[serde(default)]
    pub exclude_uploader: Vec<String>,

    #[serde(default)]
    #[serde(deserialize_with = "parse_opt_date")]
    pub uploaded_after: Option<Date>,
    #[serde(default)]
    #[serde(deserialize_with = "parse_opt_date")]
    pub uploaded_before: Option<Date>,
    pub min_seeders: Option<u64>,
    pub max_seeders: Option<u64>,
    pub min_leechers: Option<u64>,
    pub max_leechers: Option<u64>,
    pub min_snatched: Option<u64>,
    pub max_snatched: Option<u64>,
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
                error!(
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
                    error!(
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

        if self.uploaded_after.is_some() || self.uploaded_before.is_some() {
            match UtcDateTime::parse(&torrent.added, &DATE_TIME_FORMAT) {
                Ok(added) => {
                    if let Some(uploaded_after) = self.uploaded_after {
                        if added.date() < uploaded_after {
                            return false;
                        }
                    }
                    if let Some(uploaded_before) = self.uploaded_before {
                        if added.date() > uploaded_before {
                            return false;
                        }
                    }
                }
                Err(_) => {
                    error!(
                        "Failed parsing added \"{}\" for torrent \"{}\"",
                        torrent.added, torrent.title
                    );
                    return false;
                }
            }
        }

        if let Some(min_seeders) = self.min_seeders {
            if torrent.seeders < min_seeders {
                return false;
            }
        }
        if let Some(max_seeders) = self.max_seeders {
            if torrent.seeders > max_seeders {
                return false;
            }
        }
        if let Some(min_leechers) = self.min_leechers {
            if torrent.leechers < min_leechers {
                return false;
            }
        }
        if let Some(max_leechers) = self.max_leechers {
            if torrent.leechers > max_leechers {
                return false;
            }
        }
        if let Some(min_snatched) = self.min_snatched {
            if torrent.times_completed < min_snatched {
                return false;
            }
        }
        if let Some(max_snatched) = self.max_snatched {
            if torrent.times_completed > max_snatched {
                return false;
            }
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

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
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

    pub fn tag_filters(&self) -> &LibraryTagFilters {
        match self {
            Library::ByDir(l) => &l.tag_filters,
            Library::ByCategory(l) => &l.tag_filters,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryByDir {
    pub download_dir: PathBuf,
    pub library_dir: PathBuf,
    #[serde(flatten)]
    pub tag_filters: LibraryTagFilters,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryByCategory {
    pub category: String,
    pub library_dir: PathBuf,
    #[serde(flatten)]
    pub tag_filters: LibraryTagFilters,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryTagFilters {
    #[serde(default)]
    pub allow_tags: Vec<String>,
    #[serde(default)]
    pub deny_tags: Vec<String>,
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

fn default_goodreads_interval() -> u64 {
    60
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

#[cfg(test)]
mod tests {
    use time::macros::date;

    use crate::mam_enums::AudiobookCategory;

    use super::*;

    #[test]
    fn test_uploaded_after() {
        let torrent = MaMTorrent {
            category: AudiobookCategory::ActionAdventure.to_id() as u64,
            added: "2025-07-06 05:40:54".to_owned(),
            ..Default::default()
        };
        let filter = Filter {
            uploaded_after: Some(date!(2025 - 07 - 05)),
            ..Default::default()
        };
        assert!(filter.matches(&torrent));
        let filter = Filter {
            uploaded_after: Some(date!(2025 - 07 - 07)),
            ..Default::default()
        };
        assert!(!filter.matches(&torrent));
    }

    #[test]
    fn test_uploaded_after_should_be_inclusive() {
        let torrent = MaMTorrent {
            category: AudiobookCategory::ActionAdventure.to_id() as u64,
            added: "2025-07-06 05:40:54".to_owned(),
            ..Default::default()
        };
        let filter = Filter {
            uploaded_after: Some(date!(2025 - 07 - 06)),
            ..Default::default()
        };
        assert!(filter.matches(&torrent));
    }

    #[test]
    fn test_uploaded_before() {
        let torrent = MaMTorrent {
            category: AudiobookCategory::ActionAdventure.to_id() as u64,
            added: "2025-07-06 05:40:54".to_owned(),
            ..Default::default()
        };
        let filter = Filter {
            uploaded_before: Some(date!(2025 - 07 - 07)),
            ..Default::default()
        };
        assert!(filter.matches(&torrent));
        let filter = Filter {
            uploaded_before: Some(date!(2025 - 07 - 05)),
            ..Default::default()
        };
        assert!(!filter.matches(&torrent));
    }

    #[test]
    fn test_uploaded_before_should_be_inclusive() {
        let torrent = MaMTorrent {
            category: AudiobookCategory::ActionAdventure.to_id() as u64,
            added: "2025-07-06 05:40:54".to_owned(),
            ..Default::default()
        };
        let filter = Filter {
            uploaded_before: Some(date!(2025 - 07 - 06)),
            ..Default::default()
        };
        assert!(filter.matches(&torrent));
    }
}
