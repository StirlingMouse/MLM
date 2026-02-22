#[cfg(feature = "server")]
use time::UtcOffset;

#[cfg(feature = "server")]
pub fn format_timestamp(ts: &mlm_core::Timestamp) -> String {
    let format = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
        .expect("format is valid");
    ts.0.to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC))
        .replace_nanosecond(0)
        .unwrap_or_else(|_| ts.0.into())
        .format(&format)
        .unwrap_or_default()
}

#[cfg(feature = "server")]
pub fn format_timestamp_db(ts: &mlm_db::Timestamp) -> String {
    let dt: time::OffsetDateTime = ts.0.into();
    let format = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
        .expect("format is valid");
    dt.replace_nanosecond(0)
        .unwrap_or(dt)
        .format(&format)
        .unwrap_or_default()
}

#[cfg(feature = "server")]
pub fn format_datetime(dt: &time::OffsetDateTime) -> String {
    let format = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
        .expect("format is valid");
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

pub fn path_to_string(path: &std::path::Path) -> String {
    path.to_string_lossy().to_string()
}
