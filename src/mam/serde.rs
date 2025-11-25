use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use time::format_description::{self, OwnedFormatItem};

pub static DATE_FORMAT: Lazy<OwnedFormatItem> =
    Lazy::new(|| format_description::parse_owned::<2>("[year]-[month]-[day]").unwrap());

pub static DATE_TIME_FORMAT: Lazy<OwnedFormatItem> = Lazy::new(|| {
    format_description::parse_owned::<2>("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap()
});

pub fn is_false(value: &bool) -> bool {
    !value
}

pub fn is_zero(value: &u64) -> bool {
    *value == 0
}

pub fn bool_string_or_number<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Value::deserialize(deserializer)?;
    match v {
        Value::String(v) => Ok(v == "yes" || v == "1"),
        Value::Number(v) => Ok(v.as_u64().unwrap_or_default() == 1),
        _ => Err(serde::de::Error::custom("expected number or string")),
    }
}

pub fn opt_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<Value>::deserialize(deserializer)?;
    match v {
        Some(Value::String(v)) => Ok(Some(v)),
        Some(Value::Number(v)) => Ok(Some(v.to_string())),
        None => Ok(None),
        _ => Err(serde::de::Error::custom("expected number or string")),
    }
}

pub fn vec_string_or_number<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Vec::<Value>::deserialize(deserializer)?;
    v.into_iter()
        .filter_map(|v| match v {
            Value::String(v) => Some(Ok(v)),
            Value::Number(v) => Some(Ok(v.to_string())),
            Value::Null => None,
            _ => Some(Err(serde::de::Error::custom("expected number or string"))),
        })
        .collect()
}

pub fn string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Value::deserialize(deserializer)?;
    match v {
        Value::String(v) => Ok(v),
        Value::Number(v) => Ok(v.to_string()),
        _ => Err(serde::de::Error::custom("expected number or string")),
    }
}

pub static TITLE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(.*?) +\[[^\]]*\]$").unwrap());

// Workaround policy to put english translations in torrent titles
pub fn parse_title<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let title = string_or_number(deserializer)?;

    if let Some(title) = TITLE_PATTERN.captures(&title).and_then(|c| c.get(1)) {
        Ok(title.as_str().to_owned())
    } else {
        Ok(title)
    }
}

pub fn json_or_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: Deserializer<'de>,
{
    let v: Result<Value, _> = serde_nested_json::deserialize(deserializer);
    let Ok(v) = v else {
        return Ok(T::default());
    };
    Ok(T::deserialize(v).unwrap_or_default())
}
