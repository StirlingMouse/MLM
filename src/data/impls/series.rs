use std::{cmp::Ordering, str::FromStr};

use nom::{
    Finish, IResult, Parser,
    branch::alt,
    character::{complete::char, digit1, multispace0},
    combinator::{complete, map, map_res, opt, recognize},
    multi::separated_list0,
    sequence::delimited,
};

use crate::data::{Series, SeriesEntries, SeriesEntry};

impl TryFrom<(String, String)> for Series {
    type Error = nom::error::Error<String>;

    fn try_from((name, num): (String, String)) -> Result<Self, Self::Error> {
        let entries = SeriesEntries::new(series_entries(&num)?);
        Ok(Series { name, entries })
    }
}

impl SeriesEntries {
    pub fn contains(&self, num: f32) -> bool {
        self.0.iter().any(|s| s.contains(num))
    }
}

impl std::fmt::Display for SeriesEntries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut is_first = true;
        for entry in &self.0 {
            if !is_first {
                write!(f, ", ")?;
            }
            is_first = false;
            write!(f, "{entry}")?;
        }
        Ok(())
    }
}

impl SeriesEntry {
    pub(crate) fn contains(&self, num: f32) -> bool {
        match self {
            SeriesEntry::Num(n) => *n == num,
            SeriesEntry::Range(start, end) => *start <= num && *end >= num,
            SeriesEntry::Part(n, _) => *n == num,
        }
    }
}

impl std::fmt::Display for SeriesEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeriesEntry::Num(num) => write!(f, "{num}"),
            SeriesEntry::Range(start, end) => write!(f, "{start}-{end}"),
            SeriesEntry::Part(entry, part) => write!(f, "{entry}p{part}"),
        }
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for SeriesEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (SeriesEntry::Num(a), SeriesEntry::Num(b)) => a.partial_cmp(b),
            (SeriesEntry::Num(a), SeriesEntry::Range(b, _)) => {
                a.partial_cmp(b).map(|o| o.then(Ordering::Less))
            }
            (SeriesEntry::Num(a), SeriesEntry::Part(b, _)) => {
                a.partial_cmp(b).map(|o| o.then(Ordering::Less))
            }
            (SeriesEntry::Range(a, _), SeriesEntry::Num(b)) => {
                a.partial_cmp(b).map(|o| o.then(Ordering::Greater))
            }
            (SeriesEntry::Range(a, ae), SeriesEntry::Range(b, be)) => a
                .partial_cmp(b)
                .map(|o| o.then(ae.partial_cmp(be).unwrap_or(Ordering::Equal))),
            (SeriesEntry::Range(a, _), SeriesEntry::Part(b, _)) => {
                a.partial_cmp(b).map(|o| o.then(Ordering::Greater))
            }
            (SeriesEntry::Part(a, _), SeriesEntry::Num(b)) => {
                a.partial_cmp(b).map(|o| o.then(Ordering::Greater))
            }
            (SeriesEntry::Part(a, _), SeriesEntry::Range(b, _)) => {
                a.partial_cmp(b).map(|o| o.then(Ordering::Less))
            }
            (SeriesEntry::Part(a, ap), SeriesEntry::Part(b, bp)) => a
                .partial_cmp(b)
                .map(|o| o.then(ap.partial_cmp(bp).unwrap_or(Ordering::Equal))),
        }
    }
}

impl Eq for SeriesEntry {}

impl Ord for SeriesEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

fn float(input: &str) -> IResult<&str, f32> {
    map_res(
        alt((
            recognize((char('.'), decimal)),
            recognize((decimal, char('.'), opt(decimal))),
            recognize(decimal),
        )),
        f32::from_str,
    )
    .parse_complete(input)
}

fn decimal(input: &str) -> IResult<&str, &str> {
    digit1().parse_complete(input)
}

fn series_part(input: &str) -> IResult<&str, SeriesEntry> {
    map(
        (float, multispace0(), char('p'), multispace0(), float),
        |(entry, _, _, _, part)| SeriesEntry::Part(entry, part),
    )
    .parse_complete(input)
}

fn series_range(input: &str) -> IResult<&str, SeriesEntry> {
    map(
        (float, multispace0(), char('-'), multispace0(), float),
        |(start, _, _, _, end)| SeriesEntry::Range(start, end),
    )
    .parse_complete(input)
}

fn series_num(input: &str) -> IResult<&str, SeriesEntry> {
    map(float, SeriesEntry::Num).parse_complete(input)
}

fn series_entry(input: &str) -> IResult<&str, SeriesEntry> {
    alt((series_part, series_range, series_num)).parse_complete(input)
}

fn series_entries(input: &str) -> Result<Vec<SeriesEntry>, nom::error::Error<&str>> {
    if input.is_empty() {
        return Ok(vec![]);
    }
    complete(separated_list0(
        char(','),
        delimited(opt(multispace0()), series_entry, opt(multispace0())),
    ))
    .parse_complete(input)
    .finish()
    .map(|(_, entries)| entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_float() {
        assert_eq!(decimal("12"), Ok(("", "12")));
        assert_eq!(decimal("1"), Ok(("", "1")));
        assert_eq!(float("1"), Ok(("", 1.0)));
        assert_eq!(float("01"), Ok(("", 1.0)));
        assert_eq!(float("01."), Ok(("", 1.0)));
        assert_eq!(float("01.0"), Ok(("", 1.0)));
        assert_eq!(float(".5"), Ok(("", 0.5)));
    }

    #[test]
    fn test_parse_series_num() {
        assert_eq!(series_num("1"), Ok(("", SeriesEntry::Num(1.0))));
        assert_eq!(series_num("01"), Ok(("", SeriesEntry::Num(1.0))));
        assert_eq!(series_num("01."), Ok(("", SeriesEntry::Num(1.0))));
        assert_eq!(series_num("01.0"), Ok(("", SeriesEntry::Num(1.0))));
        assert_eq!(series_num(".5"), Ok(("", SeriesEntry::Num(0.5))));
    }

    #[test]
    fn test_parse_series_range() {
        assert_eq!(series_range("1-2"), Ok(("", SeriesEntry::Range(1.0, 2.0))));
        assert_eq!(
            series_range("01-10"),
            Ok(("", SeriesEntry::Range(1.0, 10.0)))
        );
        assert_eq!(
            series_range("01.-3.52"),
            Ok(("", SeriesEntry::Range(1.0, 3.52)))
        );
        assert_eq!(
            series_range("01.0-.5"),
            Ok(("", SeriesEntry::Range(1.0, 0.5)))
        );
        assert_eq!(
            series_range(".5 - 32."),
            Ok(("", SeriesEntry::Range(0.5, 32.)))
        );
    }

    #[test]
    fn test_parse_series_part() {
        assert_eq!(series_part("1p2"), Ok(("", SeriesEntry::Part(1.0, 2.0))));
        assert_eq!(series_part("01p10"), Ok(("", SeriesEntry::Part(1.0, 10.0))));
        assert_eq!(
            series_part("01.p3.52"),
            Ok(("", SeriesEntry::Part(1.0, 3.52)))
        );
        assert_eq!(
            series_part("01.0p.5"),
            Ok(("", SeriesEntry::Part(1.0, 0.5)))
        );
        assert_eq!(
            series_part(".5 p 32."),
            Ok(("", SeriesEntry::Part(0.5, 32.)))
        );
    }

    #[test]
    fn test_parse_series_entires() {
        assert_eq!(series_entries("1"), Ok(vec![SeriesEntry::Num(1.0)]));
        assert_eq!(
            series_entries("1-2"),
            Ok(vec![SeriesEntry::Range(1.0, 2.0)])
        );
        assert_eq!(series_entries("1p2"), Ok(vec![SeriesEntry::Part(1.0, 2.0)]));
        assert_eq!(
            series_entries("1,1p2,1-2"),
            Ok(vec![
                SeriesEntry::Num(1.0),
                SeriesEntry::Part(1.0, 2.0),
                SeriesEntry::Range(1.0, 2.0)
            ])
        );
        assert_eq!(
            series_entries("1p2,1,1p1,2,1-2").map(|mut s| {
                s.sort();
                s
            }),
            Ok(vec![
                SeriesEntry::Num(1.0),
                SeriesEntry::Part(1.0, 1.0),
                SeriesEntry::Part(1.0, 2.0),
                SeriesEntry::Range(1.0, 2.0),
                SeriesEntry::Num(2.0),
            ])
        );
    }
}
