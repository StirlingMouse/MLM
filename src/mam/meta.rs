use anyhow::{Error, Result};
use htmlentity::entity::{self, ICodedDataTrait as _};
use once_cell::sync::Lazy;
use regex::{Captures, Match, Regex};
use unidecode::unidecode;

use crate::data::{MediaType, OldCategory, TorrentMeta};

#[derive(thiserror::Error, Debug)]
pub enum MetaError {
    #[error("{0}")]
    UnknownMediaType(String),
    #[error("Unknown category: {0}")]
    UnknownCat(u8),
    #[error("Unknown old category: {0} ({1})")]
    UnknownOldCat(String, u64),
    #[error("Unknown language id {0}, code: {1}")]
    UnknownLanguage(u8, String),
    #[error("{0}")]
    InvalidSize(String),
    #[error("{0}")]
    InvalidSeries(&'static str),
    #[error("Invalid added date: {0}")]
    InvalidAdded(String),
    #[error("Invalid vip_expiry: {0}")]
    InvalidVipExpiry(u64),
    #[error("Unknown error: {0}")]
    Other(#[from] Error),
}

pub fn clean_value(value: &str) -> Result<String> {
    entity::decode(value.as_bytes()).to_string()
}

pub fn normalize_title(value: &str) -> String {
    unidecode(value).to_lowercase().replace(" & ", " and ")
}

impl TorrentMeta {
    pub fn clean(mut self, tags: &str) -> Result<Self> {
        // A large amount of audiobook torrents have been incorrectly set to ebook
        if self.media_type == MediaType::Ebook
            && let Some(OldCategory::Audio(_)) = self.cat
        {
            self.media_type = MediaType::Audiobook;
        }
        for author in &mut self.authors {
            *author = clean_value(author)?;
        }
        for narrator in &mut self.narrators {
            *narrator = clean_value(narrator)?;
        }
        for series in &mut self.series {
            series.name = SERIES_CLEANUP
                .replace_all(&clean_value(&series.name)?, "")
                .to_string();
        }

        let (title, edition) = parse_edition(&self.title, tags);
        self.title = title;
        self.edition = edition;

        if self.authors.len() == 1
            && let Some(author) = self.authors.first()
        {
            if let Some(title) = self.title.strip_suffix(author) {
                if let Some(title) = title
                    .strip_suffix(" by ")
                    .or_else(|| title.strip_suffix(" - "))
                {
                    self.title = title.trim().to_string();
                }
            } else if let Some(title) = self.title.strip_prefix(author)
                && let Some(title) = title.strip_prefix(" - ")
            {
                self.title = title.trim().to_string();
            }
        }

        self.title = TITLE_CLEANUP.replace_all(&self.title, "").to_string();

        Ok(self)
    }
}

static EDITION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(.*?)(?:(?:(?:\s*[-–.:;|,]\s*)((\w+?)\s+(?:[a-z]+\s+)*(?:Edition|ed\.))|(?:\s*[-–.:;|,]\s*)?(?:\s*[(\[]\s*)((\w*?)\s+(?:[a-z]+\s+)*(?:Edition|ed\.))(?:\s*[)\]]\s*))(?:\s*[-:;,]\s*)?(.*?)|\s+((\d+\w*?)\s+(?:Edition|ed\.)))$").unwrap()
});

static EDITION_START_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)((\d+(?:st|nd|rd|th)|first|second|third|fifth|sixth|seventh|eight|ninth|tenth|new|revised|updated)\s+(?:[a-z']+\s+)*(?:Edition|ed\.)|(\w+?)\s+(?:Edition|ed\.))").unwrap()
});

static TITLE_CLEANUP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(?:: A (?:Novel|Memoir)$)|(?:\s*-\s*\d+(?:\.| - )epub$)|(?:\s*[\(\[](?:digital|light novel|epub|cbz|tpb)[\)\]])*",
    )
    .unwrap()
});

static SERIES_CLEANUP: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?:\s*\((?:digital|light novel)\))*").unwrap());

pub fn parse_edition(title: &str, tags: &str) -> (String, Option<(String, u64)>) {
    if let Some(captures) = EDITION_REGEX.captures(title)
        && let Some(edition) = parse_normal_edition_match(&captures)
    {
        let mut title_str = captures.get(1).unwrap().as_str().to_string();
        if let Some(subtitle_match) = captures.get(6) {
            let subtitle_str = subtitle_match.as_str();
            if !subtitle_str.is_empty() {
                title_str.push_str(&format!(": {}", subtitle_str.trim()));
            }
        }
        return (title_str, Some(edition));
    }

    if let Some(captures) = EDITION_REGEX.captures(tags)
        && let Some(edition) = parse_normal_edition_match(&captures)
    {
        return (title.to_string(), Some(edition));
    }
    if let Some(captures) = EDITION_START_REGEX.captures(tags)
        && let Some(edition) = parse_start_edition_match(&captures)
    {
        return (title.to_string(), Some(edition));
    }

    (title.to_string(), None)
}

fn parse_normal_edition_match(captures: &Captures) -> Option<(String, u64)> {
    let edition_match = captures
        .get(2)
        .or_else(|| captures.get(4))
        .or_else(|| captures.get(7))?;

    let edition_number = captures
        .get(3)
        .or_else(|| captures.get(5))
        .or_else(|| captures.get(8))?;

    parse_edition_match(edition_match, edition_number)
}
fn parse_start_edition_match(captures: &Captures) -> Option<(String, u64)> {
    let edition_match = captures.get(1)?;
    let edition_number = captures.get(2).or_else(|| captures.get(3))?;

    parse_edition_match(edition_match, edition_number)
}

fn parse_edition_match(edition_match: Match, edition_number: Match) -> Option<(String, u64)> {
    let edition_number = match edition_number.as_str().to_lowercase().as_str() {
        "first" | "1st" => 1,
        "second" | "2nd" => 2,
        "third" | "3rd" => 3,
        "fourth" | "4th" => 4,
        "fifth" | "5th" => 5,
        "sixth" | "6th" => 6,
        "seventh" | "7th" => 7,
        "eighth" | "8th" => 8,
        "ninth" | "9th" => 9,
        "tenth" | "10th" => 10,
        n if n.ends_with("th") => n[..n.len() - 2].parse().unwrap_or(0),
        n => n.parse().unwrap_or(0),
    };
    let mut edition_str = edition_match.as_str().to_string();

    let mut first_letter = true;
    for (i, c) in edition_str.clone().char_indices() {
        if first_letter
            && !c.is_uppercase()
            && let Some(char) = edition_str.get_mut(i..=i)
        {
            char.make_ascii_uppercase();
        }
        first_letter = c == ' ';
    }
    if edition_str.ends_with("Ed.") {
        edition_str.replace_range((edition_str.len() - 3).., "Edition");
    }

    Some((edition_str, edition_number))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_edition_base() {
        let (parsed_title, parsed_edition) = parse_edition("Title", "Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, None);
    }

    #[test]
    fn test_parse_edition_in_title() {
        let (parsed_title, parsed_edition) = parse_edition("Title (1st edition)", "Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("1st Edition".to_string(), 1)));

        let (parsed_title, parsed_edition) = parse_edition("Title [2nd edition]", "Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("2nd Edition".to_string(), 2)));

        let (parsed_title, parsed_edition) = parse_edition("Title: 3rd Edition", "Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("3rd Edition".to_string(), 3)));

        let (parsed_title, parsed_edition) = parse_edition("Title, 10th Edition", "Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("10th Edition".to_string(), 10)));

        let (parsed_title, parsed_edition) = parse_edition("Title (first edition)", "Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("First Edition".to_string(), 1)));

        let (parsed_title, parsed_edition) = parse_edition("Title [Second edition]", "Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("Second Edition".to_string(), 2)));

        let (parsed_title, parsed_edition) = parse_edition("Title: Third Edition", "Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("Third Edition".to_string(), 3)));

        let (parsed_title, parsed_edition) =
            parse_edition("Title, Fourth Canadian Edition", "Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(
            parsed_edition,
            Some(("Fourth Canadian Edition".to_string(), 4))
        );

        let (parsed_title, parsed_edition) = parse_edition("Title, 1st edition A subtitle", "Tags");
        assert_eq!(parsed_title, "Title: A subtitle");
        assert_eq!(parsed_edition, Some(("1st Edition".to_string(), 1)));

        let (parsed_title, parsed_edition) =
            parse_edition("Title, and some more title, 1st edition A subtitle", "Tags");
        assert_eq!(parsed_title, "Title, and some more title: A subtitle");
        assert_eq!(parsed_edition, Some(("1st Edition".to_string(), 1)));

        let (parsed_title, parsed_edition) =
            parse_edition("Title [Second edition] A subtitle", "Tags");
        assert_eq!(parsed_title, "Title: A subtitle");
        assert_eq!(parsed_edition, Some(("Second Edition".to_string(), 2)));

        let (parsed_title, parsed_edition) =
            parse_edition("Title - 25th Anniversary Edition: A subtitle", "Tags");
        assert_eq!(parsed_title, "Title: A subtitle");
        assert_eq!(
            parsed_edition,
            Some(("25th Anniversary Edition".to_string(), 25))
        );

        let (parsed_title, parsed_edition) =
            parse_edition("Title - 2012 Edition, A subtitle", "Tags");
        assert_eq!(parsed_title, "Title: A subtitle");
        assert_eq!(parsed_edition, Some(("2012 Edition".to_string(), 2012)));

        let (parsed_title, parsed_edition) = parse_edition("Title (UK Edition)", "Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("UK Edition".to_string(), 0)));

        let (parsed_title, parsed_edition) = parse_edition("Title (New Edition)", "Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("New Edition".to_string(), 0)));

        let (parsed_title, parsed_edition) = parse_edition("Title: A subtitle 3rd Edition", "Tags");
        assert_eq!(parsed_title, "Title: A subtitle");
        assert_eq!(parsed_edition, Some(("3rd Edition".to_string(), 3)));
    }

    #[test]
    fn test_parse_edition_in_tags() {
        let (parsed_title, parsed_edition) = parse_edition("Title", "Tags (1st edition)");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("1st Edition".to_string(), 1)));

        let (parsed_title, parsed_edition) = parse_edition("Title", "Tags [2nd edition]");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("2nd Edition".to_string(), 2)));

        let (parsed_title, parsed_edition) = parse_edition("Title", "Tags | 3rd Edition");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("3rd Edition".to_string(), 3)));

        let (parsed_title, parsed_edition) = parse_edition("Title", "Tags, 10th Edition");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("10th Edition".to_string(), 10)));

        let (parsed_title, parsed_edition) = parse_edition("Title", "Tags | first edition | more");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("First Edition".to_string(), 1)));

        let (parsed_title, parsed_edition) = parse_edition("Title", "Second edition, Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("Second Edition".to_string(), 2)));

        let (parsed_title, parsed_edition) = parse_edition("Title", "Third Edition Tags");
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("Third Edition".to_string(), 3)));

        let (parsed_title, parsed_edition) = parse_edition(
            "Title",
            "Greenery Press, 3rd ed. edition, March 16, 2016; Illustrated by Barbara O'Toole",
        );
        assert_eq!(parsed_title, "Title");
        assert_eq!(parsed_edition, Some(("3rd Edition".to_string(), 3)));
    }
}
