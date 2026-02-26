use dioxus::prelude::{ReadableExt, Signal, WritableExt};
use serde::Serialize;
use serde::de::DeserializeOwned;

pub trait PageColumns: Default + PartialEq + Sized {
    fn to_query_value(&self) -> String;
    fn from_query_value(s: &str) -> Self;
}

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
    #[cfg(feature = "server")]
    {
        let Some(context) = dioxus_fullstack::FullstackContext::current() else {
            return Vec::new();
        };
        let parts = context.parts_mut();
        let Some(search) = parts.uri.query() else {
            return Vec::new();
        };
        if search.is_empty() {
            return Vec::new();
        }
        return search
            .split('&')
            .map(|pair| {
                let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
                (decode_query_value(raw_key), decode_query_value(raw_value))
            })
            .collect();
    }
    #[cfg(not(feature = "server"))]
    {
        Vec::new()
    }
}

pub fn build_query_string(params: &[(String, String)]) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

pub fn build_location_href(query_string: &str) -> String {
    let pathname = location_pathname();
    if query_string.is_empty() {
        pathname
    } else {
        format!("{pathname}?{query_string}")
    }
}

pub fn parse_query_enum<T: DeserializeOwned>(value: &str) -> Option<T> {
    serde_json::from_str::<T>(&format!("\"{value}\"")).ok()
}

pub fn encode_query_enum<T: Serialize>(value: T) -> Option<String> {
    serde_json::to_string(&value)
        .ok()
        .map(|raw| raw.trim_matches('"').to_string())
}

#[cfg(feature = "web")]
pub fn set_location_query_string(query_string: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let target = build_location_href(query_string);
    let Ok(history) = window.history() else {
        return;
    };
    let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&target));
}

#[cfg(not(feature = "web"))]
pub fn set_location_query_string(_query_string: &str) {}

#[cfg(any(feature = "web", feature = "server"))]
fn decode_query_value(value: &str) -> String {
    let replaced = value.replace('+', " ");
    urlencoding::decode(&replaced)
        .map(|s| s.to_string())
        .unwrap_or(replaced)
}

#[cfg(feature = "web")]
fn location_pathname() -> String {
    let Some(window) = web_sys::window() else {
        return "/".to_string();
    };
    window
        .location()
        .pathname()
        .unwrap_or_else(|_| "/".to_string())
}

#[cfg(not(feature = "web"))]
fn location_pathname() -> String {
    #[cfg(feature = "server")]
    {
        let Some(context) = dioxus_fullstack::FullstackContext::current() else {
            return "/".to_string();
        };
        return context.parts_mut().uri.path().to_string();
    }
    #[cfg(not(feature = "server"))]
    {
        "/".to_string()
    }
}
