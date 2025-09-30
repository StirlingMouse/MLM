pub mod categories;
pub mod language;
pub mod series;

use std::str::FromStr;

use itertools::Itertools as _;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer};
use time::Date;

use crate::{
    data::{
        ListItem, MainCat, Size, Torrent, TorrentCost, TorrentMeta, TorrentMetaDiff,
        TorrentMetaField, TorrentStatus,
    },
    mam::DATE_FORMAT,
    mam_enums::Flags,
};

use super::{FlagBits, Series};

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

pub fn parse_opt_date<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<String> = Deserialize::deserialize(deserializer)?;
    v.map(|v| Date::parse(&v, &DATE_FORMAT).map_err(serde::de::Error::custom))
        .transpose()
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

impl TorrentMeta {
    pub(crate) fn matches(&self, other: &TorrentMeta) -> bool {
        self.main_cat == other.main_cat
            && self.authors.iter().any(|a| other.authors.contains(a))
            && ((self.narrators.is_empty() && other.narrators.is_empty())
                || self.narrators.iter().any(|a| other.narrators.contains(a)))
    }

    pub(crate) fn diff(&self, other: &TorrentMeta) -> Vec<TorrentMetaDiff> {
        let mut diff = vec![];
        if self.mam_id != other.mam_id {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::MamId,
                from: self.mam_id.to_string(),
                to: other.mam_id.to_string(),
            });
        }
        if self.main_cat != other.main_cat {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::MainCat,
                from: self.main_cat.as_str().to_string(),
                to: other.main_cat.as_str().to_string(),
            });
        }
        if self.cat != other.cat {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Cat,
                from: self
                    .cat
                    .as_ref()
                    .map(|cat| cat.as_str().to_string())
                    .unwrap_or_default(),
                to: other
                    .cat
                    .as_ref()
                    .map(|cat| cat.as_str().to_string())
                    .unwrap_or_default(),
            });
        }
        if self.language != other.language {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Language,
                from: self
                    .language
                    .map(|language| language.to_str().to_string())
                    .unwrap_or_default(),
                to: other
                    .language
                    .map(|language| language.to_str().to_string())
                    .unwrap_or_default(),
            });
        }
        if self.flags != other.flags {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Flags,
                from: self
                    .flags
                    .map(|flags| format!("{}", Flags::from(flags)))
                    .unwrap_or_default(),
                to: other
                    .flags
                    .map(|flags| format!("{}", Flags::from(flags)))
                    .unwrap_or_default(),
            });
        }
        if self.filetypes != other.filetypes {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Filetypes,
                from: self.filetypes.join(", ").to_string(),
                to: other.filetypes.join(", ").to_string(),
            });
        }
        if self.size != other.size {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Size,
                from: self.size.to_string(),
                to: other.size.to_string(),
            });
        }
        if self.title != other.title {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Title,
                from: self.title.to_string(),
                to: other.title.to_string(),
            });
        }
        if self.authors != other.authors {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Authors,
                from: self.authors.join(", ").to_string(),
                to: other.authors.join(", ").to_string(),
            });
        }
        if self.narrators != other.narrators {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Narrators,
                from: self.narrators.join(", ").to_string(),
                to: other.narrators.join(", ").to_string(),
            });
        }
        if self.series != other.series {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Series,
                from: self.series.iter().map(format_serie).join(", ").to_string(),
                to: other.series.iter().map(format_serie).join(", ").to_string(),
            });
        }
        diff
    }
}

impl std::fmt::Display for TorrentMetaField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TorrentMetaField::MamId => write!(f, "mam_id"),
            TorrentMetaField::MainCat => write!(f, "main_cat"),
            TorrentMetaField::Cat => write!(f, "cat"),
            TorrentMetaField::Language => write!(f, "language"),
            TorrentMetaField::Flags => write!(f, "flags"),
            TorrentMetaField::Filetypes => write!(f, "filetypes"),
            TorrentMetaField::Size => write!(f, "size"),
            TorrentMetaField::Title => write!(f, "title"),
            TorrentMetaField::Authors => write!(f, "authors"),
            TorrentMetaField::Narrators => write!(f, "narrators"),
            TorrentMetaField::Series => write!(f, "series"),
        }
    }
}

impl ListItem {
    pub fn want_audio(&self) -> bool {
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
                .is_none_or(|f| f == MainCat::Audio || !have_ebook)
    }

    pub fn want_ebook(&self) -> bool {
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
                .is_none_or(|f| f == MainCat::Ebook || !have_audio)
    }
}

impl Size {
    pub fn unit(self) -> u64 {
        if self.bytes() > 0 { 1 } else { 0 }
    }
}

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value = self.bytes() as f64;
        let mut unit = "B";
        if value > 1024_f64.powf(3.0) {
            value /= 1024_f64.powf(3.0);
            unit = "GiB";
        } else if value > 1024_f64.powf(2.0) {
            value /= 1024_f64.powf(2.0);
            unit = "MiB";
        } else if value > 1024.0 {
            value /= 1024.0;
            unit = "KiB";
        }
        let value = ((value * 1000.0).round() as u64) / 1000;
        write!(f, "{} {}", value, unit)
    }
}

pub static SIZE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^((?:\d{1,3},)?\d{1,6}(?:\.\d{1,3})?) ([kKMG]?)(i)?B$").unwrap());

impl FromStr for Size {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Some((Some(value), Some(unit), i)) = SIZE_PATTERN
            .captures(value)
            .map(|c| (c.get(1), c.get(2), c.get(3)))
        {
            let value: f64 = value.as_str().replace(",", "").parse().unwrap();
            let base: u64 = if i.is_some() { 1024 } else { 1000 };
            let multiplier = match unit.as_str() {
                "" => 1,
                "k" | "K" => base,
                "M" => base.pow(2),
                "G" => base.pow(3),
                _ => unreachable!("unknown unit: {}", unit.as_str()),
            } as f64;
            Ok(Size::from_bytes((value * multiplier).round() as u64))
        } else {
            Err(format!("invalid size value {value}"))
        }
    }
}

impl TryFrom<String> for Size {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
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

impl From<Flags> for FlagBits {
    fn from(value: Flags) -> Self {
        FlagBits::new(value.as_bitfield())
    }
}

impl From<FlagBits> for Flags {
    fn from(value: FlagBits) -> Self {
        Flags::from_bitfield(value.0)
    }
}

pub fn format_serie(series: &Series) -> String {
    if series.entries.0.is_empty() {
        series.name.clone()
    } else {
        format!("{} #{}", series.name, series.entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_thousands_divider() {
        assert_eq!(
            Size::from_str("1,016.2 KiB"),
            Ok(Size::from_bytes(1_040_589))
        );
    }
}
