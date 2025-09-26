use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use time::Date;

use crate::{
    data::{
        Language, MainCat, Size,
        impls::{parse, parse_opt, parse_opt_date, parse_vec},
    },
    mam_enums::{Categories, Flags, SearchIn},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub mam_id: String,
    pub audiobookshelf: Option<AudiobookShelfConfig>,
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

    #[serde(default)]
    pub qbittorrent: Vec<QbitConfig>,

    #[serde(default)]
    #[serde(rename = "library")]
    pub libraries: Vec<Library>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudiobookShelfConfig {
    pub url: String,
    pub token: String,
    #[serde(default = "default_abs_interval")]
    pub interval: u64,
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
    pub sort_by: Option<SortBy>,
    #[serde(flatten)]
    pub filter: Filter,

    pub unsat_buffer: Option<u64>,
    #[serde(default)]
    pub dry_run: bool,
    pub category: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    LowSeeders,
    LowSnatches,
    OldestFirst,
    Random,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoodreadsList {
    pub url: String,
    pub name: Option<String>,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Bookmarks,
    Freeleech,
    New,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Cost {
    #[default]
    Free,
    Wedge,
    TryWedge,
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
    pub method: LibraryLinkMethod,
    #[serde(default)]
    pub allow_tags: Vec<String>,
    #[serde(default)]
    pub deny_tags: Vec<String>,
    pub audio_types: Option<Vec<String>>,
    pub ebook_types: Option<Vec<String>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LibraryLinkMethod {
    #[default]
    Hardlink,
    HardlinkOrCopy,
    HardlinkOrSymlink,
    Copy,
    Symlink,
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

fn default_abs_interval() -> u64 {
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
