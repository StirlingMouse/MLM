use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::components::{
    ActiveFilterChip, ActiveFilters, ColumnSelector, ColumnToggleOption, PageSizeSelector,
    Pagination, TorrentGridTable, apply_click_filter, set_location_query_string,
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

fn flag_icon(flag: &str) -> Option<(&'static str, &'static str)> {
    match flag {
        "language" => Some(("/assets/icons/language.png", "Crude Language")),
        "violence" => Some(("/assets/icons/hand.png", "Violence")),
        "some_explicit" => Some((
            "/assets/icons/lipssmall.png",
            "Some Sexually Explicit Content",
        )),
        "explicit" => Some(("/assets/icons/flames.png", "Sexually Explicit Content")),
        "abridged" => Some(("/assets/icons/abridged.png", "Abridged")),
        "lgbt" => Some(("/assets/icons/lgbt.png", "LGBT")),
        _ => None,
    }
}

#[component]
pub fn TorrentsPage() -> Element {
    let mut query_input = use_signal(String::new);
    let mut submitted_query = use_signal(String::new);
    let mut sort = use_signal(|| None::<TorrentsPageSort>);
    let mut asc = use_signal(|| false);
    let mut filters = use_signal(Vec::<(TorrentsPageFilter, String)>::new);
    let mut from = use_signal(|| 0usize);
    let mut page_size = use_signal(|| 500usize);
    let mut show = use_signal(TorrentsPageColumns::default);
    let mut selected = use_signal(BTreeSet::<String>::new);
    let mut status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached = use_signal(|| None::<TorrentsData>);
    let loading_action = use_signal(|| false);
    let mut last_request_key = use_signal(String::new);
    let mut url_init_done = use_signal(|| false);

    let mut torrents_data = match use_server_future(move || async move {
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
    }) {
        Ok(resource) => resource,
        Err(_) => {
            return rsx! {
                div { class: "torrents-page",
                    div { class: "row",
                        h1 { "Torrents" }
                    }
                    p { "Loading torrents..." }
                }
            };
        }
    };

    let value = torrents_data.value();
    let pending = torrents_data.pending();

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
        if *url_init_done.read() {
            return;
        }
        let parsed = parse_legacy_query_state();
        query_input.set(parsed.query.clone());
        submitted_query.set(parsed.query);
        sort.set(parsed.sort);
        asc.set(parsed.asc);
        filters.set(parsed.filters);
        from.set(parsed.from);
        page_size.set(parsed.page_size);
        show.set(parsed.show);
        url_init_done.set(true);
    });

    use_effect(move || {
        if !*url_init_done.read() {
            return;
        }
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
            torrents_data.restart();
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
                    ColumnSelector {
                        options: column_options,
                    }
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

            ActiveFilters {
                chips: active_chips,
                on_clear_all: clear_all,
            }

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
                                                    torrents_data.restart();
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

                    if pending && cached.read().is_some() {
                        p { class: "loading-indicator", "Refreshing torrent list..." }
                    }
                    TorrentGridTable {
                        grid_template: show.read().table_grid_template(),
                        extra_class: None,
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
                                            button {
                                                r#type: "button",
                                                class: "link",
                                                title: "{torrent.meta.cat_name}",
                                                onclick: {
                                                    let value = torrent.meta.media_type.clone();
                                                    move |_| {
                                                        apply_click_filter(&mut filters, TorrentsPageFilter::Kind, value.clone());
                                                        from.set(0);
                                                    }
                                                },
                                                "{torrent.meta.media_type}"
                                            }
                                            if show.read().category {
                                                if let Some(cat_id) = torrent.meta.cat_id.clone() {
                                                    div {
                                                        button {
                                                            r#type: "button",
                                                            class: "link",
                                                            onclick: {
                                                                let label = cat_id.clone();
                                                                move |_| {
                                                                    apply_click_filter(&mut filters, TorrentsPageFilter::Category, label.clone());
                                                                    from.set(0);
                                                                }
                                                            },
                                                            "{torrent.meta.cat_name}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().categories {
                                            div {
                                                for category in torrent.meta.categories.clone() {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let category = category.clone();
                                                            move |_| {
                                                                apply_click_filter(&mut filters, TorrentsPageFilter::Categories, category.clone());
                                                                from.set(0);
                                                            }
                                                        },
                                                        "{category}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().flags {
                                            div {
                                                for flag in torrent.meta.flags.clone() {
                                                    if let Some((src, title)) = flag_icon(&flag) {
                                                        button {
                                                            r#type: "button",
                                                            class: "link",
                                                            onclick: {
                                                                let flag = flag.clone();
                                                                move |_| {
                                                                    apply_click_filter(&mut filters, TorrentsPageFilter::Flags, flag.clone());
                                                                    from.set(0);
                                                                }
                                                            },
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
                                            button {
                                                r#type: "button",
                                                class: "link",
                                                onclick: {
                                                    let title = torrent.meta.title.clone();
                                                    move |_| {
                                                        apply_click_filter(&mut filters, TorrentsPageFilter::Title, title.clone());
                                                        from.set(0);
                                                    }
                                                },
                                                "{torrent.meta.title}"
                                            }
                                            if torrent.client_status.as_deref() == Some("removed_from_tracker") {
                                                span {
                                                    class: "warn",
                                                    title: "Torrent is removed from tracker but still seeding",
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: move |_| {
                                                            apply_click_filter(
                                                                &mut filters,
                                                                TorrentsPageFilter::ClientStatus,
                                                                "removed_from_tracker".to_string(),
                                                            );
                                                            from.set(0);
                                                        },
                                                        "⚠"
                                                    }
                                                }
                                            }
                                            if torrent.client_status.as_deref() == Some("not_in_client") {
                                                span { title: "Torrent is not seeding",
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: move |_| {
                                                            apply_click_filter(
                                                                &mut filters,
                                                                TorrentsPageFilter::ClientStatus,
                                                                "not_in_client".to_string(),
                                                            );
                                                            from.set(0);
                                                        },
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
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let author = author.clone();
                                                            move |_| {
                                                                apply_click_filter(&mut filters, TorrentsPageFilter::Author, author.clone());
                                                                from.set(0);
                                                            }
                                                        },
                                                        "{author}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().narrators {
                                            div {
                                                for narrator in torrent.meta.narrators.clone() {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let narrator = narrator.clone();
                                                            move |_| {
                                                                apply_click_filter(&mut filters, TorrentsPageFilter::Narrator, narrator.clone());
                                                                from.set(0);
                                                            }
                                                        },
                                                        "{narrator}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().series {
                                            div {
                                                for series in torrent.meta.series.clone() {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let series_name = series.name.clone();
                                                            move |_| {
                                                                apply_click_filter(&mut filters, TorrentsPageFilter::Series, series_name.clone());
                                                                from.set(0);
                                                            }
                                                        },
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
                                                button {
                                                    r#type: "button",
                                                    class: "link",
                                                    onclick: {
                                                        let value = torrent.meta.language.clone().unwrap_or_default();
                                                        move |_| {
                                                            apply_click_filter(&mut filters, TorrentsPageFilter::Language, value.clone());
                                                            from.set(0);
                                                        }
                                                    },
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
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let filetype = filetype.clone();
                                                            move |_| {
                                                                apply_click_filter(&mut filters, TorrentsPageFilter::Filetype, filetype.clone());
                                                                from.set(0);
                                                            }
                                                        },
                                                        "{filetype}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().linker {
                                            div {
                                                button {
                                                    r#type: "button",
                                                    class: "link",
                                                    onclick: {
                                                        let linker = torrent.linker.clone().unwrap_or_default();
                                                        move |_| {
                                                            apply_click_filter(&mut filters, TorrentsPageFilter::Linker, linker.clone());
                                                            from.set(0);
                                                        }
                                                    },
                                                    "{torrent.linker.clone().unwrap_or_default()}"
                                                }
                                            }
                                        }
                                        if show.read().qbit_category {
                                            div {
                                                button {
                                                    r#type: "button",
                                                    class: "link",
                                                    onclick: {
                                                        let category = torrent.category.clone().unwrap_or_default();
                                                        move |_| {
                                                            apply_click_filter(
                                                                &mut filters,
                                                                TorrentsPageFilter::QbitCategory,
                                                                category.clone(),
                                                            );
                                                            from.set(0);
                                                        }
                                                    },
                                                    "{torrent.category.clone().unwrap_or_default()}"
                                                }
                                            }
                                        }
                                        if show.read().path {
                                            div {
                                                "{torrent.library_path.clone().unwrap_or_default()}"
                                                if let Some(mismatch) = torrent.library_mismatch.clone() {
                                                    span { class: "warn", title: "{mismatch.title()}",
                                                        button {
                                                            r#type: "button",
                                                            class: "link",
                                                            onclick: move |_| {
                                                                apply_click_filter(
                                                                    &mut filters,
                                                                    TorrentsPageFilter::LibraryMismatch,
                                                                    mismatch.filter_value().to_string(),
                                                                );
                                                                from.set(0);
                                                            },
                                                            "⚠"
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            div {
                                                if let Some(path) = torrent.library_path.clone() {
                                                    span { title: "{path}",
                                                        button {
                                                            r#type: "button",
                                                            class: "link",
                                                            onclick: {
                                                                let linked = torrent.linked;
                                                                move |_| {
                                                                    apply_click_filter(&mut filters, TorrentsPageFilter::Linked, linked.to_string());
                                                                    from.set(0);
                                                                }
                                                            },
                                                            "{torrent.linked}"
                                                        }
                                                    }
                                                } else {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let linked = torrent.linked;
                                                            move |_| {
                                                                apply_click_filter(&mut filters, TorrentsPageFilter::Linked, linked.to_string());
                                                                from.set(0);
                                                            }
                                                        },
                                                        "{torrent.linked}"
                                                    }
                                                }
                                                if let Some(mismatch) = torrent.library_mismatch.clone() {
                                                    span { class: "warn", title: "{mismatch.title()}",
                                                        button {
                                                            r#type: "button",
                                                            class: "link",
                                                            onclick: move |_| {
                                                                apply_click_filter(
                                                                    &mut filters,
                                                                    TorrentsPageFilter::LibraryMismatch,
                                                                    mismatch.filter_value().to_string(),
                                                                );
                                                                from.set(0);
                                                            },
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
                                        div {
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
            } else if let Some(Err(e)) = &*value.read() {
                p { class: "error", "Error: {e}" }
            } else {
                p { "Loading torrents..." }
            }
        }
    }
}
