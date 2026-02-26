use crate::dto::{Event, Torrent};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct EventsFilter {
    pub show: Option<String>,
    pub grabber: Option<String>,
    pub linker: Option<String>,
    pub category: Option<String>,
    pub has_updates: Option<String>,
    pub field: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EventWithTorrentData {
    pub event: Event,
    pub torrent: Option<Torrent>,
    pub replacement: Option<Torrent>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct EventData {
    pub events: Vec<EventWithTorrentData>,
    pub total: usize,
    pub from: usize,
    pub page_size: usize,
}
