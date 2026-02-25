use dioxus::prelude::*;
use serde::Serialize;

use super::query_params::{
    build_location_href, build_query_string, encode_query_enum, parse_location_query_pairs,
};

#[component]
pub fn FilterLink<F: Copy + PartialEq + Serialize + 'static>(
    filters: Signal<Vec<(F, String)>>,
    field: F,
    value: String,
    children: Element,
    #[props(default = false)] reset_from: bool,
    #[props(default = None)] title: Option<String>,
    #[props(default = None)] on_apply: Option<EventHandler<()>>,
) -> Element {
    let _ = filters;
    let _ = on_apply;
    let href = if let Some(name) = encode_query_enum(field) {
        let mut params = parse_location_query_pairs();
        params.retain(|(key, _)| key != &name && !(reset_from && key == "from"));
        params.push((name, value.clone()));
        let query_string = build_query_string(&params);
        build_location_href(&query_string)
    } else {
        build_location_href("")
    };

    rsx! {
        Link {
            class: "link",
            to: href,
            title: title.unwrap_or_default(),
            {children}
        }
    }
}
