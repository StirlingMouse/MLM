use std::str::FromStr;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::Size;

impl Size {
    pub fn unit(self) -> u64 {
        if self.bytes() > 0 { 1 } else { 0 }
    }
}

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value = self.bytes() as f64;
        let mut unit = "B";
        if value > 1024_f64.powf(4.0) {
            value /= 1024_f64.powf(4.0);
            unit = "TiB";
        } else if value > 1024_f64.powf(3.0) {
            value /= 1024_f64.powf(3.0);
            unit = "GiB";
        } else if value > 1024_f64.powf(2.0) {
            value /= 1024_f64.powf(2.0);
            unit = "MiB";
        } else if value > 1024.0 {
            value /= 1024.0;
            unit = "KiB";
        }
        let value = ((value * 100.0).round() as u64) as f64 / 100.0;
        write!(f, "{} {}", value, unit)
    }
}

pub static SIZE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^((?:\d{1,3},)?\d{1,6}(?:\.\d{1,3})?) ([kKMGT]?)(i)?B$").unwrap());

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
                "T" => base.pow(4),
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

    #[test]
    fn test_size_display() {
        assert_eq!(
            format!("{}", Size::from_str("1.43 GiB").unwrap()),
            "1.43 GiB"
        );
    }
}
