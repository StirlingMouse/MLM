use std::str::FromStr;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer};

use crate::data::{Language, Size, Torrent, TorrentMeta};

pub fn parse<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: TryFrom<String, Error = String>,
    D: Deserializer<'de>,
{
    let v: String = Deserialize::deserialize(deserializer)?;
    v.try_into().map_err(serde::de::Error::custom)
}

pub fn parse_vec<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: TryFrom<String, Error = String>,
    D: Deserializer<'de>,
{
    let v: Vec<String> = Deserialize::deserialize(deserializer)?;
    v.into_iter()
        .map(|v| v.try_into().map_err(serde::de::Error::custom))
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

impl TorrentMeta {
    pub(crate) fn matches(&self, other: &TorrentMeta) -> bool {
        self.main_cat == other.main_cat
            && self.authors.iter().any(|a| other.authors.contains(a))
            && ((self.narrators.is_empty() && other.narrators.is_empty())
                || self.narrators.iter().any(|a| other.narrators.contains(a)))
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

impl Language {
    pub fn from_id(id: u8) -> Option<Language> {
        match id {
            1 => Some(Language::English),
            17 => Some(Language::Afrikaans),
            32 => Some(Language::Arabic),
            35 => Some(Language::Bengali),
            51 => Some(Language::Bosnian),
            18 => Some(Language::Bulgarian),
            6 => Some(Language::Burmese),
            44 => Some(Language::Cantonese),
            19 => Some(Language::Catalan),
            2 => Some(Language::Chinese),
            49 => Some(Language::Croatian),
            20 => Some(Language::Czech),
            21 => Some(Language::Danish),
            22 => Some(Language::Dutch),
            61 => Some(Language::Estonian),
            39 => Some(Language::Farsi),
            23 => Some(Language::Finnish),
            36 => Some(Language::French),
            37 => Some(Language::German),
            26 => Some(Language::Greek),
            59 => Some(Language::GreekAncient),
            3 => Some(Language::Gujarati),
            27 => Some(Language::Hebrew),
            8 => Some(Language::Hindi),
            28 => Some(Language::Hungarian),
            63 => Some(Language::Icelandic),
            53 => Some(Language::Indonesian),
            56 => Some(Language::Irish),
            43 => Some(Language::Italian),
            38 => Some(Language::Japanese),
            12 => Some(Language::Javanese),
            5 => Some(Language::Kannada),
            41 => Some(Language::Korean),
            50 => Some(Language::Lithuanian),
            46 => Some(Language::Latin),
            62 => Some(Language::Latvian),
            33 => Some(Language::Malay),
            58 => Some(Language::Malayalam),
            57 => Some(Language::Manx),
            9 => Some(Language::Marathi),
            48 => Some(Language::Norwegian),
            45 => Some(Language::Polish),
            34 => Some(Language::Portuguese),
            52 => Some(Language::BrazilianPortuguese),
            14 => Some(Language::Punjabi),
            30 => Some(Language::Romanian),
            16 => Some(Language::Russian),
            24 => Some(Language::ScottishGaelic),
            60 => Some(Language::Sanskrit),
            31 => Some(Language::Serbian),
            54 => Some(Language::Slovenian),
            4 => Some(Language::Spanish),
            55 => Some(Language::CastilianSpanish),
            40 => Some(Language::Swedish),
            29 => Some(Language::Tagalog),
            11 => Some(Language::Tamil),
            10 => Some(Language::Telugu),
            7 => Some(Language::Thai),
            42 => Some(Language::Turkish),
            25 => Some(Language::Ukrainian),
            15 => Some(Language::Urdu),
            13 => Some(Language::Vietnamese),
            47 => Some(Language::Other),
            _ => None,
        }
    }

    pub fn to_id(self) -> u8 {
        match self {
            Language::English => 1,
            Language::Afrikaans => 17,
            Language::Arabic => 32,
            Language::Bengali => 35,
            Language::Bosnian => 51,
            Language::Bulgarian => 18,
            Language::Burmese => 6,
            Language::Cantonese => 44,
            Language::Catalan => 19,
            Language::Chinese => 2,
            Language::Croatian => 49,
            Language::Czech => 20,
            Language::Danish => 21,
            Language::Dutch => 22,
            Language::Estonian => 61,
            Language::Farsi => 39,
            Language::Finnish => 23,
            Language::French => 36,
            Language::German => 37,
            Language::Greek => 26,
            Language::GreekAncient => 59,
            Language::Gujarati => 3,
            Language::Hebrew => 27,
            Language::Hindi => 8,
            Language::Hungarian => 28,
            Language::Icelandic => 63,
            Language::Indonesian => 53,
            Language::Irish => 56,
            Language::Italian => 43,
            Language::Japanese => 38,
            Language::Javanese => 12,
            Language::Kannada => 5,
            Language::Korean => 41,
            Language::Lithuanian => 50,
            Language::Latin => 46,
            Language::Latvian => 62,
            Language::Malay => 33,
            Language::Malayalam => 58,
            Language::Manx => 57,
            Language::Marathi => 9,
            Language::Norwegian => 48,
            Language::Polish => 45,
            Language::Portuguese => 34,
            Language::BrazilianPortuguese => 52,
            Language::Punjabi => 14,
            Language::Romanian => 30,
            Language::Russian => 16,
            Language::ScottishGaelic => 24,
            Language::Sanskrit => 60,
            Language::Serbian => 31,
            Language::Slovenian => 54,
            Language::Spanish => 4,
            Language::CastilianSpanish => 55,
            Language::Swedish => 40,
            Language::Tagalog => 29,
            Language::Tamil => 11,
            Language::Telugu => 10,
            Language::Thai => 7,
            Language::Turkish => 42,
            Language::Ukrainian => 25,
            Language::Urdu => 15,
            Language::Vietnamese => 13,
            Language::Other => 47,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            Language::English => "english",
            Language::Afrikaans => "afrikaans",
            Language::Arabic => "arabic",
            Language::Bengali => "bengali",
            Language::Bosnian => "bosnian",
            Language::Bulgarian => "bulgarian",
            Language::Burmese => "burmese",
            Language::Cantonese => "cantonese",
            Language::Catalan => "catalan",
            Language::Chinese => "chinese",
            Language::Croatian => "croatian",
            Language::Czech => "czech",
            Language::Danish => "danish",
            Language::Dutch => "dutch",
            Language::Estonian => "estonian",
            Language::Farsi => "farsi",
            Language::Finnish => "finnish",
            Language::French => "french",
            Language::German => "german",
            Language::Greek => "greek",
            Language::GreekAncient => "ancient greek",
            Language::Gujarati => "gujarati",
            Language::Hebrew => "hebrew",
            Language::Hindi => "hindi",
            Language::Hungarian => "hungarian",
            Language::Icelandic => "icelandic",
            Language::Indonesian => "indonesian",
            Language::Irish => "irish",
            Language::Italian => "italian",
            Language::Japanese => "japanese",
            Language::Javanese => "javanese",
            Language::Kannada => "kannada",
            Language::Korean => "korean",
            Language::Lithuanian => "lithuanian",
            Language::Latin => "latin",
            Language::Latvian => "latvian",
            Language::Malay => "malay",
            Language::Malayalam => "malayalam",
            Language::Manx => "manx",
            Language::Marathi => "marathi",
            Language::Norwegian => "norwegian",
            Language::Polish => "polish",
            Language::Portuguese => "portuguese",
            Language::BrazilianPortuguese => "brazilian",
            Language::Punjabi => "punjabi",
            Language::Romanian => "romanian",
            Language::Russian => "russian",
            Language::ScottishGaelic => "scottish",
            Language::Sanskrit => "sanskrit",
            Language::Serbian => "serbian",
            Language::Slovenian => "slovenian",
            Language::Spanish => "spanish",
            Language::CastilianSpanish => "castilian",
            Language::Swedish => "swedish",
            Language::Tagalog => "tagalog",
            Language::Tamil => "tamil",
            Language::Telugu => "telugu",
            Language::Thai => "thai",
            Language::Turkish => "turkish",
            Language::Ukrainian => "ukrainian",
            Language::Urdu => "urdu",
            Language::Vietnamese => "vietnamese",
            Language::Other => "other",
        }
    }
}

impl FromStr for Language {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let l = match value.to_lowercase().as_str() {
            "english" => Some(Language::English),
            "afrikaans" => Some(Language::Afrikaans),
            "arabic" => Some(Language::Arabic),
            "bengali" => Some(Language::Bengali),
            "bosnian" => Some(Language::Bosnian),
            "bulgarian" => Some(Language::Bulgarian),
            "burmese" => Some(Language::Burmese),
            "cantonese" => Some(Language::Cantonese),
            "catalan" => Some(Language::Catalan),
            "chinese" => Some(Language::Chinese),
            "croatian" => Some(Language::Croatian),
            "czech" => Some(Language::Czech),
            "danish" => Some(Language::Danish),
            "dutch" => Some(Language::Dutch),
            "estonian" => Some(Language::Estonian),
            "farsi" => Some(Language::Farsi),
            "finnish" => Some(Language::Finnish),
            "french" => Some(Language::French),
            "german" => Some(Language::German),
            "greek" => Some(Language::Greek),
            "ancient greek" => Some(Language::GreekAncient),
            "greek ancient" => Some(Language::GreekAncient),
            "greek, ancient" => Some(Language::GreekAncient),
            "gujarati" => Some(Language::Gujarati),
            "hebrew" => Some(Language::Hebrew),
            "hindi" => Some(Language::Hindi),
            "hungarian" => Some(Language::Hungarian),
            "icelandic" => Some(Language::Icelandic),
            "indonesian" => Some(Language::Indonesian),
            "irish" => Some(Language::Irish),
            "italian" => Some(Language::Italian),
            "japanese" => Some(Language::Japanese),
            "javanese" => Some(Language::Javanese),
            "kannada" => Some(Language::Kannada),
            "korean" => Some(Language::Korean),
            "lithuanian" => Some(Language::Lithuanian),
            "latin" => Some(Language::Latin),
            "latvian" => Some(Language::Latvian),
            "malay" => Some(Language::Malay),
            "malayalam" => Some(Language::Malayalam),
            "manx" => Some(Language::Manx),
            "marathi" => Some(Language::Marathi),
            "norwegian" => Some(Language::Norwegian),
            "polish" => Some(Language::Polish),
            "portuguese" => Some(Language::Portuguese),
            "bp" => Some(Language::BrazilianPortuguese),
            "brazilian" => Some(Language::BrazilianPortuguese),
            "brazilian portuguese" => Some(Language::BrazilianPortuguese),
            "brazilian portuguese (bp)" => Some(Language::BrazilianPortuguese),
            "punjabi" => Some(Language::Punjabi),
            "romanian" => Some(Language::Romanian),
            "russian" => Some(Language::Russian),
            "scottish" => Some(Language::ScottishGaelic),
            "scottish gaelic" => Some(Language::ScottishGaelic),
            "gaelic" => Some(Language::ScottishGaelic),
            "sanskrit" => Some(Language::Sanskrit),
            "serbian" => Some(Language::Serbian),
            "slovenian" => Some(Language::Slovenian),
            "spanish" => Some(Language::Spanish),
            "castilian" => Some(Language::CastilianSpanish),
            "castilian spanish" => Some(Language::CastilianSpanish),
            "swedish" => Some(Language::Swedish),
            "tagalog" => Some(Language::Tagalog),
            "tamil" => Some(Language::Tamil),
            "telugu" => Some(Language::Telugu),
            "thai" => Some(Language::Thai),
            "turkish" => Some(Language::Turkish),
            "ukrainian" => Some(Language::Ukrainian),
            "urdu" => Some(Language::Urdu),
            "vietnamese" => Some(Language::Vietnamese),
            "other" => Some(Language::Other),
            _ => None,
        };
        match l {
            Some(l) => Ok(l),
            None => Err(format!("invalid language {value}")),
        }
    }
}

impl TryFrom<String> for Language {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
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
