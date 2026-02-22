use dioxus::prelude::{ReadableExt, Signal, WritableExt};

pub fn apply_click_filter<F: Copy + PartialEq + 'static>(
    filters: &mut Signal<Vec<(F, String)>>,
    field: F,
    value: String,
) {
    let mut next = filters.read().clone();
    next.retain(|(f, _)| *f != field);
    next.push((field, value));
    filters.set(next);
}

#[cfg(feature = "web")]
pub fn parse_location_query_pairs() -> Vec<(String, String)> {
    let Some(window) = web_sys::window() else {
        return Vec::new();
    };
    let Ok(search) = window.location().search() else {
        return Vec::new();
    };
    let search = search.trim_start_matches('?');
    if search.is_empty() {
        return Vec::new();
    }
    search
        .split('&')
        .map(|pair| {
            let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode_query_value(raw_key), decode_query_value(raw_value))
        })
        .collect()
}

#[cfg(not(feature = "web"))]
pub fn parse_location_query_pairs() -> Vec<(String, String)> {
    Vec::new()
}

pub fn build_query_string(params: &[(String, String)]) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(feature = "web")]
fn decode_query_value(value: &str) -> String {
    let replaced = value.replace('+', " ");
    urlencoding::decode(&replaced)
        .map(|s| s.to_string())
        .unwrap_or(replaced)
}
