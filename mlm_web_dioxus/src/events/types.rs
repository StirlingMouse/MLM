use crate::dto::{Event, Torrent};
use serde::{Deserialize, Serialize};

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
