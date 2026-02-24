#[cfg(feature = "server")]
use time::UtcOffset;

#[cfg(feature = "server")]
const DATETIME_FORMAT: &str = "[year]-[month]-[day] [hour]:[minute]:[second]";

#[cfg(feature = "server")]
pub fn format_timestamp(ts: &mlm_core::Timestamp) -> String {
    let format = time::format_description::parse(DATETIME_FORMAT).expect("format is valid");
    ts.0.to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC))
        .replace_nanosecond(0)
        .unwrap_or_else(|_| ts.0.into())
        .format(&format)
        .unwrap_or_default()
}

#[cfg(feature = "server")]
pub(crate) trait DbTimeValue {
    fn as_timestamp(&self) -> Option<&mlm_db::Timestamp>;
}

#[cfg(feature = "server")]
impl<T: DbTimeValue + ?Sized> DbTimeValue for &T {
    fn as_timestamp(&self) -> Option<&mlm_db::Timestamp> {
        (*self).as_timestamp()
    }
}

#[cfg(feature = "server")]
impl DbTimeValue for mlm_db::Timestamp {
    fn as_timestamp(&self) -> Option<&mlm_db::Timestamp> {
        Some(self)
    }
}

#[cfg(feature = "server")]
impl DbTimeValue for Option<mlm_db::Timestamp> {
    fn as_timestamp(&self) -> Option<&mlm_db::Timestamp> {
        self.as_ref()
    }
}

#[cfg(feature = "server")]
pub(crate) fn format_timestamp_db<T: DbTimeValue>(ts: &T) -> String {
    let Some(ts) = ts.as_timestamp() else {
        return String::new();
    };
    let format = time::format_description::parse(DATETIME_FORMAT).expect("format is valid");
    let dt =
        ts.0.to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC));
    dt.replace_nanosecond(0)
        .unwrap_or(dt)
        .format(&format)
        .unwrap_or_default()
}

#[cfg(feature = "server")]
pub fn format_datetime(dt: &time::OffsetDateTime) -> String {
    let format = time::format_description::parse(DATETIME_FORMAT).expect("format is valid");
    dt.to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC))
        .replace_nanosecond(0)
        .unwrap_or(*dt)
        .format(&format)
        .unwrap_or_default()
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
