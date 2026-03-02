use crate::components::{
    ActiveFilterChip, ActiveFilters, ColumnSelector, ColumnToggleOption, FilterLink, PageColumns,
    PageSizeSelector, Pagination, SortHeader, TorrentGridTable, TorrentTitleLink,
    build_query_string, encode_query_enum, parse_location_query_pairs, parse_query_enum,
    set_location_query_string,
};
use dioxus::prelude::*;
use std::collections::BTreeSet;
use std::sync::Arc;

use super::server_fns::*;
use super::types::*;

fn filter_name(filter: ReplacedPageFilter) -> &'static str {
    match filter {
        ReplacedPageFilter::Kind => "Type",
        ReplacedPageFilter::Title => "Title",
        ReplacedPageFilter::Author => "Authors",
        ReplacedPageFilter::Narrator => "Narrators",
        ReplacedPageFilter::Series => "Series",
        ReplacedPageFilter::Language => "Language",
        ReplacedPageFilter::Filetype => "Filetypes",
        ReplacedPageFilter::Linked => "Linked",
    }
}

#[derive(Clone)]
struct PageQueryState {
    sort: Option<ReplacedPageSort>,
    asc: bool,
    filters: Vec<(ReplacedPageFilter, String)>,
    from: usize,
    page_size: usize,
    show: ReplacedPageColumns,
}

impl Default for PageQueryState {
    fn default() -> Self {
        Self {
            sort: None,
            asc: false,
            filters: Vec::new(),
            from: 0,
            page_size: 500,
            show: ReplacedPageColumns::default(),
        }
    }
}

fn parse_query_state() -> PageQueryState {
    let mut state = PageQueryState::default();
    for (key, value) in parse_location_query_pairs() {
        match key.as_str() {
            "sort_by" => state.sort = parse_query_enum::<ReplacedPageSort>(&value),
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
            "show" => state.show = ReplacedPageColumns::from_query_value(&value),
            _ => {
                if let Some(field) = parse_query_enum::<ReplacedPageFilter>(&key) {
                    state.filters.push((field, value));
                }
            }
        }
    }
    state
}

fn build_query_url(
    sort: Option<ReplacedPageSort>,
    asc: bool,
    filters: &[(ReplacedPageFilter, String)],
    from: usize,
    page_size: usize,
    show: ReplacedPageColumns,
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
    if show != ReplacedPageColumns::default() {
        params.push(("show".to_string(), show.to_query_value()));
    }
    for (field, value) in filters {
        if let Some(name) = encode_query_enum(*field) {
            params.push((name, value.clone()));
        }
    }
    build_query_string(&params)
}

#[component]
pub fn ReplacedPage() -> Element {
    let _route: crate::app::Route = use_route();
    let initial_state = parse_query_state();
    let initial_sort = initial_state.sort;
    let initial_asc = initial_state.asc;
    let initial_filters = initial_state.filters.clone();
    let initial_from = initial_state.from;
    let initial_page_size = initial_state.page_size;
    let initial_show = initial_state.show;
    let initial_request_key = build_query_url(
        initial_state.sort,
        initial_state.asc,
        &initial_state.filters,
        initial_state.from,
        initial_state.page_size,
        initial_state.show,
    );

    let sort = use_signal(move || initial_sort);
    let asc = use_signal(move || initial_asc);
    let filters = use_signal(move || initial_filters.clone());
    let mut from = use_signal(move || initial_from);
    let mut page_size = use_signal(move || initial_page_size);
    let show = use_signal(move || initial_show);
    let mut selected = use_signal(BTreeSet::<String>::new);
    let mut last_selected_idx = use_signal(|| None::<usize>);
    let mut status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached = use_signal(|| None::<ReplacedData>);
    let loading_action = use_signal(|| false);
    let mut last_request_key = use_signal(move || initial_request_key.clone());

    let mut replaced_data = use_server_future(move || async move {
        get_replaced_data(
            *sort.read(),
            *asc.read(),
            filters.read().clone(),
            Some(*from.read()),
            Some(*page_size.read()),
            *show.read(),
        )
        .await
    })
    .ok();

    let pending = replaced_data
        .as_ref()
        .map(|resource| resource.pending())
        .unwrap_or(true);
    let value = replaced_data.as_ref().map(|resource| resource.value());

    {
        let route_state = parse_query_state();
        let route_request_key = build_query_url(
            route_state.sort,
            route_state.asc,
            &route_state.filters,
            route_state.from,
            route_state.page_size,
            route_state.show,
        );
        if *last_request_key.read() != route_request_key {
            let mut sort = sort;
            let mut asc = asc;
            let mut filters_signal = filters;
            let mut from = from;
            let mut page_size = page_size;
            let mut show = show;
            sort.set(route_state.sort);
            asc.set(route_state.asc);
            filters_signal.set(route_state.filters);
            from.set(route_state.from);
            page_size.set(route_state.page_size);
            show.set(route_state.show);
            last_request_key.set(route_request_key);
            if let Some(resource) = replaced_data.as_mut() {
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
            *show.read(),
        );
        let should_restart = *last_request_key.read() != query_string;
        if should_restart {
            last_request_key.set(query_string.clone());
            set_location_query_string(&query_string);
            if let Some(resource) = replaced_data.as_mut() {
                resource.restart();
            }
        }
    });

    let column_options = COLUMN_OPTIONS
        .iter()
        .map(|(column, label)| {
            let checked = show.read().get(*column);
            let column = *column;
            ColumnToggleOption {
                label,
                checked,
                on_toggle: Callback::new({
                    let mut show = show;
                    move |enabled| {
                        let mut next = *show.read();
                        next.set(column, enabled);
                        show.set(next);
                    }
                }),
            }
        })
        .collect::<Vec<_>>();

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
                    .map(|p| p.torrent.id.clone())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default(),
    );

    rsx! {
        div { class: "replaced-page",
            div { class: "row",
                h1 { "Replaced Torrents" }
                div { class: "actions actions_torrent",
                    style: if selected.read().is_empty() { "" } else { "display: flex" },
                    for action in [ReplacedBulkAction::Refresh, ReplacedBulkAction::RefreshRelink, ReplacedBulkAction::Remove] {
                        button {
                            r#type: "button",
                            disabled: *loading_action.read(),
                            onclick: {
                                let mut loading_action = loading_action;
                                let mut status_msg = status_msg;
                                let mut replaced_data = replaced_data;
                                let mut selected = selected;
                                move |_| {
                                    let ids = selected.read().iter().cloned().collect::<Vec<_>>();
                                    if ids.is_empty() {
                                        status_msg.set(Some(("Select at least one torrent".to_string(), true)));
                                        return;
                                    }
                                    loading_action.set(true);
                                    status_msg.set(None);
                                    spawn(async move {
                                        match apply_replaced_action(action, ids).await {
                                            Ok(_) => {
                                                status_msg.set(Some((action.success_label().to_string(), false)));
                                                selected.set(BTreeSet::new());
                                                if let Some(resource) = replaced_data.as_mut() {
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
                    ColumnSelector { options: column_options }
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

            p { "Torrents that were unlinked from the library and replaced with a preferred version" }

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
                        i { "You have no replaced torrents" }
                    }
                } else {
                    TorrentGridTable {
                        grid_template: show.read().table_grid_template(),
                        extra_class: None,
                        pending: pending && cached.read().is_some(),
                        {
                            let all_selected = data.torrents.iter().all(|p| selected.read().contains(&p.torrent.id));
                            rsx! {
                                div { class: "torrents-grid-row",
                                    div { class: "header",
                                        input {
                                            r#type: "checkbox",
                                            checked: all_selected,
                                            onchange: {
                                                let row_ids = data.torrents.iter().map(|p| p.torrent.id.clone()).collect::<Vec<_>>();
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
                                    SortHeader { label: "Type", sort_key: ReplacedPageSort::Kind, sort, asc, from }
                                    SortHeader { label: "Title", sort_key: ReplacedPageSort::Title, sort, asc, from }
                                    if show.read().authors {
                                        SortHeader { label: "Authors", sort_key: ReplacedPageSort::Authors, sort, asc, from }
                                    }
                                    if show.read().narrators {
                                        SortHeader { label: "Narrators", sort_key: ReplacedPageSort::Narrators, sort, asc, from }
                                    }
                                    if show.read().series {
                                        SortHeader { label: "Series", sort_key: ReplacedPageSort::Series, sort, asc, from }
                                    }
                                    if show.read().language {
                                        SortHeader { label: "Language", sort_key: ReplacedPageSort::Language, sort, asc, from }
                                    }
                                    if show.read().size {
                                        SortHeader { label: "Size", sort_key: ReplacedPageSort::Size, sort, asc, from }
                                    }
                                    if show.read().filetypes {
                                        div { class: "header", "Filetypes" }
                                    }
                                    SortHeader { label: "Replaced", sort_key: ReplacedPageSort::Replaced, sort, asc, from }
                                    SortHeader { label: "Added At", sort_key: ReplacedPageSort::CreatedAt, sort, asc, from }
                                }
                            }
                        }

                        for (i, pair) in data.torrents.iter().enumerate() {
                            {
                                let row_id = pair.torrent.id.clone();
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
                                                field: ReplacedPageFilter::Kind,
                                                value: pair.torrent.meta.media_type.clone(),
                                                reset_from: true,
                                                "{pair.torrent.meta.media_type}"
                                            }
                                        }
                                        div {
                                            TorrentTitleLink {
                                                detail_id: pair.torrent.id.clone(),
                                                field: ReplacedPageFilter::Title,
                                                value: pair.torrent.meta.title.clone(),
                                                reset_from: true,
                                                "{pair.torrent.meta.title}"
                                            }
                                        }
                                        if show.read().authors {
                                            div {
                                                for author in pair.torrent.meta.authors.clone() {
                                                    FilterLink {
                                                        field: ReplacedPageFilter::Author,
                                                        value: author.clone(),
                                                        reset_from: true,
                                                        "{author}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().narrators {
                                            div {
                                                for narrator in pair.torrent.meta.narrators.clone() {
                                                    FilterLink {
                                                        field: ReplacedPageFilter::Narrator,
                                                        value: narrator.clone(),
                                                        reset_from: true,
                                                        "{narrator}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().series {
                                            div {
                                                for series in pair.torrent.meta.series.clone() {
                                                    FilterLink {
                                                        field: ReplacedPageFilter::Series,
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
                                        }
                                        if show.read().language {
                                            div {
                                                FilterLink {
                                                    field: ReplacedPageFilter::Language,
                                                    value: pair.torrent.meta.language.clone().unwrap_or_default(),
                                                    reset_from: true,
                                                    "{pair.torrent.meta.language.clone().unwrap_or_default()}"
                                                }
                                            }
                                        }
                                        if show.read().size {
                                            div { "{pair.torrent.meta.size}" }
                                        }
                                        if show.read().filetypes {
                                            div {
                                                for filetype in pair.torrent.meta.filetypes.clone() {
                                                    FilterLink {
                                                        field: ReplacedPageFilter::Filetype,
                                                        value: filetype.clone(),
                                                        reset_from: true,
                                                        "{filetype}"
                                                    }
                                                }
                                            }
                                        }
                                        div { "{pair.torrent.replaced_at.clone().unwrap_or_default()}" }
                                        div { "{pair.torrent.created_at}" }

                                        div {}
                                        div { class: "faint", "replaced with:" }
                                        div { "{pair.replacement.meta.title}" }
                                        if show.read().authors {
                                            div {
                                                for author in pair.replacement.meta.authors.clone() {
                                                    span { "{author} " }
                                                }
                                            }
                                        }
                                        if show.read().narrators {
                                            div {
                                                for narrator in pair.replacement.meta.narrators.clone() {
                                                    span { "{narrator} " }
                                                }
                                            }
                                        }
                                        if show.read().series {
                                            div {
                                                for series in pair.replacement.meta.series.clone() {
                                                    span {
                                                        if series.entries.is_empty() {
                                                            "{series.name} "
                                                        } else {
                                                            "{series.name} #{series.entries} "
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().language {
                                            div { "{pair.replacement.meta.language.clone().unwrap_or_default()}" }
                                        }
                                        if show.read().size {
                                            div { "{pair.replacement.meta.size}" }
                                        }
                                        if show.read().filetypes {
                                            div { "{pair.replacement.meta.filetypes.join(\", \")}" }
                                        }
                                        div { "{pair.replacement.replaced_at.clone().unwrap_or_default()}" }
                                        div { "{pair.replacement.created_at}" }
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
                    p { "Loading replaced torrents..." }
                }
            } else {
                p { "Loading replaced torrents..." }
            }
        }
    }
}
