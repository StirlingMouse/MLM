use std::collections::BTreeSet;

use crate::components::{
    ActiveFilterChip, ActiveFilters, FilterLink, PageSizeSelector, Pagination, SortHeader,
    TorrentGridTable, build_query_string, encode_query_enum, parse_location_query_pairs,
    parse_query_enum, set_location_query_string,
};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::error::{IntoServerFnError, OptionIntoServerFnError};
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
#[cfg(feature = "server")]
use mlm_core::{ContextExt, Torrent, cleaner::clean_torrent};
#[cfg(feature = "server")]
use mlm_db::{DatabaseExt as _, DuplicateTorrent, SelectedTorrent, Timestamp, TorrentCost, ids};
#[cfg(feature = "server")]
use mlm_parse::normalize_title;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DuplicatePageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Size,
    CreatedAt,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DuplicatePageFilter {
    Kind,
    Title,
    Author,
    Narrator,
    Series,
    Filetype,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicateSeries {
    pub name: String,
    pub entries: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicateMeta {
    pub title: String,
    pub media_type: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<DuplicateSeries>,
    pub size: String,
    pub filetypes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicateCandidateRow {
    pub mam_id: u64,
    pub meta: DuplicateMeta,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicateOriginalRow {
    pub id: String,
    pub mam_id: Option<u64>,
    pub meta: DuplicateMeta,
    pub linked: bool,
    pub linked_path: Option<String>,
    pub created_at: String,
    pub abs_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicatePairRow {
    pub torrent: DuplicateCandidateRow,
    pub duplicate_of: DuplicateOriginalRow,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct DuplicateData {
    pub torrents: Vec<DuplicatePairRow>,
    pub total: usize,
    pub from: usize,
    pub page_size: usize,
    pub abs_url: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateBulkAction {
    Replace,
    Remove,
}

impl DuplicateBulkAction {
    fn label(self) -> &'static str {
        match self {
            Self::Replace => "replace original",
            Self::Remove => "remove duplicate",
        }
    }

    fn success_label(self) -> &'static str {
        match self {
            Self::Replace => "Replaced original torrents",
            Self::Remove => "Removed duplicate torrents",
        }
    }
}

#[cfg(feature = "server")]
fn matches_filter(t: &DuplicateTorrent, field: DuplicatePageFilter, value: &str) -> bool {
    match field {
        DuplicatePageFilter::Kind => t.meta.media_type.as_str() == value,
        DuplicatePageFilter::Title => t.meta.title == value,
        DuplicatePageFilter::Author => t.meta.authors.contains(&value.to_string()),
        DuplicatePageFilter::Narrator => t.meta.narrators.contains(&value.to_string()),
        DuplicatePageFilter::Series => t.meta.series.iter().any(|s| s.name == value),
        DuplicatePageFilter::Filetype => t.meta.filetypes.iter().any(|f| f == value),
    }
}

#[cfg(feature = "server")]
fn convert_candidate_row(t: &DuplicateTorrent) -> DuplicateCandidateRow {
    DuplicateCandidateRow {
        mam_id: t.mam_id,
        meta: DuplicateMeta {
            title: t.meta.title.clone(),
            media_type: t.meta.media_type.as_str().to_string(),
            authors: t.meta.authors.clone(),
            narrators: t.meta.narrators.clone(),
            series: t
                .meta
                .series
                .iter()
                .map(|series| DuplicateSeries {
                    name: series.name.clone(),
                    entries: series.entries.to_string(),
                })
                .collect(),
            size: t.meta.size.to_string(),
            filetypes: t.meta.filetypes.clone(),
        },
        created_at: format_timestamp_db(&t.created_at),
    }
}

#[cfg(feature = "server")]
fn convert_original_row(t: &Torrent) -> DuplicateOriginalRow {
    DuplicateOriginalRow {
        id: t.id.clone(),
        mam_id: t.mam_id,
        meta: DuplicateMeta {
            title: t.meta.title.clone(),
            media_type: t.meta.media_type.as_str().to_string(),
            authors: t.meta.authors.clone(),
            narrators: t.meta.narrators.clone(),
            series: t
                .meta
                .series
                .iter()
                .map(|series| DuplicateSeries {
                    name: series.name.clone(),
                    entries: series.entries.to_string(),
                })
                .collect(),
            size: t.meta.size.to_string(),
            filetypes: t.meta.filetypes.clone(),
        },
        linked: t.library_path.is_some(),
        linked_path: t
            .library_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        created_at: format_timestamp_db(&t.created_at),
        abs_id: t.meta.ids.get(ids::ABS).cloned(),
    }
}

#[server]
pub async fn get_duplicate_data(
    sort: Option<DuplicatePageSort>,
    asc: bool,
    filters: Vec<(DuplicatePageFilter, String)>,
    from: Option<usize>,
    page_size: Option<usize>,
) -> Result<DuplicateData, ServerFnError> {
    let context = crate::error::get_context()?;

    let mut from_val = from.unwrap_or(0);
    let page_size_val = page_size.unwrap_or(500);

    let r = context.db().r_transaction().server_err()?;

    let mut duplicates = r
        .scan()
        .primary::<DuplicateTorrent>()
        .server_err()?
        .all()
        .server_err()?
        .filter_map(Result::ok)
        .filter(|t| {
            filters
                .iter()
                .all(|(field, value)| matches_filter(t, *field, value))
        })
        .collect::<Vec<_>>();

    if let Some(sort_by) = sort {
        duplicates.sort_by(|a, b| {
            let ord = match sort_by {
                DuplicatePageSort::Kind => a.meta.media_type.cmp(&b.meta.media_type),
                DuplicatePageSort::Title => a.meta.title.cmp(&b.meta.title),
                DuplicatePageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                DuplicatePageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                DuplicatePageSort::Series => a.meta.series.cmp(&b.meta.series),
                DuplicatePageSort::Size => a.meta.size.cmp(&b.meta.size),
                DuplicatePageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if asc { ord.reverse() } else { ord }
        });
    }

    let total = duplicates.len();
    if page_size_val > 0 && from_val >= total && total > 0 {
        from_val = ((total - 1) / page_size_val) * page_size_val;
    }

    let limit = if page_size_val == 0 {
        usize::MAX
    } else {
        page_size_val
    };

    let mut rows = Vec::new();
    for duplicate in duplicates.into_iter().skip(from_val).take(limit) {
        let Some(duplicate_of_id) = &duplicate.duplicate_of else {
            continue;
        };
        let Some(duplicate_of) = r
            .get()
            .primary::<Torrent>(duplicate_of_id.clone())
            .server_err()?
        else {
            continue;
        };
        rows.push(DuplicatePairRow {
            torrent: convert_candidate_row(&duplicate),
            duplicate_of: convert_original_row(&duplicate_of),
        });
    }

    let abs_url = context
        .config()
        .await
        .audiobookshelf
        .as_ref()
        .map(|abs| abs.url.clone());

    Ok(DuplicateData {
        torrents: rows,
        total,
        from: from_val,
        page_size: page_size_val,
        abs_url,
    })
}

#[server]
pub async fn apply_duplicate_action(
    action: DuplicateBulkAction,
    torrent_ids: Vec<u64>,
) -> Result<(), ServerFnError> {
    if torrent_ids.is_empty() {
        return Err(ServerFnError::new("No torrents selected"));
    }

    let context = crate::error::get_context()?;
    let config = context.config().await;

    match action {
        DuplicateBulkAction::Replace => {
            let mam = context.mam().server_err()?;
            for mam_id in torrent_ids {
                let r = context.db().r_transaction().server_err()?;
                let Some(duplicate_torrent) =
                    r.get().primary::<DuplicateTorrent>(mam_id).server_err()?
                else {
                    continue;
                };
                let Some(hash) = duplicate_torrent.duplicate_of.clone() else {
                    return Err(ServerFnError::new("No duplicate_of set"));
                };
                let Some(duplicate_of) = r.get().primary::<Torrent>(hash).server_err()? else {
                    return Err(ServerFnError::new("Could not find original torrent"));
                };

                let Some(mam_torrent) = mam
                    .get_torrent_info_by_id(duplicate_torrent.mam_id)
                    .await
                    .server_err()?
                else {
                    return Err(ServerFnError::new(
                        "Could not find duplicate torrent on MaM",
                    ));
                };

                let meta = mam_torrent.as_meta().server_err()?;
                let title_search = normalize_title(&meta.title);
                let tags: Vec<_> = config
                    .tags
                    .iter()
                    .filter(|t| t.filter.matches(&mam_torrent))
                    .collect();
                let category = tags.iter().find_map(|t| t.category.clone());
                let tags = tags.iter().flat_map(|t| t.tags.clone()).collect();
                let cost = if mam_torrent.vip {
                    TorrentCost::Vip
                } else if mam_torrent.personal_freeleech {
                    TorrentCost::PersonalFreeleech
                } else if mam_torrent.free {
                    TorrentCost::GlobalFreeleech
                } else {
                    TorrentCost::TryWedge
                };

                let (_guard, rw) = context.db().rw_async().await.server_err()?;
                rw.insert(SelectedTorrent {
                    mam_id: mam_torrent.id,
                    hash: None,
                    dl_link: mam_torrent
                        .dl
                        .clone()
                        .or_else(|| duplicate_torrent.dl_link.clone())
                        .ok_or_server_err("No download link for duplicate torrent")?,
                    unsat_buffer: None,
                    wedge_buffer: None,
                    cost,
                    category,
                    tags,
                    title_search,
                    meta,
                    grabber: None,
                    created_at: Timestamp::now(),
                    started_at: None,
                    removed_at: None,
                })
                .server_err()?;
                rw.remove(duplicate_torrent).server_err()?;
                rw.commit().server_err()?;

                clean_torrent(&config, context.db(), duplicate_of, false, &context.events)
                    .await
                    .server_err()?;
            }
        }
        DuplicateBulkAction::Remove => {
            let (_guard, rw) = context.db().rw_async().await.server_err()?;
            for mam_id in torrent_ids {
                let Some(torrent) = rw.get().primary::<DuplicateTorrent>(mam_id).server_err()?
                else {
                    continue;
                };
                rw.remove(torrent).server_err()?;
            }
            rw.commit().server_err()?;
        }
    }

    Ok(())
}

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

    rsx! {
        div { class: "duplicate-page",
            div { class: "row",
                h1 { "Duplicate Torrents" }
                div { class: "actions actions_torrent",
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

                        for pair in data.torrents.clone() {
                            {
                                let row_id = pair.torrent.mam_id;
                                let row_selected = selected.read().contains(&row_id);
                                rsx! {
                                    div { class: "torrents-grid-row", key: "{row_id}",
                                        div {
                                            input {
                                                r#type: "checkbox",
                                                checked: row_selected,
                                                onchange: move |ev| {
                                                    let mut next = selected.read().clone();
                                                    if ev.value() == "true" {
                                                        next.insert(row_id);
                                                    } else {
                                                        next.remove(&row_id);
                                                    }
                                                    selected.set(next);
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
