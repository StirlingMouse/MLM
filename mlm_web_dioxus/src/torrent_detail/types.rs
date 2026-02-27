use crate::{
    dto::{Event, Series, TorrentMetaDiff},
    search::SearchTorrent,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentDetailData {
    pub torrent: TorrentInfo,
    pub events: Vec<Event>,
    pub replacement_torrent: Option<ReplacementTorrentInfo>,
    pub replacement_missing: bool,
    pub abs_item_url: Option<String>,
    pub abs_cover_url: Option<String>,
    pub mam_torrent: Option<SearchTorrent>,
    pub mam_meta_diff: Vec<TorrentMetaDiff>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentInfo {
    pub id: String,
    pub title: String,
    pub edition: Option<String>,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<Series>,
    pub tags: Vec<String>,
    pub description: String,
    pub media_type: String,
    pub mediatype_id: u8,
    pub main_cat: Option<String>,
    pub main_cat_id: u8,
    pub language: Option<String>,
    pub filetypes: Vec<String>,
    pub size: String,
    pub num_files: u64,
    pub categories: Vec<String>,
    pub old_category: Option<String>,
    pub flags: Vec<String>,
    pub library_path: Option<PathBuf>,
    pub library_files: Vec<PathBuf>,
    pub linker: Option<String>,
    pub category: Option<String>,
    pub mam_id: Option<u64>,
    pub vip_status: Option<String>,
    pub source: String,
    pub uploaded_at: String,
    pub client_status: Option<String>,
    pub replaced_with: Option<String>,
    pub goodreads_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TorrentPageData {
    Downloaded(TorrentDetailData),
    MamOnly(TorrentMamData),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentMamData {
    pub mam_torrent: SearchTorrent,
    pub meta: TorrentInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplacementTorrentInfo {
    pub id: String,
    pub title: String,
    pub size: String,
    pub filetypes: Vec<String>,
    pub library_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct QbitData {
    pub torrent_state: String,
    pub torrent_category: String,
    pub torrent_tags: Vec<String>,
    pub categories: Vec<QbitCategory>,
    pub tags: Vec<String>,
    pub trackers: Vec<QbitTracker>,
    pub tracker_message: Option<String>,
    pub uploaded: String,
    pub wanted_path: Option<PathBuf>,
    pub no_longer_wanted: bool,
    pub qbit_files: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct QbitCategory {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct QbitTracker {
    pub url: String,
    pub msg: Option<String>,
}
