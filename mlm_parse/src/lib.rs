mod html;

use anyhow::Result;
use htmlentity::entity::{self, ICodedDataTrait as _};
use once_cell::sync::Lazy;
use regex::{Captures, Match, Regex};
use unidecode::unidecode;

use crate::html::AMMONIA;

pub fn clean_value(value: &str) -> Result<String> {
    entity::decode(value.as_bytes()).to_string()
}

pub fn clean_html(value: &str) -> String {
    AMMONIA.clean(value).to_string()
}

pub fn normalize_title(value: &str) -> String {
    let title = unidecode(value).to_lowercase().replace(" & ", " and ");
    let title = SEARCH_TITLE_CLEANUP.replace_all(&title, "");
    SEARCH_TITLE_VOLUME.replace_all(&title, "").to_string()
}

pub fn clean_name(name: &mut String) -> Result<()> {
    *name = clean_value(name)?;
    let n = NAME_CLEANUP.replace_all(name, " ");
    *name = NAME_INITIALS
        .replace_all(&n, |captures: &Captures| {
            format!("{} {}", &captures[1], &captures[2])
        })
        .to_string();

    let mut to_lowercase = vec![];
    let mut to_uppercase = vec![];
    let mut start = 0;

    let mut check_word = |start, end| {
        let word = &name.get(start..end);
        if let Some(word) = word
            && word.len() > 3
            && word.chars().all(|c| c.is_uppercase())
        {
            to_lowercase.push((start + 1)..end);
        } else if let Some(word) = word
            && word.len() > 3
            && word.chars().all(|c| c.is_lowercase())
        {
            to_uppercase.push(start);
        }
    };
    for (i, char) in name.char_indices() {
        if char == ' ' {
            check_word(start, i);
            start = i + 1;
        }
    }
    check_word(start, name.len());

    for range in to_lowercase {
        let Some(part) = name.get(range.clone()) else {
            continue;
        };
        name.replace_range(range, &part.to_lowercase());
    }
    for i in to_uppercase {
        let Some(part) = name.get(i..=i) else {
            continue;
        };
        name.replace_range(i..=i, &part.to_uppercase());
    }

    Ok(())
}

static EDITION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(.*?)(?:(?:(?:\s*[-–.:;|,]\s*)((\w+?)\s+(?:[a-z]+\s+)*(?:Edition|ed\.|utgåva))|(?:\s*[-–.:;|,]\s*)?(?:\s*[(\[]\s*)((\w*?)\s+(?:[a-z]+\s+)*(?:Edition|ed\.|utgåva))(?:\s*[)\]]\s*))(?:\s*[-:;,]\s*)?(.*?)|\s+((\d+\w*?)\s+(?:Edition|ed\.|utgåva)))$").unwrap()
});

static EDITION_START_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)((\d+(?:st|nd|rd|th)|first|second|third|fifth|sixth|seventh|eight|ninth|tenth|new|revised|updated)\s+(?:[a-z']+\s+)*(?:Edition|ed\.|utgåva)|(\w+?)\s+(?:Edition|ed\.|utgåva))").unwrap()
});

pub static TITLE_CLEANUP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(?:: A (?:Novel|Memoir)$)|(?:: An? (?:\w+ )(?:(?:Fantasy|LitRPG(?:/Gamelit)?) Adventure)$)|(?:: (?:An?|Stand-?alone) (?:[\w!'/]+ ){1,4}(?:Romance|LitRPG|Novella|Anthology)$)|(?:: light novel)|(?:\s*-\s*\d+(?:\.| - )epub$)|(?:\s*[\(\[]\.?(?:digital|light novel|epub|pdf|cbz|cbr|mp3|m4b|tpb|fixed|unabridged|Dramatized Adaptation|full cast)[\)\]])*",
    )
    .unwrap()
});

pub static TITLE_SERIES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?: \((.+), (?:Book|Vol\.) (\d+(?:\.\d+)?|[IXV]+|one|two|three|four|five|six|seven|eight|nine|ten)\)|: (.+), (?:Book|Vol\.) (\d+(?:\.\d+)?|one|two|three|four|five|six|seven|eight|nine|ten)|: ([\w\s]+) (\d+)|: Book (\d+(?:\.\d+)?|[IXV]+|one|two|three|four|five|six|seven|eight|nine|ten) (?:of|in) the ([\w\s!']+)|: An? ([\w\s]+) (?:Standalone|Novel)(?:, (?:Book |Vol\. )?(\d+(?:\.\d+)?|[IXV]+|one|two|three|four|five|six|seven|eight|nine|ten))?|: ([\w\s!']+) (?:collection))$").unwrap()
});

static SEARCH_TITLE_CLEANUP: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(?:the|a|an)\s+|[^\w ]").unwrap());

static SEARCH_TITLE_VOLUME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?:volume|vol\.)").unwrap());

pub static NAME_CLEANUP: Lazy<Regex> = Lazy::new(|| Regex::new(r"\.\s*").unwrap());
pub static NAME_INITIALS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b([A-Z])([A-Z])\b").unwrap());
pub static NAME_ROLES: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i) - (?:translator|foreword|introduction|afterword)").unwrap());

pub static SERIES_NAME_CLEANUP: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(?:the|a)\s+(.+)\s+(?:series|novel)$").unwrap());
pub static SERIES_CLEANUP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:\s*\((?:digital|light novel)\))+|\s+series$|^A LitRPG Adventure: ").unwrap()
});

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

pub fn parse_series_from_title(title: &str) -> Option<(&str, Option<f32>)> {
    if let Some(captures) = TITLE_SERIES.captures(title) {
        let series_title = captures
            .get(1)
            .or(captures.get(3))
            .or(captures.get(5))
            .or(captures.get(8))
            .or(captures.get(9))
            .or(captures.get(11))?;
        let series_number = captures
            .get(2)
            .or(captures.get(4))
            .or(captures.get(6))
            .or(captures.get(7))
            .or(captures.get(11))?;
        let series_number = match series_number.as_str().to_lowercase().as_str() {
            "one" | "i" => 1.0,
            "two" | "ii" => 2.0,
            "three" | "iii" => 3.0,
            "four" | "iv" => 4.0,
            "five" | "v" => 5.0,
            "six" | "vi" => 6.0,
            "seven" | "vii" => 7.0,
            "eight" | "viii" => 8.0,
            "nine" | "ix" => 9.0,
            "ten" | "x" => 10.0,
            n => n.parse().unwrap_or(-1.0),
        };
        let series_number = if series_number >= 0.0 {
            Some(series_number)
        } else {
            None
        };
        return Some((series_title.as_str(), series_number));
    }
    None
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

    #[test]
    fn test_parse_series_from_title_libation_order_pattern() {
        let parsed =
            parse_series_from_title("The Order: Kingdom of Fallen Ash: The Order Series, Book 1");
        assert_eq!(
            parsed,
            Some(("Kingdom of Fallen Ash: The Order Series", Some(1.0)))
        );

        let parsed = parse_series_from_title(
            "The Order: Labyrinth of Twisted Games: The Order Series, Book 2",
        );
        assert_eq!(
            parsed,
            Some(("Labyrinth of Twisted Games: The Order Series", Some(2.0)))
        );
    }

    #[test]
    fn test_parse_series_from_title_does_not_match_without_series_suffix() {
        let parsed = parse_series_from_title("The Order: Kingdom of Fallen Ash");
        assert_eq!(parsed, None);
    }
}
