use std::collections::BTreeSet;
use std::sync::Arc;

use crate::components::{
    ActiveFilterChip, ActiveFilters, FilterLink, PageSizeSelector, Pagination, SortHeader,
    TorrentGridTable, build_query_string, encode_query_enum, parse_location_query_pairs,
    parse_query_enum, set_location_query_string,
};
use dioxus::prelude::*;

use super::server_fns::{apply_duplicate_action, get_duplicate_data};
use super::types::*;

fn filter_name(filter: DuplicatePageFilter) -> &'static str {
    match filter {
        DuplicatePageFilter::Kind => "Type",
        DuplicatePageFilter::Title => "Title",
        DuplicatePageFilter::Author => "Authors",
        DuplicatePageFilter::Narrator => "Narrators",
        DuplicatePageFilter::Series => "Series",
        DuplicatePageFilter::Filetype => "Filetypes",
    }
}

#[derive(Clone)]
struct PageQueryState {
    sort: Option<DuplicatePageSort>,
    asc: bool,
    filters: Vec<(DuplicatePageFilter, String)>,
    from: usize,
    page_size: usize,
}

impl Default for PageQueryState {
    fn default() -> Self {
        Self {
            sort: None,
            asc: false,
            filters: Vec::new(),
            from: 0,
            page_size: 500,
        }
    }
}

fn parse_query_state() -> PageQueryState {
    let mut state = PageQueryState::default();
    for (key, value) in parse_location_query_pairs() {
        match key.as_str() {
            "sort_by" => state.sort = parse_query_enum::<DuplicatePageSort>(&value),
            "asc" => state.asc = value == "true",
            "from" => {
                if let Ok(v) = value.parse::<usize>() {
                    state.from = v;
                }
            }
            "page_size" => {
                if let Ok(v) = value.parse::<usize>() {
                    state.page_size = v;
                }
            }
            _ => {
                if let Some(field) = parse_query_enum::<DuplicatePageFilter>(&key) {
                    state.filters.push((field, value));
                }
            }
        }
    }
    state
}

fn build_query_url(
    sort: Option<DuplicatePageSort>,
    asc: bool,
    filters: &[(DuplicatePageFilter, String)],
    from: usize,
    page_size: usize,
) -> String {
    let mut params = Vec::new();
    if let Some(sort) = sort.and_then(encode_query_enum) {
        params.push(("sort_by".to_string(), sort));
    }
    if asc {
        params.push(("asc".to_string(), "true".to_string()));
    }
    if from > 0 {
        params.push(("from".to_string(), from.to_string()));
    }
    if page_size != 500 {
        params.push(("page_size".to_string(), page_size.to_string()));
    }
    for (field, value) in filters {
        if let Some(name) = encode_query_enum(*field) {
            params.push((name, value.clone()));
        }
    }
    build_query_string(&params)
}

#[component]
pub fn DuplicatePage() -> Element {
    let _route: crate::app::Route = use_route();
    let initial_state = parse_query_state();
    let initial_sort = initial_state.sort;
    let initial_asc = initial_state.asc;
    let initial_filters = initial_state.filters.clone();
    let initial_from = initial_state.from;
    let initial_page_size = initial_state.page_size;
    let initial_request_key = build_query_url(
        initial_state.sort,
        initial_state.asc,
        &initial_state.filters,
        initial_state.from,
        initial_state.page_size,
    );

    let sort = use_signal(move || initial_sort);
    let asc = use_signal(move || initial_asc);
    let filters = use_signal(move || initial_filters.clone());
    let mut from = use_signal(move || initial_from);
    let mut page_size = use_signal(move || initial_page_size);
    let mut selected = use_signal(BTreeSet::<u64>::new);
    let mut last_selected_idx = use_signal(|| None::<usize>);
    let mut status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached = use_signal(|| None::<DuplicateData>);
    let loading_action = use_signal(|| false);
    let mut last_request_key = use_signal(move || initial_request_key.clone());

    let mut duplicate_data = use_server_future(move || async move {
        get_duplicate_data(
            *sort.read(),
            *asc.read(),
            filters.read().clone(),
            Some(*from.read()),
            Some(*page_size.read()),
        )
        .await
    })
    .ok();

    let pending = duplicate_data
        .as_ref()
        .map(|resource| resource.pending())
        .unwrap_or(true);
    let value = duplicate_data.as_ref().map(|resource| resource.value());

    {
        let route_state = parse_query_state();
        let route_request_key = build_query_url(
            route_state.sort,
            route_state.asc,
            &route_state.filters,
            route_state.from,
            route_state.page_size,
        );
        if *last_request_key.read() != route_request_key {
            let mut sort = sort;
            let mut asc = asc;
            let mut filters_signal = filters;
            let mut from = from;
            let mut page_size = page_size;
            sort.set(route_state.sort);
            asc.set(route_state.asc);
            filters_signal.set(route_state.filters);
            from.set(route_state.from);
            page_size.set(route_state.page_size);
            last_request_key.set(route_request_key);
            if let Some(resource) = duplicate_data.as_mut() {
                resource.restart();
            }
        }
    }

    if let Some(value) = &value {
        let value = value.read();
        if let Some(Ok(data)) = &*value {
            cached.set(Some(data.clone()));
        }
    }

    let data_to_show = {
        if let Some(value) = &value {
            let value = value.read();
            match &*value {
                Some(Ok(data)) => Some(data.clone()),
                _ => cached.read().clone(),
            }
        } else {
            cached.read().clone()
        }
    };

    use_effect(move || {
        let query_string = build_query_url(
            *sort.read(),
            *asc.read(),
            &filters.read().clone(),
            *from.read(),
            *page_size.read(),
        );
        let should_restart = *last_request_key.read() != query_string;
        if should_restart {
            last_request_key.set(query_string.clone());
            set_location_query_string(&query_string);
            if let Some(resource) = duplicate_data.as_mut() {
                resource.restart();
            }
        }
    });

    let mut active_chips = Vec::new();
    for (field, value) in filters.read().clone() {
        active_chips.push(ActiveFilterChip {
            label: format!("{}: {}", filter_name(field), value),
            on_remove: Callback::new({
                let value = value.clone();
                let mut filters = filters;
                let mut from = from;
                move |_| {
                    filters
                        .write()
                        .retain(|(f, v)| !(*f == field && *v == value));
                    from.set(0);
                }
            }),
        });
    }

    let clear_all: Option<Callback<()>> = if active_chips.is_empty() {
        None
    } else {
        Some(Callback::new({
            let mut filters = filters;
            let mut from = from;
            move |_| {
                filters.set(Vec::new());
                from.set(0);
            }
        }))
    };

    let all_row_ids = Arc::new(
        data_to_show
            .as_ref()
            .map(|data| {
                data.torrents
                    .iter()
                    .map(|p| p.torrent.mam_id)
                    .collect::<Vec<u64>>()
            })
            .unwrap_or_default(),
    );

    rsx! {
        div { class: "duplicate-page",
            div { class: "row",
                h1 { "Duplicate Torrents" }
                div { class: "actions actions_torrent",
                    style: if selected.read().is_empty() { "" } else { "display: flex" },
                    for action in [DuplicateBulkAction::Replace, DuplicateBulkAction::Remove] {
                        button {
                            r#type: "button",
                            disabled: *loading_action.read(),
                            onclick: {
                                let mut loading_action = loading_action;
                                let mut status_msg = status_msg;
                                let mut duplicate_data = duplicate_data;
                                let mut selected = selected;
                                move |_| {
                                    let ids = selected.read().iter().copied().collect::<Vec<_>>();
                                    if ids.is_empty() {
                                        status_msg.set(Some(("Select at least one torrent".to_string(), true)));
                                        return;
                                    }
                                    loading_action.set(true);
                                    status_msg.set(None);
                                    spawn(async move {
                                        match apply_duplicate_action(action, ids).await {
                                            Ok(_) => {
                                                status_msg.set(Some((action.success_label().to_string(), false)));
                                                selected.set(BTreeSet::new());
                                                if let Some(resource) = duplicate_data.as_mut() {
                                                    resource.restart();
                                                }
                                            }
                                            Err(e) => {
                                                status_msg.set(Some((format!("{} failed: {e}", action.label()), true)));
                                            }
                                        }
                                        loading_action.set(false);
                                    });
                                }
                            },
                            "{action.label()}"
                        }
                    }
                }
                div { class: "table_options",
                    PageSizeSelector {
                        page_size: *page_size.read(),
                        options: vec![100, 500, 1000, 5000],
                        show_all_option: true,
                        on_change: move |next| {
                            page_size.set(next);
                            from.set(0);
                        },
                    }
                }
            }

            p { "Torrents that were not selected due to an existing torrent in your library" }

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
                if data.torrents.is_empty() {
                    p {
                        i { "There are currently no duplicate torrents" }
                    }
                } else {
                    TorrentGridTable {
                        grid_template: "30px 110px 2fr 1fr 1fr 1fr 81px 100px 72px 157px 132px"
                            .to_string(),
                        extra_class: Some("DuplicateTable".to_string()),
                        pending: pending && cached.read().is_some(),
                        {
                            let all_selected = data.torrents.iter().all(|p| selected.read().contains(&p.torrent.mam_id));
                            rsx! {
                                div { class: "torrents-grid-row",
                                    div { class: "header",
                                        input {
                                            r#type: "checkbox",
                                            checked: all_selected,
                                            onchange: {
                                                let row_ids = data.torrents.iter().map(|p| p.torrent.mam_id).collect::<Vec<_>>();
                                                move |ev| {
                                                    if ev.value() == "true" {
                                                        let mut next = selected.read().clone();
                                                        for id in &row_ids {
                                                            next.insert(*id);
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
                                    SortHeader { label: "Type", sort_key: DuplicatePageSort::Kind, sort, asc, from }
                                    SortHeader { label: "Title", sort_key: DuplicatePageSort::Title, sort, asc, from }
                                    SortHeader { label: "Authors", sort_key: DuplicatePageSort::Authors, sort, asc, from }
                                    SortHeader { label: "Narrators", sort_key: DuplicatePageSort::Narrators, sort, asc, from }
                                    SortHeader { label: "Series", sort_key: DuplicatePageSort::Series, sort, asc, from }
                                    SortHeader { label: "Size", sort_key: DuplicatePageSort::Size, sort, asc, from }
                                    div { class: "header", "Filetypes" }
                                    div { class: "header", "Linked" }
                                    SortHeader { label: "Added At", sort_key: DuplicatePageSort::CreatedAt, sort, asc, from }
                                    div { class: "header", "" }
                                }
                            }
                        }

                        for (i, pair) in data.torrents.iter().enumerate() {
                            {
                                let row_id = pair.torrent.mam_id;
                                let row_selected = selected.read().contains(&row_id);
                                let all_row_ids = all_row_ids.clone();
                                rsx! {
                                    div { class: "torrents-grid-row", key: "{row_id}",
                                        div {
                                            input {
                                                r#type: "checkbox",
                                                checked: row_selected,
                                                onclick: move |ev| {
                                                    let will_select = !selected.read().contains(&row_id);
                                                    let mut next = selected.read().clone();
                                                    if ev.modifiers().shift() {
                                                        if let Some(last_idx) = *last_selected_idx.read() {
                                                            let (start, end) = if last_idx <= i { (last_idx, i) } else { (i, last_idx) };
                                                            for id in &all_row_ids[start..=end] {
                                                                if will_select { next.insert(*id); } else { next.remove(id); }
                                                            }
                                                        } else if will_select { next.insert(row_id); } else { next.remove(&row_id); }
                                                    } else if will_select { next.insert(row_id); } else { next.remove(&row_id); }
                                                    selected.set(next);
                                                    last_selected_idx.set(Some(i));
                                                },
                                            }
                                        }
                                        div {
                                            FilterLink {
                                                field: DuplicatePageFilter::Kind,
                                                value: pair.torrent.meta.media_type.clone(),
                                                reset_from: true,
                                                "{pair.torrent.meta.media_type}"
                                            }
                                        }
                                        div {
                                            FilterLink {
                                                field: DuplicatePageFilter::Title,
                                                value: pair.torrent.meta.title.clone(),
                                                reset_from: true,
                                                "{pair.torrent.meta.title}"
                                            }
                                        }
                                        div {
                                            for author in pair.torrent.meta.authors.clone() {
                                                FilterLink {
                                                    field: DuplicatePageFilter::Author,
                                                    value: author.clone(),
                                                    reset_from: true,
                                                    "{author}"
                                                }
                                            }
                                        }
                                        div {
                                            for narrator in pair.torrent.meta.narrators.clone() {
                                                FilterLink {
                                                    field: DuplicatePageFilter::Narrator,
                                                    value: narrator.clone(),
                                                    reset_from: true,
                                                    "{narrator}"
                                                }
                                            }
                                        }
                                        div {
                                            for series in pair.torrent.meta.series.clone() {
                                                FilterLink {
                                                    field: DuplicatePageFilter::Series,
                                                    value: series.name.clone(),
                                                    reset_from: true,
                                                    if series.entries.is_empty() {
                                                        "{series.name}"
                                                    } else {
                                                        "{series.name} #{series.entries}"
                                                    }
                                                }
                                            }
                                        }
                                        div { "{pair.torrent.meta.size}" }
                                        div {
                                            for filetype in pair.torrent.meta.filetypes.clone() {
                                                FilterLink {
                                                    field: DuplicatePageFilter::Filetype,
                                                    value: filetype.clone(),
                                                    reset_from: true,
                                                    "{filetype}"
                                                }
                                            }
                                        }
                                        div {}
                                        div { "{pair.torrent.created_at}" }
                                        div {
                                            a {
                                                href: "https://www.myanonamouse.net/t/{pair.torrent.mam_id}",
                                                target: "_blank",
                                                "MaM"
                                            }
                                        }

                                        div {}
                                        div { class: "faint", "duplicate of:" }
                                        div { "{pair.duplicate_of.meta.title}" }
                                        div { "{pair.duplicate_of.meta.authors.join(\", \")}" }
                                        div { "{pair.duplicate_of.meta.narrators.join(\", \")}" }
                                        div {
                                            for series in pair.duplicate_of.meta.series.clone() {
                                                span {
                                                    if series.entries.is_empty() {
                                                        "{series.name} "
                                                    } else {
                                                        "{series.name} #{series.entries} "
                                                    }
                                                }
                                            }
                                        }
                                        div { "{pair.duplicate_of.meta.size}" }
                                        div { "{pair.duplicate_of.meta.filetypes.join(\", \")}" }
                                        div {
                                            span { title: "{pair.duplicate_of.linked_path.clone().unwrap_or_default()}",
                                                "{pair.duplicate_of.linked}"
                                            }
                                        }
                                        div { "{pair.duplicate_of.created_at}" }
                                        div {
                                            a { href: "/dioxus/torrents/{pair.duplicate_of.id}", "open" }
                                            if let Some(mam_id) = pair.duplicate_of.mam_id {
                                                a { href: "https://www.myanonamouse.net/t/{mam_id}", target: "_blank", "MaM" }
                                            }
                                            if let (Some(abs_url), Some(abs_id)) = (&data.abs_url, &pair.duplicate_of.abs_id) {
                                                a {
                                                    href: "{abs_url}/audiobookshelf/item/{abs_id}",
                                                    target: "_blank",
                                                    "ABS"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    p { class: "faint",
                        "Showing {data.from} to {data.from + data.torrents.len()} of {data.total}"
                    }
                    Pagination {
                        total: data.total,
                        from: data.from,
                        page_size: data.page_size,
                        on_change: move |new_from| from.set(new_from),
                    }
                }
            } else if let Some(value) = &value {
                if let Some(Err(e)) = &*value.read() {
                    p { class: "error", "Error: {e}" }
                } else {
                    p { "Loading duplicate torrents..." }
                }
            } else {
                p { "Loading duplicate torrents..." }
            }
        }
    }
}
