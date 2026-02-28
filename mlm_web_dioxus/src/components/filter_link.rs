use dioxus::prelude::*;
use serde::Serialize;

use super::query_params::{
    build_location_href, build_query_string, encode_query_enum, parse_location_query_pairs,
};

pub fn filter_href<F: Copy + PartialEq + Serialize + 'static>(
    field: F,
    value: String,
    reset_from: bool,
    current_params: Option<Vec<(String, String)>>,
) -> String {
    if let Some(name) = encode_query_enum(field) {
        let mut params = current_params.unwrap_or_else(parse_location_query_pairs);
        params.retain(|(key, _)| key != &name && !(reset_from && key == "from"));
        params.push((name, value));
        let query_string = build_query_string(&params);
        build_location_href(&query_string)
    } else {
        build_location_href("")
    }
}

#[component]
pub fn TorrentTitleLink<F: Copy + PartialEq + Serialize + 'static>(
    detail_id: String,
    field: F,
    value: String,
    children: Element,
    #[props(default = false)] reset_from: bool,
    #[props(default = None)] current_params: Option<Vec<(String, String)>>,
) -> Element {
    let title_filter_href = filter_href(field, value, reset_from, current_params);

    rsx! {
        a {
            class: "link",
            href: "/dioxus/torrents/{detail_id}",
            onclick: move |ev: MouseEvent| {
                if ev.modifiers().alt() {
                    ev.prevent_default();
                    #[cfg(feature = "web")]
                    {
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().set_href(&title_filter_href);
                        }
                    }
                    #[cfg(not(feature = "web"))]
                    {
                        let _ = &title_filter_href;
                    }
                }
            },
            {children}
        }
    }
}

#[component]
pub fn FilterLink<F: Copy + PartialEq + Serialize + 'static>(
    field: F,
    value: String,
    children: Element,
    #[props(default = false)] reset_from: bool,
    #[props(default = None)] title: Option<String>,
    /// Pre-parsed query pairs from the parent. When provided, avoids a redundant
    /// `parse_location_query_pairs()` call on every render (e.g. inside row loops).
    #[props(default = None)]
    current_params: Option<Vec<(String, String)>>,
) -> Element {
    let href = filter_href(field, value.clone(), reset_from, current_params);

    rsx! {
        Link {
            class: "link",
            to: href,
            title: title.unwrap_or_default(),
            {children}
        }
    }
}
