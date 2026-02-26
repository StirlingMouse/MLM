use std::collections::BTreeSet;
use std::sync::Arc;

use dioxus::prelude::*;

use crate::components::{
    ActiveFilterChip, ActiveFilters, ColumnSelector, ColumnToggleOption, FilterLink,
    PageSizeSelector, Pagination, SortHeader, StatusMessage, TorrentGridTable, flag_icon,
    set_location_query_string,
};

use super::query::{build_query_url, parse_query_state};
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

impl TorrentsPageColumns {
    fn get(self, col: TorrentColumn) -> bool {
        match col {
            TorrentColumn::Category => self.category,
            TorrentColumn::Categories => self.categories,
            TorrentColumn::Flags => self.flags,
            TorrentColumn::Edition => self.edition,
            TorrentColumn::Authors => self.authors,
            TorrentColumn::Narrators => self.narrators,
            TorrentColumn::Series => self.series,
            TorrentColumn::Language => self.language,
            TorrentColumn::Size => self.size,
            TorrentColumn::Filetypes => self.filetypes,
            TorrentColumn::Linker => self.linker,
            TorrentColumn::QbitCategory => self.qbit_category,
            TorrentColumn::Path => self.path,
            TorrentColumn::CreatedAt => self.created_at,
            TorrentColumn::UploadedAt => self.uploaded_at,
        }
    }

    fn set(&mut self, col: TorrentColumn, enabled: bool) {
        match col {
            TorrentColumn::Category => self.category = enabled,
            TorrentColumn::Categories => self.categories = enabled,
            TorrentColumn::Flags => self.flags = enabled,
            TorrentColumn::Edition => self.edition = enabled,
            TorrentColumn::Authors => self.authors = enabled,
            TorrentColumn::Narrators => self.narrators = enabled,
            TorrentColumn::Series => self.series = enabled,
            TorrentColumn::Language => self.language = enabled,
            TorrentColumn::Size => self.size = enabled,
            TorrentColumn::Filetypes => self.filetypes = enabled,
            TorrentColumn::Linker => self.linker = enabled,
            TorrentColumn::QbitCategory => self.qbit_category = enabled,
            TorrentColumn::Path => self.path = enabled,
            TorrentColumn::CreatedAt => self.created_at = enabled,
            TorrentColumn::UploadedAt => self.uploaded_at = enabled,
        }
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
    let initial_state = parse_query_state();
    let initial_query_input = initial_state.query.clone();
    let initial_submitted_query = initial_state.query.clone();
    let initial_sort = initial_state.sort;
    let initial_asc = initial_state.asc;
    let initial_filters = initial_state.filters.clone();
    let initial_from = initial_state.from;
    let initial_page_size = initial_state.page_size;
    let initial_show = initial_state.show;
    let initial_request_key = build_query_url(
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
    let mut last_selected_idx = use_signal(|| None::<usize>);
    let status_msg = use_signal(|| None::<(String, bool)>);
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
        let route_state = parse_query_state();
        let route_request_key = build_query_url(
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

        let query_string = build_query_url(&query, sort, asc, &filters, from, page_size, show);
        let should_restart = *last_request_key.read() != query_string;
        if should_restart {
            last_request_key.set(query_string.clone());
            set_location_query_string(&query_string);
            if let Some(resource) = torrents_data.as_mut() {
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

    let all_row_ids = Arc::new(
        data_to_show
            .as_ref()
            .map(|data| {
                data.torrents
                    .iter()
                    .map(|t| t.id.clone())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default(),
    );

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
                        "aria-hidden": "true",
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

            StatusMessage { status_msg }

            ActiveFilters { chips: active_chips, on_clear_all: clear_all }

            if let Some(data) = data_to_show {
                if data.torrents.is_empty() {
                    p {
                        i { "You have no torrents selected by MLM" }
                    }
                } else {
                    div { class: "actions actions_torrent",
                        style: if selected.read().is_empty() { "" } else { "display: flex" },
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
                                    SortHeader { label: "Type", sort_key: TorrentsPageSort::Kind, sort, asc, from }
                                    if show.read().categories {
                                        div { class: "header", "Categories" }
                                    }
                                    if show.read().flags {
                                        div { class: "header", "Flags" }
                                    }
                                    SortHeader { label: "Title", sort_key: TorrentsPageSort::Title, sort, asc, from }
                                    if show.read().edition {
                                        SortHeader { label: "Edition", sort_key: TorrentsPageSort::Edition, sort, asc, from }
                                    }
                                    if show.read().authors {
                                        SortHeader { label: "Authors", sort_key: TorrentsPageSort::Authors, sort, asc, from }
                                    }
                                    if show.read().narrators {
                                        SortHeader { label: "Narrators", sort_key: TorrentsPageSort::Narrators, sort, asc, from }
                                    }
                                    if show.read().series {
                                        SortHeader { label: "Series", sort_key: TorrentsPageSort::Series, sort, asc, from }
                                    }
                                    if show.read().language {
                                        SortHeader { label: "Language", sort_key: TorrentsPageSort::Language, sort, asc, from }
                                    }
                                    if show.read().size {
                                        SortHeader { label: "Size", sort_key: TorrentsPageSort::Size, sort, asc, from }
                                    }
                                    if show.read().filetypes {
                                        div { class: "header", "Filetypes" }
                                    }
                                    if show.read().linker {
                                        SortHeader { label: "Linker", sort_key: TorrentsPageSort::Linker, sort, asc, from }
                                    }
                                    if show.read().qbit_category {
                                        SortHeader { label: "Qbit Category", sort_key: TorrentsPageSort::QbitCategory, sort, asc, from }
                                    }
                                    SortHeader {
                                        label: if show.read().path { "Path" } else { "Linked" },
                                        sort_key: TorrentsPageSort::Linked,
                                        sort,
                                        asc,
                                        from,
                                    }
                                    if show.read().created_at {
                                        SortHeader { label: "Added At", sort_key: TorrentsPageSort::CreatedAt, sort, asc, from }
                                    }
                                    if show.read().uploaded_at {
                                        SortHeader { label: "Uploaded At", sort_key: TorrentsPageSort::UploadedAt, sort, asc, from }
                                    }
                                    div { class: "header", "" }
                                }
                            }
                        }

                        for (i, torrent) in data.torrents.iter().enumerate() {
                            {
                                let row_id = torrent.id.clone();
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
                                                field: TorrentsPageFilter::Kind,
                                                value: torrent.meta.media_type.clone(),
                                                title: Some(torrent.meta.cat_name.clone()),
                                                reset_from: true,
                                                "{torrent.meta.media_type}"
                                            }
                                            if show.read().category {
                                                if let Some(cat_id) = torrent.meta.cat_id.clone() {
                                                    div {
                                                        FilterLink {
                                                            field: TorrentsPageFilter::Category,
                                                            value: cat_id.clone(),
                                                            reset_from: true,
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
                                                        field: TorrentsPageFilter::Categories,
                                                        value: category.clone(),
                                                        reset_from: true,
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
                                                            field: TorrentsPageFilter::Flags,
                                                            value: flag.clone(),
                                                            reset_from: true,
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
                                                field: TorrentsPageFilter::Title,
                                                value: torrent.meta.title.clone(),
                                                reset_from: true,
                                                "{torrent.meta.title}"
                                            }
                                            if torrent.client_status.as_deref() == Some("removed_from_tracker") {
                                                span {
                                                    class: "warn",
                                                    title: "Torrent is removed from tracker but still seeding",
                                                    FilterLink {
                                                        field: TorrentsPageFilter::ClientStatus,
                                                        value: "removed_from_tracker".to_string(),
                                                        reset_from: true,
                                                        "⚠"
                                                    }
                                                }
                                            }
                                            if torrent.client_status.as_deref() == Some("not_in_client") {
                                                span { title: "Torrent is not seeding",
                                                    FilterLink {
                                                        field: TorrentsPageFilter::ClientStatus,
                                                        value: "not_in_client".to_string(),
                                                        reset_from: true,
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
                                                        field: TorrentsPageFilter::Author,
                                                        value: author.clone(),
                                                        reset_from: true,
                                                        "{author}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().narrators {
                                            div {
                                                for narrator in torrent.meta.narrators.clone() {
                                                    FilterLink {
                                                        field: TorrentsPageFilter::Narrator,
                                                        value: narrator.clone(),
                                                        reset_from: true,
                                                        "{narrator}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().series {
                                            div {
                                                for series in torrent.meta.series.clone() {
                                                    FilterLink {
                                                        field: TorrentsPageFilter::Series,
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
                                                    field: TorrentsPageFilter::Language,
                                                    value: torrent.meta.language.clone().unwrap_or_default(),
                                                    reset_from: true,
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
                                                        field: TorrentsPageFilter::Filetype,
                                                        value: filetype.clone(),
                                                        reset_from: true,
                                                        "{filetype}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().linker {
                                            div {
                                                FilterLink {
                                                    field: TorrentsPageFilter::Linker,
                                                    value: torrent.linker.clone().unwrap_or_default(),
                                                    reset_from: true,
                                                    "{torrent.linker.clone().unwrap_or_default()}"
                                                }
                                            }
                                        }
                                        if show.read().qbit_category {
                                            div {
                                                FilterLink {
                                                    field: TorrentsPageFilter::QbitCategory,
                                                    value: torrent.category.clone().unwrap_or_default(),
                                                    reset_from: true,
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
                                                            field: TorrentsPageFilter::LibraryMismatch,
                                                            value: mismatch.filter_value().to_string(),
                                                            reset_from: true,
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
                                                            field: TorrentsPageFilter::Linked,
                                                            value: torrent.linked.to_string(),
                                                            reset_from: true,
                                                            "{torrent.linked}"
                                                        }
                                                    }
                                                } else {
                                                    FilterLink {
                                                        field: TorrentsPageFilter::Linked,
                                                        value: torrent.linked.to_string(),
                                                        reset_from: true,
                                                        "{torrent.linked}"
                                                    }
                                                }
                                                if let Some(mismatch) = torrent.library_mismatch.clone() {
                                                    span { class: "warn", title: "{mismatch.title()}",
                                                        FilterLink {
                                                            field: TorrentsPageFilter::LibraryMismatch,
                                                            value: mismatch.filter_value().to_string(),
                                                            reset_from: true,
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
