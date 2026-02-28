use std::collections::BTreeSet;
use std::sync::Arc;

use crate::components::{
    ActiveFilterChip, ActiveFilters, ColumnSelector, ColumnToggleOption, FilterLink, SortHeader,
    TorrentGridTable, TorrentTitleLink, flag_icon, set_location_query_string,
};
use crate::sse::{QBIT_PROGRESS, SELECTED_UPDATE_TRIGGER};
use dioxus::prelude::*;

use super::query::{build_query_url, parse_query_state};
use super::server_fns::{apply_selected_action, get_selected_data, get_selected_user_info};
use super::types::{
    COLUMN_OPTIONS, SelectedBulkAction, SelectedData, SelectedPageFilter, SelectedPageSort,
    filter_name,
};

#[component]
pub fn SelectedPage() -> Element {
    let _route: crate::app::Route = use_route();
    let initial_state = parse_query_state();
    let initial_sort = initial_state.sort;
    let initial_asc = initial_state.asc;
    let initial_filters = initial_state.filters.clone();
    let initial_show = initial_state.show;
    let initial_request_key = build_query_url(
        initial_state.sort,
        initial_state.asc,
        &initial_state.filters,
        initial_state.show,
    );

    let sort = use_signal(move || initial_sort);
    let asc = use_signal(move || initial_asc);
    let from = use_signal(|| 0usize);
    let filters = use_signal(move || initial_filters.clone());
    let show = use_signal(move || initial_show);
    let mut selected = use_signal(BTreeSet::<u64>::new);
    let mut last_selected_idx = use_signal(|| None::<usize>);
    let mut unsats_input = use_signal(|| "1".to_string());
    let mut status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached = use_signal(|| None::<SelectedData>);
    let loading_action = use_signal(|| false);
    let mut last_request_key = use_signal(move || initial_request_key.clone());

    let mut selected_data = use_server_future(move || async move {
        get_selected_data(
            *sort.read(),
            *asc.read(),
            filters.read().clone(),
            *show.read(),
        )
        .await
    })
    .ok();

    let mut user_info = use_signal(|| None::<super::types::SelectedUserInfo>);
    use_effect(move || {
        spawn(async move {
            user_info.set(get_selected_user_info().await.ok().flatten());
        });
    });

    let pending = selected_data
        .as_ref()
        .map(|resource| resource.pending())
        .unwrap_or(true);
    let value = selected_data.as_ref().map(|resource| resource.value());

    {
        let route_state = parse_query_state();
        let route_request_key = build_query_url(
            route_state.sort,
            route_state.asc,
            &route_state.filters,
            route_state.show,
        );
        if *last_request_key.read() != route_request_key {
            let mut sort = sort;
            let mut asc = asc;
            let mut filters_signal = filters;
            let mut show = show;
            sort.set(route_state.sort);
            asc.set(route_state.asc);
            filters_signal.set(route_state.filters);
            show.set(route_state.show);
            last_request_key.set(route_request_key);
            if let Some(resource) = selected_data.as_mut() {
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

    use_effect(move || {
        let _ = *SELECTED_UPDATE_TRIGGER.read();
        if let Some(resource) = selected_data.as_mut() {
            resource.restart();
        }
    });

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
            *show.read(),
        );
        let should_restart = *last_request_key.read() != query_string;
        if should_restart {
            last_request_key.set(query_string.clone());
            set_location_query_string(&query_string);
            if let Some(resource) = selected_data.as_mut() {
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
            .map(|data| data.torrents.iter().map(|t| t.mam_id).collect::<Vec<u64>>())
            .unwrap_or_default(),
    );

    rsx! {
        div { class: "selected-page",
            div { class: "row",
                h1 { "Selected Torrents" }
                div { class: "actions actions_torrent",
                    style: if selected.read().is_empty() { "" } else { "display: flex" },
                    button {
                        r#type: "button",
                        disabled: *loading_action.read(),
                        onclick: {
                            let mut loading_action = loading_action;
                            let mut status_msg = status_msg;
                            let mut selected_data = selected_data;
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
                                    match apply_selected_action(SelectedBulkAction::Remove, ids, None).await {
                                        Ok(_) => {
                                            status_msg.set(Some((SelectedBulkAction::Remove.success_label().to_string(), false)));
                                            selected.set(BTreeSet::new());
                                            if let Some(resource) = selected_data.as_mut() {
                                                resource.restart();
                                            }
                                        }
                                        Err(e) => {
                                            status_msg.set(Some((format!("{} failed: {e}", SelectedBulkAction::Remove.label()), true)));
                                        }
                                    }
                                    loading_action.set(false);
                                });
                            }
                        },
                        "{SelectedBulkAction::Remove.label()}"
                    }
                    span { "{SelectedBulkAction::Update.label()}:" }
                    input {
                        r#type: "number",
                        value: "{unsats_input}",
                        min: "0",
                        oninput: move |ev| unsats_input.set(ev.value()),
                    }
                    button {
                        r#type: "button",
                        disabled: *loading_action.read(),
                        onclick: {
                            let mut loading_action = loading_action;
                            let mut status_msg = status_msg;
                            let mut selected_data = selected_data;
                            let mut selected = selected;
                            move |_| {
                                let ids = selected.read().iter().copied().collect::<Vec<_>>();
                                if ids.is_empty() {
                                    status_msg.set(Some(("Select at least one torrent".to_string(), true)));
                                    return;
                                }
                                let unsats = unsats_input.read().trim().parse::<u64>().ok();
                                loading_action.set(true);
                                status_msg.set(None);
                                spawn(async move {
                                    match apply_selected_action(SelectedBulkAction::Update, ids, unsats).await {
                                        Ok(_) => {
                                            status_msg.set(Some((SelectedBulkAction::Update.success_label().to_string(), false)));
                                            selected.set(BTreeSet::new());
                                            if let Some(resource) = selected_data.as_mut() {
                                                resource.restart();
                                            }
                                        }
                                        Err(e) => {
                                            status_msg.set(Some((format!("{} failed: {e}", SelectedBulkAction::Update.label()), true)));
                                        }
                                    }
                                    loading_action.set(false);
                                });
                            }
                        },
                        "apply"
                    }
                }
                div { class: "table_options",
                    ColumnSelector {
                        options: column_options,
                    }
                }
            }
            p { "Torrents that the autograbber has selected and will be downloaded" }

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

            if let Some(info) = &*user_info.read() {
                p {
                    if let Some(buffer) = &info.remaining_buffer {
                        "Buffer: {buffer}"
                        br {}
                    }
                    "Unsats: {info.unsat_count} / {info.unsat_limit}"
                    br {}
                    "Wedges: {info.wedges}"
                    br {}
                    "Bonus: {info.bonus}"
                    if let Some(data) = data_to_show.clone() {
                        if !data.torrents.is_empty() {
                            br {}
                            "Queued Torrents: {data.queued}"
                            br {}
                            "Downloading Torrents: {data.downloading}"
                        }
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
                        i { "There are currently no torrents selected for downloading" }
                    }
                } else {
                    TorrentGridTable {
                        grid_template: show.read().table_grid_template(),
                        extra_class: Some("SelectedTable".to_string()),
                        pending: pending && cached.read().is_some(),
                        {
                            let all_selected = data.torrents.iter().all(|t| selected.read().contains(&t.mam_id));
                            rsx! {
                                div { class: "torrents-grid-row",
                                    div { class: "header",
                                        input {
                                            r#type: "checkbox",
                                            checked: all_selected,
                                            onchange: {
                                                let row_ids = data.torrents.iter().map(|t| t.mam_id).collect::<Vec<_>>();
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
                                    SortHeader { label: "Type", sort_key: SelectedPageSort::Kind, sort, asc, from }
                                    if show.read().flags {
                                        div { class: "header", "Flags" }
                                    }
                                    SortHeader { label: "Title", sort_key: SelectedPageSort::Title, sort, asc, from }
                                    if show.read().authors {
                                        SortHeader { label: "Authors", sort_key: SelectedPageSort::Authors, sort, asc, from }
                                    }
                                    if show.read().narrators {
                                        SortHeader { label: "Narrators", sort_key: SelectedPageSort::Narrators, sort, asc, from }
                                    }
                                    if show.read().series {
                                        SortHeader { label: "Series", sort_key: SelectedPageSort::Series, sort, asc, from }
                                    }
                                    if show.read().language {
                                        SortHeader { label: "Language", sort_key: SelectedPageSort::Language, sort, asc, from }
                                    }
                                    if show.read().size {
                                        SortHeader { label: "Size", sort_key: SelectedPageSort::Size, sort, asc, from }
                                    }
                                    if show.read().filetypes {
                                        div { class: "header", "Filetypes" }
                                    }
                                    SortHeader { label: "Cost", sort_key: SelectedPageSort::Cost, sort, asc, from }
                                    SortHeader { label: "Required Unsats", sort_key: SelectedPageSort::Buffer, sort, asc, from }
                                    if show.read().grabber {
                                        SortHeader { label: "Grabber", sort_key: SelectedPageSort::Grabber, sort, asc, from }
                                    }
                                    if show.read().created_at {
                                        SortHeader { label: "Added At", sort_key: SelectedPageSort::CreatedAt, sort, asc, from }
                                    }
                                    if show.read().started_at {
                                        SortHeader { label: "Started At", sort_key: SelectedPageSort::StartedAt, sort, asc, from }
                                    }
                                    if show.read().removed_at {
                                        div { class: "header", "Removed At" }
                                    }
                                }
                            }
                        }

                        for (i, torrent) in data.torrents.into_iter().enumerate() {
                            {
                                let row_id = torrent.mam_id;
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
                                                field: SelectedPageFilter::Kind,
                                                value: torrent.meta.media_type.clone(),
                                                title: Some(torrent.meta.cat_name.clone()),
                                                "{torrent.meta.media_type}"
                                            }
                                            if show.read().category {
                                                if let Some(cat_id) = torrent.meta.cat_id.clone() {
                                                    div {
                                                        FilterLink {
                                                            field: SelectedPageFilter::Category,
                                                            value: cat_id.clone(),
                                                            "{torrent.meta.cat_name}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().flags {
                                            div {
                                                for flag in torrent.meta.flags.clone() {
                                                    if let Some((src, title)) = flag_icon(&flag) {
                                                        FilterLink {
                                                            field: SelectedPageFilter::Flags,
                                                            value: flag.clone(),
                                                            img {
                                                                class: "flag",
                                                                src: "{src}",
                                                                alt: "{title}",
                                                                title: "{title}",
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div {
                                            TorrentTitleLink {
                                                detail_id: torrent.mam_id.to_string(),
                                                field: SelectedPageFilter::Title,
                                                value: torrent.meta.title.clone(),
                                                "{torrent.meta.title}"
                                            }
                                        }
                                        if show.read().authors {
                                            div {
                                                for author in torrent.meta.authors.clone() {
                                                    FilterLink {
                                                        field: SelectedPageFilter::Author,
                                                        value: author.clone(),
                                                        "{author}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().narrators {
                                            div {
                                                for narrator in torrent.meta.narrators.clone() {
                                                    FilterLink {
                                                        field: SelectedPageFilter::Narrator,
                                                        value: narrator.clone(),
                                                        "{narrator}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().series {
                                            div {
                                                for series in torrent.meta.series.clone() {
                                                    FilterLink {
                                                        field: SelectedPageFilter::Series,
                                                        value: series.name.clone(),
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
                                                    field: SelectedPageFilter::Language,
                                                    value: torrent.meta.language.clone().unwrap_or_default(),
                                                    "{torrent.meta.language.clone().unwrap_or_default()}"
                                                }
                                            }
                                        }
                                        if show.read().size {
                                            div { "{torrent.meta.size}" }
                                        }
                                        if show.read().filetypes {
                                            div {
                                                for filetype in torrent.meta.filetypes.clone() {
                                                    FilterLink {
                                                        field: SelectedPageFilter::Filetype,
                                                        value: filetype.clone(),
                                                        "{filetype}"
                                                    }
                                                }
                                            }
                                        }
                                        div {
                                            FilterLink {
                                                field: SelectedPageFilter::Cost,
                                                value: torrent.cost.clone(),
                                                "{torrent.cost}"
                                            }
                                        }
                                        div { "{torrent.required_unsats}" }
                                        if show.read().grabber {
                                            div {
                                                FilterLink {
                                                    field: SelectedPageFilter::Grabber,
                                                    value: torrent.grabber.clone().unwrap_or_default(),
                                                    "{torrent.grabber.clone().unwrap_or_default()}"
                                                }
                                            }
                                        }
                                        if show.read().created_at {
                                            div { "{torrent.created_at}" }
                                        }
                                        if show.read().started_at {
                                            div {
                                                "{torrent.started_at.clone().unwrap_or_default()}"
                                                if torrent.started_at.is_some() && torrent.removed_at.is_none() {
                                                    if let Some(pct) = QBIT_PROGRESS.read().iter().find(|(id, _)| *id == torrent.mam_id).map(|(_, p)| *p) {
                                                        " "
                                                        span {
                                                            title: "qBittorrent download progress",
                                                            "{pct}%"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().removed_at {
                                            div { "{torrent.removed_at.clone().unwrap_or_default()}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if let Some(value) = &value {
                if let Some(Err(e)) = &*value.read() {
                    p { class: "error", "Error: {e}" }
                } else {
                    p { class: "loading-indicator", "Loading selected torrents..." }
                }
            } else {
                p { class: "loading-indicator", "Loading selected torrents..." }
            }
        }
    }
}
