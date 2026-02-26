use std::collections::BTreeSet;
use std::sync::Arc;

use crate::components::{
    ActiveFilterChip, ActiveFilters, FilterLink, SortHeader, TorrentGridTable,
    set_location_query_string,
};
use crate::sse::ERRORS_UPDATE_TRIGGER;
use dioxus::prelude::*;

use super::server_fns::*;
use super::types::*;

#[component]
pub fn ErrorsPage() -> Element {
    let _route: crate::app::Route = use_route();
    let initial_state = parse_query_state();
    let initial_sort = initial_state.sort;
    let initial_asc = initial_state.asc;
    let initial_filters = initial_state.filters.clone();
    let initial_request_key = build_query_url(
        initial_state.sort,
        initial_state.asc,
        &initial_state.filters,
    );

    let sort = use_signal(move || initial_sort);
    let asc = use_signal(move || initial_asc);
    let filters = use_signal(move || initial_filters.clone());
    let mut selected = use_signal(BTreeSet::<String>::new);
    let mut last_selected_idx = use_signal(|| None::<usize>);
    let mut status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached = use_signal(|| None::<ErrorsData>);
    let loading_action = use_signal(|| false);
    let mut last_request_key = use_signal(move || initial_request_key.clone());
    let from = use_signal(|| 0usize);

    let mut errors_data = match use_server_future(move || async move {
        get_errors_data(*sort.read(), *asc.read(), filters.read().clone()).await
    }) {
        Ok(resource) => resource,
        Err(_) => {
            return rsx! {
                div { class: "errors-page",
                    h1 { "Torrent Errors" }
                    p { "Loading errors..." }
                }
            };
        }
    };

    let value = errors_data.value();
    let pending = errors_data.pending();

    {
        let route_state = parse_query_state();
        let route_request_key =
            build_query_url(route_state.sort, route_state.asc, &route_state.filters);
        if *last_request_key.read() != route_request_key {
            let mut sort = sort;
            let mut asc = asc;
            let mut filters_signal = filters;
            sort.set(route_state.sort);
            asc.set(route_state.asc);
            filters_signal.set(route_state.filters);
            last_request_key.set(route_request_key);
            errors_data.restart();
        }
    }

    {
        let value = value.read();
        if let Some(Ok(data)) = &*value {
            cached.set(Some(data.clone()));
        }
    }

    use_effect(move || {
        let _ = *ERRORS_UPDATE_TRIGGER.read();
        errors_data.restart();
    });

    let data_to_show = {
        let value = value.read();
        match &*value {
            Some(Ok(data)) => Some(data.clone()),
            _ => cached.read().clone(),
        }
    };

    use_effect(move || {
        let sort = *sort.read();
        let asc = *asc.read();
        let filters = filters.read().clone();
        let query_string = build_query_url(sort, asc, &filters);
        let should_restart = *last_request_key.read() != query_string;
        if should_restart {
            last_request_key.set(query_string.clone());
            set_location_query_string(&query_string);
            errors_data.restart();
        }
    });

    let mut active_chips = Vec::new();
    for (field, value) in filters.read().clone() {
        active_chips.push(ActiveFilterChip {
            label: format!("{}: {}", filter_name(field), value),
            on_remove: Callback::new({
                let value = value.clone();
                let mut filters = filters;
                move |_| {
                    filters
                        .write()
                        .retain(|(f, v)| !(*f == field && *v == value));
                }
            }),
        });
    }

    let clear_all: Option<Callback<()>> = if active_chips.is_empty() {
        None
    } else {
        Some(Callback::new({
            let mut filters = filters;
            move |_| filters.set(Vec::new())
        }))
    };

    let all_row_ids = Arc::new(
        data_to_show
            .as_ref()
            .map(|data| {
                data.errors
                    .iter()
                    .map(|e| e.id_json.clone())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default(),
    );

    rsx! {
        div { class: "errors-page",
            div { class: "row",
                h1 { "Torrent Errors" }
                p { "Errors encountered while grabbing, linking, or cleaning torrents" }
            }

            if let Some((msg, is_error)) = status_msg.read().as_ref() {
                p { class: if *is_error { "error" } else { "loading-indicator" },
                    "{msg}"
                    button {
                        r#type: "button",
                        style: "margin-left: 10px; cursor: pointer;",
                        onclick: move |_| status_msg.set(None),
                        "⨯"
                    }
                }
            }

            ActiveFilters {
                chips: active_chips,
                on_clear_all: clear_all,
            }

            if let Some(data) = data_to_show {
                if data.errors.is_empty() {
                    p {
                        i { "There are currently no errors" }
                    }
                } else {
                    div { class: "actions actions_error",
                        style: if selected.read().is_empty() { "" } else { "display: flex" },
                        button {
                            r#type: "button",
                            disabled: *loading_action.read(),
                            onclick: {
                                let mut loading_action = loading_action;
                                let mut status_msg = status_msg;
                                let mut errors_data = errors_data;
                                let mut selected = selected;
                                move |_| {
                                    let ids = selected.read().iter().cloned().collect::<Vec<_>>();
                                    if ids.is_empty() {
                                        status_msg.set(Some(("Select at least one error".to_string(), true)));
                                        return;
                                    }
                                    loading_action.set(true);
                                    status_msg.set(None);
                                    spawn(async move {
                                        match remove_errors_action(ids).await {
                                            Ok(_) => {
                                                status_msg.set(Some(("Removed errors".to_string(), false)));
                                                selected.set(BTreeSet::new());
                                                errors_data.restart();
                                            }
                                            Err(e) => {
                                                status_msg.set(Some((format!("remove failed: {e}"), true)));
                                            }
                                        }
                                        loading_action.set(false);
                                    });
                                }
                            },
                            "remove"
                        }
                    }

                    TorrentGridTable {
                        grid_template: "30px 100px 1fr 1fr 157px 88px".to_string(),
                        extra_class: Some("ErrorsTable".to_string()),
                        pending: pending && cached.read().is_some(),
                        {
                            let all_selected = data.errors.iter().all(|e| selected.read().contains(&e.id_json));
                            rsx! {
                                div { class: "torrents-grid-row",
                                    div { class: "header",
                                        input {
                                            r#type: "checkbox",
                                            checked: all_selected,
                                            onchange: {
                                                let row_ids = data.errors.iter().map(|e| e.id_json.clone()).collect::<Vec<_>>();
                                                move |ev| {
                                                    if ev.value() == "true" {
                                                        let mut next = selected.read().clone();
                                                        for id in &row_ids {
                                                            next.insert(id.clone());
                                                        }
                                                        selected.set(next);
                                                    } else {
                                                        let mut next = selected.read().clone();
                                                        for id in &row_ids {
                                                            next.remove(id);
                                                        }
                                                        selected.set(next);
                                                    }
                                                }
                                            },
                                        }
                                    }
                                    SortHeader { label: "Step".to_string(), sort_key: ErrorsPageSort::Step, sort, asc, from }
                                    SortHeader { label: "Title".to_string(), sort_key: ErrorsPageSort::Title, sort, asc, from }
                                    SortHeader { label: "Error".to_string(), sort_key: ErrorsPageSort::Error, sort, asc, from }
                                    SortHeader { label: "When".to_string(), sort_key: ErrorsPageSort::CreatedAt, sort, asc, from }
                                    div { class: "header", "" }
                                }
                            }
                        }

                        for (i, error) in data.errors.into_iter().enumerate() {
                            {
                                let row_id = error.id_json.clone();
                                let row_selected = selected.read().contains(&row_id);
                                let all_row_ids = all_row_ids.clone();
                                rsx! {
                                    div { class: "torrents-grid-row", key: "{row_id}",
                                        div {
                                            input {
                                                r#type: "checkbox",
                                                checked: row_selected,
                                                onclick: {
                                                    let row_id = row_id.clone();
                                                    move |ev: MouseEvent| {
                                                        let will_select = !selected.read().contains(&row_id);
                                                        let mut next = selected.read().clone();
                                                        if ev.modifiers().shift() {
                                                            if let Some(last_idx) = *last_selected_idx.read() {
                                                                let (start, end) = if last_idx <= i { (last_idx, i) } else { (i, last_idx) };
                                                                for id in &all_row_ids[start..=end] {
                                                                    if will_select { next.insert(id.clone()); } else { next.remove(id); }
                                                                }
                                                            } else if will_select { next.insert(row_id.clone()); } else { next.remove(&row_id); }
                                                        } else if will_select { next.insert(row_id.clone()); } else { next.remove(&row_id); }
                                                        selected.set(next);
                                                        last_selected_idx.set(Some(i));
                                                    }
                                                },
                                            }
                                        }
                                        div {
                                            FilterLink {
                                                field: ErrorsPageFilter::Step,
                                                value: error.step.clone(),
                                                "{error.step}"
                                            }
                                        }
                                        div {
                                            FilterLink {
                                                field: ErrorsPageFilter::Title,
                                                value: error.title.clone(),
                                                "{error.title}"
                                            }
                                        }
                                        div { "{error.error}" }
                                        div { "{error.created_at}" }
                                        div {
                                            if let Some(mam_id) = error.mam_id {
                                                a { href: "/dioxus/torrents/{mam_id}", "open" }
                                                a {
                                                    href: "https://www.myanonamouse.net/t/{mam_id}",
                                                    target: "_blank",
                                                    "MaM"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if let Some(Err(e)) = &*value.read() {
                p { class: "error", "Error: {e}" }
            } else {
                p { "Loading errors..." }
            }
        }
    }
}
