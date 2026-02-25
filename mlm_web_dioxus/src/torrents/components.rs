use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::components::{
    ActiveFilterChip, ActiveFilters, ColumnSelector, ColumnToggleOption, FilterLink,
    PageSizeSelector, Pagination, TorrentGridTable, flag_icon, set_location_query_string,
};

use super::query::{build_legacy_query_string, parse_legacy_query_state};
use super::{
    TorrentsBulkAction, TorrentsData, TorrentsPageColumns, TorrentsPageFilter, TorrentsPageSort,
    apply_torrents_action, get_torrents_data,
};

#[derive(Clone, Copy)]
enum TorrentColumn {
    Category,
    Categories,
    Flags,
    Edition,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Filetypes,
    Linker,
    QbitCategory,
    Path,
    CreatedAt,
    UploadedAt,
}

const COLUMN_OPTIONS: &[(TorrentColumn, &str)] = &[
    (TorrentColumn::Category, "Category"),
    (TorrentColumn::Categories, "Categories"),
    (TorrentColumn::Flags, "Flags"),
    (TorrentColumn::Edition, "Edition"),
    (TorrentColumn::Authors, "Authors"),
    (TorrentColumn::Narrators, "Narrators"),
    (TorrentColumn::Series, "Series"),
    (TorrentColumn::Language, "Language"),
    (TorrentColumn::Size, "Size"),
    (TorrentColumn::Filetypes, "Filetypes"),
    (TorrentColumn::Linker, "Linker"),
    (TorrentColumn::QbitCategory, "Qbit Category"),
    (TorrentColumn::Path, "Path"),
    (TorrentColumn::CreatedAt, "Added At"),
    (TorrentColumn::UploadedAt, "Uploaded At"),
];

fn column_enabled(show: TorrentsPageColumns, column: TorrentColumn) -> bool {
    match column {
        TorrentColumn::Category => show.category,
        TorrentColumn::Categories => show.categories,
        TorrentColumn::Flags => show.flags,
        TorrentColumn::Edition => show.edition,
        TorrentColumn::Authors => show.authors,
        TorrentColumn::Narrators => show.narrators,
        TorrentColumn::Series => show.series,
        TorrentColumn::Language => show.language,
        TorrentColumn::Size => show.size,
        TorrentColumn::Filetypes => show.filetypes,
        TorrentColumn::Linker => show.linker,
        TorrentColumn::QbitCategory => show.qbit_category,
        TorrentColumn::Path => show.path,
        TorrentColumn::CreatedAt => show.created_at,
        TorrentColumn::UploadedAt => show.uploaded_at,
    }
}

fn set_column_enabled(show: &mut TorrentsPageColumns, column: TorrentColumn, enabled: bool) {
    match column {
        TorrentColumn::Category => show.category = enabled,
        TorrentColumn::Categories => show.categories = enabled,
        TorrentColumn::Flags => show.flags = enabled,
        TorrentColumn::Edition => show.edition = enabled,
        TorrentColumn::Authors => show.authors = enabled,
        TorrentColumn::Narrators => show.narrators = enabled,
        TorrentColumn::Series => show.series = enabled,
        TorrentColumn::Language => show.language = enabled,
        TorrentColumn::Size => show.size = enabled,
        TorrentColumn::Filetypes => show.filetypes = enabled,
        TorrentColumn::Linker => show.linker = enabled,
        TorrentColumn::QbitCategory => show.qbit_category = enabled,
        TorrentColumn::Path => show.path = enabled,
        TorrentColumn::CreatedAt => show.created_at = enabled,
        TorrentColumn::UploadedAt => show.uploaded_at = enabled,
    }
}

fn filter_name(filter: TorrentsPageFilter) -> &'static str {
    match filter {
        TorrentsPageFilter::Kind => "Type",
        TorrentsPageFilter::Category => "Category",
        TorrentsPageFilter::Categories => "Categories",
        TorrentsPageFilter::Flags => "Flags",
        TorrentsPageFilter::Title => "Title",
        TorrentsPageFilter::Author => "Authors",
        TorrentsPageFilter::Narrator => "Narrators",
        TorrentsPageFilter::Series => "Series",
        TorrentsPageFilter::Language => "Language",
        TorrentsPageFilter::Filetype => "Filetypes",
        TorrentsPageFilter::Linker => "Linker",
        TorrentsPageFilter::QbitCategory => "Qbit Category",
        TorrentsPageFilter::Linked => "Linked",
        TorrentsPageFilter::LibraryMismatch => "Library mismatch",
        TorrentsPageFilter::ClientStatus => "Client status",
        TorrentsPageFilter::Abs => "ABS",
        TorrentsPageFilter::Query => "Query",
        TorrentsPageFilter::Source => "Source",
        TorrentsPageFilter::Metadata => "Metadata",
    }
}

#[component]
pub fn TorrentsPage() -> Element {
    let _route: crate::app::Route = use_route();
    let initial_state = parse_legacy_query_state();
    let initial_query_input = initial_state.query.clone();
    let initial_submitted_query = initial_state.query.clone();
    let initial_sort = initial_state.sort;
    let initial_asc = initial_state.asc;
    let initial_filters = initial_state.filters.clone();
    let initial_from = initial_state.from;
    let initial_page_size = initial_state.page_size;
    let initial_show = initial_state.show;
    let initial_request_key = build_legacy_query_string(
        &initial_state.query,
        initial_state.sort,
        initial_state.asc,
        &initial_state.filters,
        initial_state.from,
        initial_state.page_size,
        initial_state.show,
    );

    let mut query_input = use_signal(move || initial_query_input.clone());
    let mut submitted_query = use_signal(move || initial_submitted_query.clone());
    let sort = use_signal(move || initial_sort);
    let asc = use_signal(move || initial_asc);
    let filters = use_signal(move || initial_filters.clone());
    let mut from = use_signal(move || initial_from);
    let mut page_size = use_signal(move || initial_page_size);
    let show = use_signal(move || initial_show);
    let mut selected = use_signal(BTreeSet::<String>::new);
    let mut status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached = use_signal(|| None::<TorrentsData>);
    let loading_action = use_signal(|| false);
    let mut last_request_key = use_signal(move || initial_request_key.clone());

    let mut torrents_data = use_server_future(move || async move {
        let mut server_filters = filters.read().clone();
        let query = submitted_query.read().trim().to_string();
        if !query.is_empty() {
            server_filters.push((TorrentsPageFilter::Query, query));
        }
        get_torrents_data(
            *sort.read(),
            *asc.read(),
            server_filters,
            Some(*from.read()),
            Some(*page_size.read()),
            *show.read(),
        )
        .await
    })
    .ok();

    let pending = torrents_data
        .as_ref()
        .map(|resource| resource.pending())
        .unwrap_or(true);
    let value = torrents_data.as_ref().map(|resource| resource.value());

    {
        let route_state = parse_legacy_query_state();
        let route_request_key = build_legacy_query_string(
            &route_state.query,
            route_state.sort,
            route_state.asc,
            &route_state.filters,
            route_state.from,
            route_state.page_size,
            route_state.show,
        );
        if *last_request_key.read() != route_request_key {
            let mut query_input = query_input;
            let mut submitted_query = submitted_query;
            let mut sort = sort;
            let mut asc = asc;
            let mut filters_signal = filters;
            let mut from = from;
            let mut page_size = page_size;
            let mut show = show;
            query_input.set(route_state.query.clone());
            submitted_query.set(route_state.query);
            sort.set(route_state.sort);
            asc.set(route_state.asc);
            filters_signal.set(route_state.filters);
            from.set(route_state.from);
            page_size.set(route_state.page_size);
            show.set(route_state.show);
            last_request_key.set(route_request_key);
            if let Some(resource) = torrents_data.as_mut() {
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
        let query = submitted_query.read().trim().to_string();
        let sort = *sort.read();
        let asc = *asc.read();
        let filters = filters.read().clone();
        let from = *from.read();
        let page_size = *page_size.read();
        let show = *show.read();

        let query_string =
            build_legacy_query_string(&query, sort, asc, &filters, from, page_size, show);
        let should_restart = *last_request_key.read() != query_string;
        if should_restart {
            last_request_key.set(query_string.clone());
            set_location_query_string(&query_string);
            if let Some(resource) = torrents_data.as_mut() {
                resource.restart();
            }
        }
    });

    let sort_header = |label: &'static str, key: TorrentsPageSort| {
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
                        let mut from = from;
                        move |_| {
                            if *sort.read() == Some(key) {
                                let next_asc = !*asc.read();
                                asc.set(next_asc);
                            } else {
                                sort.set(Some(key));
                                asc.set(false);
                            }
                            from.set(0);
                        }
                    },
                    "{label}"
                    "{arrow}"
                }
            }
        }
    };

    let column_options = COLUMN_OPTIONS
        .iter()
        .map(|(column, label)| {
            let checked = column_enabled(*show.read(), *column);
            let column = *column;
            ColumnToggleOption {
                label,
                checked,
                on_toggle: Callback::new({
                    let mut show = show;
                    move |enabled| {
                        let mut next = *show.read();
                        set_column_enabled(&mut next, column, enabled);
                        show.set(next);
                    }
                }),
            }
        })
        .collect::<Vec<_>>();

    let mut active_chips = Vec::new();
    if !submitted_query.read().is_empty() {
        active_chips.push(ActiveFilterChip {
            label: format!("Query: {}", submitted_query.read()),
            on_remove: Callback::new({
                let mut submitted_query = submitted_query;
                let mut query_input = query_input;
                let mut from = from;
                move |_| {
                    submitted_query.set(String::new());
                    query_input.set(String::new());
                    from.set(0);
                }
            }),
        });
    }
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
            let mut submitted_query = submitted_query;
            let mut query_input = query_input;
            let mut from = from;
            move |_| {
                filters.set(Vec::new());
                submitted_query.set(String::new());
                query_input.set(String::new());
                from.set(0);
            }
        }))
    };

    rsx! {
        div { class: "torrents-page",
            form {
                class: "row",
                onsubmit: move |ev: Event<FormData>| {
                    ev.prevent_default();
                    submitted_query.set(query_input.read().trim().to_string());
                    from.set(0);
                },
                h1 { "Torrents" }
                label {
                    input {
                        r#type: "submit",
                        value: "Search",
                        style: "display: none;",
                    }
                    "Search: "
                    input {
                        r#type: "text",
                        name: "query",
                        value: "{query_input}",
                        oninput: move |ev| query_input.set(ev.value()),
                    }
                    button {
                        r#type: "button",
                        onclick: move |_| {
                            query_input.set(String::new());
                            submitted_query.set(String::new());
                            from.set(0);
                        },
                        "×"
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

            ActiveFilters { chips: active_chips, on_clear_all: clear_all }

            if let Some(data) = data_to_show {
                if data.torrents.is_empty() {
                    p {
                        i { "You have no torrents selected by MLM" }
                    }
                } else {
                    div { class: "actions actions_torrent",
                        for action in [
                            TorrentsBulkAction::Refresh,
                            TorrentsBulkAction::RefreshRelink,
                            TorrentsBulkAction::Clean,
                            TorrentsBulkAction::Remove,
                        ]
                        {
                            button {
                                r#type: "button",
                                disabled: *loading_action.read(),
                                onclick: {
                                    let mut loading_action = loading_action;
                                    let mut status_msg = status_msg;
                                    let mut torrents_data = torrents_data;
                                    let mut selected = selected;
                                    move |_| {
                                        let ids: Vec<String> = selected.read().iter().cloned().collect();
                                        if ids.is_empty() {
                                            status_msg.set(Some(("Select at least one torrent".to_string(), true)));
                                            return;
                                        }
                                        loading_action.set(true);
                                        status_msg.set(None);
                                        spawn(async move {
                                            match apply_torrents_action(action, ids).await {
                                                Ok(_) => {
                                                    status_msg
                                                        .set(Some((action.success_label().to_string(), false)));
                                                    selected.set(BTreeSet::new());
                                                    if let Some(resource) = torrents_data.as_mut() {
                                                        resource.restart();
                                                    }
                                                }
                                                Err(e) => {
                                                    status_msg
                                                        .set(
                                                            Some((format!("{} failed: {e}", action.label()), true)),
                                                        );
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

                    TorrentGridTable {
                        grid_template: show.read().table_grid_template(),
                        extra_class: None,
                        pending: pending && cached.read().is_some(),
                        {
                            let all_selected = data
                                .torrents
                                .iter()
                                .all(|torrent| selected.read().contains(&torrent.id));
                            rsx! {
                                div { class: "torrents-grid-row",
                                    div { class: "header",
                                        input {
                                            r#type: "checkbox",
                                            checked: all_selected,
                                            onchange: {
                                                let row_ids = data
                                                    .torrents
                                                    .iter()
                                                    .map(|torrent| torrent.id.clone())
                                                    .collect::<Vec<_>>();
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
                                    {sort_header("Type", TorrentsPageSort::Kind)}
                                    if show.read().categories {
                                        div { class: "header", "Categories" }
                                    }
                                    if show.read().flags {
                                        div { class: "header", "Flags" }
                                    }
                                    {sort_header("Title", TorrentsPageSort::Title)}
                                    if show.read().edition {
                                        {sort_header("Edition", TorrentsPageSort::Edition)}
                                    }
                                    if show.read().authors {
                                        {sort_header("Authors", TorrentsPageSort::Authors)}
                                    }
                                    if show.read().narrators {
                                        {sort_header("Narrators", TorrentsPageSort::Narrators)}
                                    }
                                    if show.read().series {
                                        {sort_header("Series", TorrentsPageSort::Series)}
                                    }
                                    if show.read().language {
                                        {sort_header("Language", TorrentsPageSort::Language)}
                                    }
                                    if show.read().size {
                                        {sort_header("Size", TorrentsPageSort::Size)}
                                    }
                                    if show.read().filetypes {
                                        div { class: "header", "Filetypes" }
                                    }
                                    if show.read().linker {
                                        {sort_header("Linker", TorrentsPageSort::Linker)}
                                    }
                                    if show.read().qbit_category {
                                        {sort_header("Qbit Category", TorrentsPageSort::QbitCategory)}
                                    }
                                    {
                                        sort_header(
                                            if show.read().path { "Path" } else { "Linked" },
                                            TorrentsPageSort::Linked,
                                        )
                                    }
                                    if show.read().created_at {
                                        {sort_header("Added At", TorrentsPageSort::CreatedAt)}
                                    }
                                    if show.read().uploaded_at {
                                        {sort_header("Uploaded At", TorrentsPageSort::UploadedAt)}
                                    }
                                    div { class: "header", "" }
                                }
                            }
                        }

                        for torrent in data.torrents.clone() {
                            {
                                let row_id = torrent.id.clone();
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
                                                field: TorrentsPageFilter::Kind,
                                                value: torrent.meta.media_type.clone(),
                                                title: Some(torrent.meta.cat_name.clone()),
                                                reset_from: true,
                                                on_apply: move |_| from.set(0),
                                                "{torrent.meta.media_type}"
                                            }
                                            if show.read().category {
                                                if let Some(cat_id) = torrent.meta.cat_id.clone() {
                                                    div {
                                                        FilterLink {
                                                            filters: filters,
                                                            field: TorrentsPageFilter::Category,
                                                            value: cat_id.clone(),
                                                            reset_from: true,
                                                            on_apply: move |_| from.set(0),
                                                            "{torrent.meta.cat_name}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().categories {
                                            div {
                                                for category in torrent.meta.categories.clone() {
                                                    FilterLink {
                                                        filters: filters,
                                                        field: TorrentsPageFilter::Categories,
                                                        value: category.clone(),
                                                        reset_from: true,
                                                        on_apply: move |_| from.set(0),
                                                        "{category}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().flags {
                                            div {
                                                for flag in torrent.meta.flags.clone() {
                                                    if let Some((src, title)) = flag_icon(&flag) {
                                                        FilterLink {
                                                            filters: filters,
                                                            field: TorrentsPageFilter::Flags,
                                                            value: flag.clone(),
                                                            reset_from: true,
                                                            on_apply: move |_| from.set(0),
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
                                            FilterLink {
                                                filters: filters,
                                                field: TorrentsPageFilter::Title,
                                                value: torrent.meta.title.clone(),
                                                reset_from: true,
                                                on_apply: move |_| from.set(0),
                                                "{torrent.meta.title}"
                                            }
                                            if torrent.client_status.as_deref() == Some("removed_from_tracker") {
                                                span {
                                                    class: "warn",
                                                    title: "Torrent is removed from tracker but still seeding",
                                                    FilterLink {
                                                        filters: filters,
                                                        field: TorrentsPageFilter::ClientStatus,
                                                        value: "removed_from_tracker".to_string(),
                                                        reset_from: true,
                                                        on_apply: move |_| from.set(0),
                                                        "⚠"
                                                    }
                                                }
                                            }
                                            if torrent.client_status.as_deref() == Some("not_in_client") {
                                                span { title: "Torrent is not seeding",
                                                    FilterLink {
                                                        filters: filters,
                                                        field: TorrentsPageFilter::ClientStatus,
                                                        value: "not_in_client".to_string(),
                                                        reset_from: true,
                                                        on_apply: move |_| from.set(0),
                                                        "ℹ"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().edition {
                                            div { "{torrent.meta.edition.clone().unwrap_or_default()}" }
                                        }
                                        if show.read().authors {
                                            div {
                                                for author in torrent.meta.authors.clone() {
                                                    FilterLink {
                                                        filters: filters,
                                                        field: TorrentsPageFilter::Author,
                                                        value: author.clone(),
                                                        reset_from: true,
                                                        on_apply: move |_| from.set(0),
                                                        "{author}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().narrators {
                                            div {
                                                for narrator in torrent.meta.narrators.clone() {
                                                    FilterLink {
                                                        filters: filters,
                                                        field: TorrentsPageFilter::Narrator,
                                                        value: narrator.clone(),
                                                        reset_from: true,
                                                        on_apply: move |_| from.set(0),
                                                        "{narrator}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().series {
                                            div {
                                                for series in torrent.meta.series.clone() {
                                                    FilterLink {
                                                        filters: filters,
                                                        field: TorrentsPageFilter::Series,
                                                        value: series.name.clone(),
                                                        reset_from: true,
                                                        on_apply: move |_| from.set(0),
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
                                                    filters: filters,
                                                    field: TorrentsPageFilter::Language,
                                                    value: torrent.meta.language.clone().unwrap_or_default(),
                                                    reset_from: true,
                                                    on_apply: move |_| from.set(0),
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
                                                        filters: filters,
                                                        field: TorrentsPageFilter::Filetype,
                                                        value: filetype.clone(),
                                                        reset_from: true,
                                                        on_apply: move |_| from.set(0),
                                                        "{filetype}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().linker {
                                            div {
                                                FilterLink {
                                                    filters: filters,
                                                    field: TorrentsPageFilter::Linker,
                                                    value: torrent.linker.clone().unwrap_or_default(),
                                                    reset_from: true,
                                                    on_apply: move |_| from.set(0),
                                                    "{torrent.linker.clone().unwrap_or_default()}"
                                                }
                                            }
                                        }
                                        if show.read().qbit_category {
                                            div {
                                                FilterLink {
                                                    filters: filters,
                                                    field: TorrentsPageFilter::QbitCategory,
                                                    value: torrent.category.clone().unwrap_or_default(),
                                                    reset_from: true,
                                                    on_apply: move |_| from.set(0),
                                                    "{torrent.category.clone().unwrap_or_default()}"
                                                }
                                            }
                                        }
                                        if show.read().path {
                                            div {
                                                "{torrent.library_path.clone().unwrap_or_default()}"
                                                if let Some(mismatch) = torrent.library_mismatch.clone() {
                                                    span { class: "warn", title: "{mismatch.title()}",
                                                        FilterLink {
                                                            filters: filters,
                                                            field: TorrentsPageFilter::LibraryMismatch,
                                                            value: mismatch.filter_value().to_string(),
                                                            reset_from: true,
                                                            on_apply: move |_| from.set(0),
                                                            "⚠"
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            div {
                                                if let Some(path) = torrent.library_path.clone() {
                                                    span { title: "{path}",
                                                        FilterLink {
                                                            filters: filters,
                                                            field: TorrentsPageFilter::Linked,
                                                            value: torrent.linked.to_string(),
                                                            reset_from: true,
                                                            on_apply: move |_| from.set(0),
                                                            "{torrent.linked}"
                                                        }
                                                    }
                                                } else {
                                                    FilterLink {
                                                        filters: filters,
                                                        field: TorrentsPageFilter::Linked,
                                                        value: torrent.linked.to_string(),
                                                        reset_from: true,
                                                        on_apply: move |_| from.set(0),
                                                        "{torrent.linked}"
                                                    }
                                                }
                                                if let Some(mismatch) = torrent.library_mismatch.clone() {
                                                    span { class: "warn", title: "{mismatch.title()}",
                                                        FilterLink {
                                                            filters: filters,
                                                            field: TorrentsPageFilter::LibraryMismatch,
                                                            value: mismatch.filter_value().to_string(),
                                                            reset_from: true,
                                                            on_apply: move |_| from.set(0),
                                                            "⚠"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().created_at {
                                            div { "{torrent.created_at}" }
                                        }
                                        if show.read().uploaded_at {
                                            div { "{torrent.uploaded_at}" }
                                        }
                                        div { class: "links",
                                            a { href: "/dioxus/torrents/{torrent.id}", "open" }
                                            if let Some(mam_id) = torrent.mam_id {
                                                a {
                                                    href: "https://www.myanonamouse.net/t/{mam_id}",
                                                    target: "_blank",
                                                    "MaM"
                                                }
                                            }
                                            if let (Some(abs_url), Some(abs_id)) = (&data.abs_url, &torrent.abs_id) {
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
                        on_change: move |new_from| {
                            from.set(new_from);
                        },
                    }
                }
            } else if let Some(value) = &value {
                if let Some(Err(e)) = &*value.read() {
                    p { class: "error", "Error: {e}" }
                } else {
                    p { "Loading torrents..." }
                }
            } else {
                p { "Loading torrents..." }
            }
        }
    }
}
