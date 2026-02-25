use std::collections::BTreeSet;

use crate::components::{
    ActiveFilterChip, ActiveFilters, FilterLink, TorrentGridTable, build_query_string,
    encode_query_enum, parse_location_query_pairs, parse_query_enum, set_location_query_string,
};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::error::OptionIntoServerFnError;
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
#[cfg(feature = "server")]
use mlm_core::{Context, ContextExt};
#[cfg(feature = "server")]
use mlm_db::{DatabaseExt as _, ErroredTorrent, ErroredTorrentId, ErroredTorrentKey, ids};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ErrorsPageSort {
    Step,
    Title,
    Error,
    CreatedAt,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ErrorsPageFilter {
    Step,
    Title,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorsRow {
    pub id_json: String,
    pub step: String,
    pub title: String,
    pub error: String,
    pub created_at: String,
    pub mam_id: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct ErrorsData {
    pub errors: Vec<ErrorsRow>,
}

#[server]
pub async fn get_errors_data(
    sort: Option<ErrorsPageSort>,
    asc: bool,
    filters: Vec<(ErrorsPageFilter, String)>,
) -> Result<ErrorsData, ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_server_err("Context not found in extensions")?;

    let mut errors = context
        .db()
        .r_transaction()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .scan()
        .secondary::<ErroredTorrent>(ErroredTorrentKey::created_at)
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .all()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .rev()
        .filter_map(Result::ok)
        .filter(|t| {
            filters.iter().all(|(field, value)| match field {
                ErrorsPageFilter::Step => error_step(&t.id) == value,
                ErrorsPageFilter::Title => t.title == *value,
            })
        })
        .collect::<Vec<_>>();

    if let Some(sort_by) = sort {
        errors.sort_by(|a, b| {
            let ord = match sort_by {
                ErrorsPageSort::Step => error_step(&a.id).cmp(error_step(&b.id)),
                ErrorsPageSort::Title => a.title.cmp(&b.title),
                ErrorsPageSort::Error => a.error.cmp(&b.error),
                ErrorsPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if asc { ord.reverse() } else { ord }
        });
    }

    Ok(ErrorsData {
        errors: errors.into_iter().map(convert_error_row).collect(),
    })
}

#[server]
pub async fn remove_errors_action(error_ids: Vec<String>) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    if error_ids.is_empty() {
        return Err(ServerFnError::new("No errors selected"));
    }

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_server_err("Context not found in extensions")?;

    let (_guard, rw) = context
        .db()
        .rw_async()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    for error_id in error_ids {
        let id = serde_json::from_str::<ErroredTorrentId>(&error_id)
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let Some(error) = rw
            .get()
            .primary::<ErroredTorrent>(id)
            .map_err(|e| ServerFnError::new(e.to_string()))?
        else {
            continue;
        };
        rw.remove(error)
            .map_err(|e| ServerFnError::new(e.to_string()))?;
    }

    rw.commit().map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[cfg(feature = "server")]
fn error_step(id: &ErroredTorrentId) -> &'static str {
    match id {
        ErroredTorrentId::Grabber(_) => "auto grabber",
        ErroredTorrentId::Linker(_) => "library linker",
        ErroredTorrentId::Cleaner(_) => "library cleaner",
    }
}

#[cfg(feature = "server")]
fn convert_error_row(error: ErroredTorrent) -> ErrorsRow {
    ErrorsRow {
        id_json: serde_json::to_string(&error.id).unwrap_or_default(),
        step: error_step(&error.id).to_string(),
        title: error.title,
        error: error.error,
        created_at: format_timestamp_db(&error.created_at),
        mam_id: error
            .meta
            .and_then(|meta| meta.ids.get(ids::MAM).cloned())
            .and_then(|id| id.parse::<u64>().ok()),
    }
}

fn filter_name(filter: ErrorsPageFilter) -> &'static str {
    match filter {
        ErrorsPageFilter::Step => "Step",
        ErrorsPageFilter::Title => "Title",
    }
}

#[derive(Clone, Default)]
struct LegacyQueryState {
    sort: Option<ErrorsPageSort>,
    asc: bool,
    filters: Vec<(ErrorsPageFilter, String)>,
}

fn parse_legacy_query_state() -> LegacyQueryState {
    let mut state = LegacyQueryState::default();
    for (key, value) in parse_location_query_pairs() {
        match key.as_str() {
            "sort_by" => state.sort = parse_query_enum::<ErrorsPageSort>(&value),
            "asc" => state.asc = value == "true",
            _ => {
                if let Some(field) = parse_query_enum::<ErrorsPageFilter>(&key) {
                    state.filters.push((field, value));
                }
            }
        }
    }
    state
}

fn build_legacy_query_string(
    sort: Option<ErrorsPageSort>,
    asc: bool,
    filters: &[(ErrorsPageFilter, String)],
) -> String {
    let mut params = Vec::new();
    if let Some(sort) = sort.and_then(encode_query_enum) {
        params.push(("sort_by".to_string(), sort));
    }
    if asc {
        params.push(("asc".to_string(), "true".to_string()));
    }
    for (field, value) in filters {
        if let Some(name) = encode_query_enum(*field) {
            params.push((name, value.clone()));
        }
    }
    build_query_string(&params)
}

#[component]
pub fn ErrorsPage() -> Element {
    let _route: crate::app::Route = use_route();
    let initial_state = parse_legacy_query_state();
    let initial_sort = initial_state.sort;
    let initial_asc = initial_state.asc;
    let initial_filters = initial_state.filters.clone();
    let initial_request_key = build_legacy_query_string(
        initial_state.sort,
        initial_state.asc,
        &initial_state.filters,
    );

    let sort = use_signal(move || initial_sort);
    let asc = use_signal(move || initial_asc);
    let filters = use_signal(move || initial_filters.clone());
    let mut selected = use_signal(BTreeSet::<String>::new);
    let mut status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached = use_signal(|| None::<ErrorsData>);
    let loading_action = use_signal(|| false);
    let mut last_request_key = use_signal(move || initial_request_key.clone());

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
        let route_state = parse_legacy_query_state();
        let route_request_key =
            build_legacy_query_string(route_state.sort, route_state.asc, &route_state.filters);
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
        let query_string = build_legacy_query_string(sort, asc, &filters);
        let should_restart = *last_request_key.read() != query_string;
        if should_restart {
            last_request_key.set(query_string.clone());
            set_location_query_string(&query_string);
            errors_data.restart();
        }
    });

    let sort_header = |label: &'static str, key: ErrorsPageSort| {
        let active = *sort.read() == Some(key);
        let arrow = if active {
            if *asc.read() { "↑" } else { "↓" }
        } else {
            ""
        };
        rsx! {
            div { class: "header",
                button {
                    r#type: "button",
                    class: "link",
                    onclick: {
                        let mut sort = sort;
                        let mut asc = asc;
                        move |_| {
                            if *sort.read() == Some(key) {
                                let next_asc = !*asc.read();
                                asc.set(next_asc);
                            } else {
                                sort.set(Some(key));
                                asc.set(false);
                            }
                        }
                    },
                    "{label}{arrow}"
                }
            }
        }
    };

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
                                    {sort_header("Step", ErrorsPageSort::Step)}
                                    {sort_header("Title", ErrorsPageSort::Title)}
                                    {sort_header("Error", ErrorsPageSort::Error)}
                                    {sort_header("When", ErrorsPageSort::CreatedAt)}
                                    div { class: "header", "" }
                                }
                            }
                        }

                        for error in data.errors {
                            {
                                let row_id = error.id_json.clone();
                                let row_selected = selected.read().contains(&row_id);
                                rsx! {
                                    div { class: "torrents-grid-row", key: "{row_id}",
                                        div {
                                            input {
                                                r#type: "checkbox",
                                                checked: row_selected,
                                                onchange: {
                                                    let row_id = row_id.clone();
                                                    move |ev| {
                                                        let mut next = selected.read().clone();
                                                        if ev.value() == "true" {
                                                            next.insert(row_id.clone());
                                                        } else {
                                                            next.remove(&row_id);
                                                        }
                                                        selected.set(next);
                                                    }
                                                },
                                            }
                                        }
                                        div {
                                            FilterLink {
                                                filters: filters,
                                                field: ErrorsPageFilter::Step,
                                                value: error.step.clone(),
                                                "{error.step}"
                                            }
                                        }
                                        div {
                                            FilterLink {
                                                filters: filters,
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
