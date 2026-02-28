use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use time::{
    Date,
    format_description::{self, OwnedFormatItem},
};

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

pub fn num_string_or_number<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: TryFrom<u64>,
{
    let v = Value::deserialize(deserializer)?;
    match v {
        Value::String(v) => Ok(v
            .parse::<u64>()
            .map_err(|_| serde::de::Error::custom("invalid number string"))
            .and_then(|v| {
                v.try_into()
                    .map_err(|_| serde::de::Error::custom("invalid number string"))
            })?),
        Value::Number(v) => Ok(v
            .as_u64()
            .ok_or_else(|| serde::de::Error::custom("invalid number"))
            .and_then(|v| {
                v.try_into()
                    .map_err(|_| serde::de::Error::custom("invalid number"))
            })?),
        _ => Err(serde::de::Error::custom("expected number or string")),
    }
}

pub fn opt_num_string_or_number<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: TryFrom<u64>,
{
    let v = Option::<Value>::deserialize(deserializer)?;
    match v {
        Some(Value::String(v)) => Ok(Some(
            v.parse::<u64>()
                .map_err(|_| serde::de::Error::custom("invalid number string"))
                .and_then(|v| {
                    v.try_into()
                        .map_err(|_| serde::de::Error::custom("invalid number string"))
                })?,
        )),
        Some(Value::Number(v)) => Ok(Some(
            v.as_u64()
                .ok_or_else(|| serde::de::Error::custom("invalid number"))
                .and_then(|v| {
                    v.try_into()
                        .map_err(|_| serde::de::Error::custom("invalid number"))
                })?,
        )),
        None => Ok(None),
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

pub fn parse_opt_date<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<String> = Deserialize::deserialize(deserializer)?;
    v.map(|v| Date::parse(&v, &DATE_FORMAT).map_err(serde::de::Error::custom))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct BoolWrap {
        #[serde(deserialize_with = "bool_string_or_number")]
        val: bool,
    }

    #[test]
    fn test_bool_string_or_number_variants() {
        assert!(
            serde_json::from_str::<BoolWrap>(r#"{"val":"yes"}"#)
                .unwrap()
                .val
        );
        assert!(
            serde_json::from_str::<BoolWrap>(r#"{"val":"1"}"#)
                .unwrap()
                .val
        );
        assert!(
            serde_json::from_str::<BoolWrap>(r#"{"val":1}"#)
                .unwrap()
                .val
        );
        assert!(
            !serde_json::from_str::<BoolWrap>(r#"{"val":"no"}"#)
                .unwrap()
                .val
        );
        assert!(
            !serde_json::from_str::<BoolWrap>(r#"{"val":"0"}"#)
                .unwrap()
                .val
        );
        assert!(
            !serde_json::from_str::<BoolWrap>(r#"{"val":0}"#)
                .unwrap()
                .val
        );
        // boolean true/false should be rejected by this deserializer
        assert!(serde_json::from_str::<BoolWrap>(r#"{"val": true}"#).is_err());
    }

    #[derive(Deserialize)]
    struct NumWrap {
        #[serde(deserialize_with = "num_string_or_number")]
        val: u32,
    }

    #[test]
    fn test_num_string_or_number() {
        assert_eq!(
            serde_json::from_str::<NumWrap>(r#"{"val":"42"}"#)
                .unwrap()
                .val,
            42
        );
        assert_eq!(
            serde_json::from_str::<NumWrap>(r#"{"val":42}"#)
                .unwrap()
                .val,
            42
        );
        assert!(serde_json::from_str::<NumWrap>(r#"{"val":"notanumber"}"#).is_err());
    }

    #[derive(Deserialize)]
    struct OptNumWrap {
        #[serde(deserialize_with = "opt_num_string_or_number")]
        val: Option<u64>,
    }

    #[test]
    fn test_opt_num_and_string_or_number() {
        assert_eq!(
            serde_json::from_str::<OptNumWrap>(r#"{"val":"100"}"#)
                .unwrap()
                .val,
            Some(100)
        );
        assert_eq!(
            serde_json::from_str::<OptNumWrap>(r#"{"val":100}"#)
                .unwrap()
                .val,
            Some(100)
        );
        assert_eq!(
            serde_json::from_str::<OptNumWrap>(r#"{"val":null}"#)
                .unwrap()
                .val,
            None
        );

        #[derive(Deserialize)]
        struct OptStrWrap {
            #[serde(deserialize_with = "opt_string_or_number")]
            val: Option<String>,
        }

        assert_eq!(
            serde_json::from_str::<OptStrWrap>(r#"{"val":"abc"}"#)
                .unwrap()
                .val,
            Some("abc".to_string())
        );
        assert_eq!(
            serde_json::from_str::<OptStrWrap>(r#"{"val":123}"#)
                .unwrap()
                .val,
            Some("123".to_string())
        );
        assert_eq!(
            serde_json::from_str::<OptStrWrap>(r#"{"val":null}"#)
                .unwrap()
                .val,
            None
        );
    }

    #[derive(Deserialize)]
    struct VecWrap {
        #[serde(deserialize_with = "vec_string_or_number")]
        val: Vec<String>,
    }

    #[test]
    fn test_vec_string_or_number() {
        let v: VecWrap = serde_json::from_str(r#"{"val":["1", 2, null]}"#).unwrap();
        assert_eq!(v.val, vec!["1".to_string(), "2".to_string()]);
        // invalid element type (object) should error
        assert!(serde_json::from_str::<VecWrap>(r#"{"val":[{}]}"#).is_err());
    }

    #[derive(Deserialize)]
    struct StrWrap {
        #[serde(deserialize_with = "string_or_number")]
        val: String,
    }

    #[test]
    fn test_string_or_number() {
        assert_eq!(
            serde_json::from_str::<StrWrap>(r#"{"val":"abc"}"#)
                .unwrap()
                .val,
            "abc"
        );
        assert_eq!(
            serde_json::from_str::<StrWrap>(r#"{"val":123}"#)
                .unwrap()
                .val,
            "123"
        );
        assert!(serde_json::from_str::<StrWrap>(r#"{"val": null}"#).is_err());
    }

    #[derive(Deserialize)]
    struct DateWrap {
        #[serde(deserialize_with = "parse_opt_date")]
        d: Option<Date>,
    }

    #[test]
    fn test_parse_opt_date() {
        let ok = serde_json::from_str::<DateWrap>(r#"{"d":"2023-01-02"}"#).unwrap();
        assert!(ok.d.is_some());
        let none = serde_json::from_str::<DateWrap>(r#"{"d":null}"#).unwrap();
        assert!(none.d.is_none());
        // invalid date should error
        assert!(serde_json::from_str::<DateWrap>(r#"{"d":"not-a-date"}"#).is_err());
    }
}
