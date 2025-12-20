pub mod categories;
pub mod flags;
pub mod language;
pub mod meta;
pub mod old_categories;
pub mod series;
pub mod size;

use std::{fmt, str::FromStr};

use matchr::score;
use serde::{Deserialize, Deserializer};
use time::UtcDateTime;

use crate::{
    Event, EventType, ListItem, OldDbMainCat, Series, Timestamp, Torrent, TorrentCost, TorrentMeta,
    TorrentStatus, Uuid, VipStatus,
};

pub fn parse<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr<Err = String>,
    D: Deserializer<'de>,
{
    let v: String = Deserialize::deserialize(deserializer)?;
    v.parse().map_err(serde::de::Error::custom)
}

pub fn parse_opt<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: FromStr<Err = String>,
    D: Deserializer<'de>,
{
    let v: Option<String> = Deserialize::deserialize(deserializer)?;
    v.map(|v| v.parse().map_err(serde::de::Error::custom))
        .transpose()
}

pub fn parse_vec<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: FromStr<Err = String>,
    D: Deserializer<'de>,
{
    let v: Vec<String> = Deserialize::deserialize(deserializer)?;
    v.into_iter()
        .map(|v| v.parse().map_err(serde::de::Error::custom))
        .collect()
}

impl Torrent {
    pub fn matches(&self, other: &Torrent) -> bool {
        // if self.hash == other.hash { return true };
        if self.title_search != other.title_search {
            return false;
        };
        self.meta.matches(&other.meta)
    }
}

impl Event {
    pub fn new(torrent_id: Option<String>, mam_id: Option<u64>, event: EventType) -> Self {
        Self {
            id: Uuid::new(),
            torrent_id,
            mam_id,
            created_at: Timestamp::now(),
            event,
        }
    }
}

impl ListItem {
    pub fn want_audio(&self) -> bool {
        if self.marked_done_at.is_some() {
            return false;
        }
        let have_audio = self
            .audio_torrent
            .as_ref()
            .is_some_and(|t| t.status != TorrentStatus::Wanted);
        let have_ebook = self
            .ebook_torrent
            .as_ref()
            .is_some_and(|t| t.status != TorrentStatus::Wanted);

        self.allow_audio
            && !have_audio
            && self
                .prefer_format
                .is_none_or(|f| f == OldDbMainCat::Audio || !have_ebook)
    }

    pub fn want_ebook(&self) -> bool {
        if self.marked_done_at.is_some() {
            return false;
        }
        let have_audio = self
            .audio_torrent
            .as_ref()
            .is_some_and(|t| t.status != TorrentStatus::Wanted);
        let have_ebook = self
            .ebook_torrent
            .as_ref()
            .is_some_and(|t| t.status != TorrentStatus::Wanted);

        self.allow_ebook
            && !have_ebook
            && self
                .prefer_format
                .is_none_or(|f| f == OldDbMainCat::Ebook || !have_audio)
    }

    pub fn matches(&self, meta: &TorrentMeta) -> bool {
        if score(&self.title, &meta.title) < 80 {
            return false;
        }

        let authors = self
            .authors
            .iter()
            .map(|a| a.to_lowercase())
            .collect::<Vec<_>>();

        meta.authors
            .iter()
            .map(|a| a.to_lowercase())
            .any(|a| authors.iter().any(|b| score(b, &a) > 90))
    }
}

impl VipStatus {
    pub fn is_vip(&self) -> bool {
        match self {
            VipStatus::NotVip => false,
            VipStatus::Permanent => true,
            VipStatus::Temp(date) => date > &UtcDateTime::now().date(),
        }
    }
}

impl TorrentCost {
    pub fn as_str(&self) -> &'static str {
        match self {
            TorrentCost::GlobalFreeleech => "free",
            TorrentCost::PersonalFreeleech => "PF",
            TorrentCost::Vip => "VIP",
            TorrentCost::UseWedge => "wedge",
            TorrentCost::TryWedge => "try wedge",
            TorrentCost::Ratio => "ratio",
        }
    }
}

impl fmt::Display for VipStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VipStatus::NotVip => write!(f, "not VIP"),
            VipStatus::Permanent => write!(f, "VIP"),
            VipStatus::Temp(date) => write!(f, "VIP (expires {date})"),
        }
    }
}

pub fn format_serie(series: &Series) -> String {
    if series.entries.0.is_empty() {
        series.name.clone()
    } else {
        format!("{} #{}", series.name, series.entries)
    }
}
