use crate::components::{
    ActiveFilterChip, ActiveFilters, ColumnSelector, ColumnToggleOption, PageSizeSelector,
    Pagination, TorrentGridTable, apply_click_filter, build_query_string, encode_query_enum,
    parse_location_query_pairs, parse_query_enum, set_location_query_string,
};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[cfg(feature = "server")]
use std::str::FromStr;

#[cfg(feature = "server")]
use crate::error::OptionIntoServerFnError;
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
#[cfg(feature = "server")]
use mlm_core::{
    Context, ContextExt, Torrent,
    linker::{refresh_mam_metadata, refresh_metadata_relink},
};
#[cfg(feature = "server")]
use mlm_db::{DatabaseExt as _, Language, TorrentKey, ids};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ReplacedPageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Replaced,
    CreatedAt,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ReplacedPageFilter {
    Kind,
    Title,
    Author,
    Narrator,
    Series,
    Language,
    Filetype,
    Linked,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplacedPageColumns {
    pub authors: bool,
    pub narrators: bool,
    pub series: bool,
    pub language: bool,
    pub size: bool,
    pub filetypes: bool,
}

impl Default for ReplacedPageColumns {
    fn default() -> Self {
        Self {
            authors: true,
            narrators: true,
            series: true,
            language: false,
            size: true,
            filetypes: true,
        }
    }
}

impl ReplacedPageColumns {
    fn table_grid_template(self) -> String {
        let mut cols = vec!["30px", "110px", "2fr"];
        if self.authors {
            cols.push("1fr");
        }
        if self.narrators {
            cols.push("1fr");
        }
        if self.series {
            cols.push("1fr");
        }
        if self.language {
            cols.push("100px");
        }
        if self.size {
            cols.push("81px");
        }
        if self.filetypes {
            cols.push("100px");
        }
        cols.push("157px");
        cols.push("157px");
        cols.push("132px");
        cols.join(" ")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplacedSeries {
    pub name: String,
    pub entries: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplacedMeta {
    pub title: String,
    pub media_type: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<ReplacedSeries>,
    pub language: Option<String>,
    pub size: String,
    pub filetypes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplacedRow {
    pub id: String,
    pub mam_id: Option<u64>,
    pub meta: ReplacedMeta,
    pub linked: bool,
    pub created_at: String,
    pub replaced_at: Option<String>,
    pub abs_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplacedPairRow {
    pub torrent: ReplacedRow,
    pub replacement: ReplacedRow,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct ReplacedData {
    pub torrents: Vec<ReplacedPairRow>,
    pub total: usize,
    pub from: usize,
    pub page_size: usize,
    pub abs_url: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ReplacedBulkAction {
    Refresh,
    RefreshRelink,
    Remove,
}

impl ReplacedBulkAction {
    fn label(self) -> &'static str {
        match self {
            Self::Refresh => "refresh metadata",
            Self::RefreshRelink => "refresh metadata and relink",
            Self::Remove => "remove torrent from MLM",
        }
    }

    fn success_label(self) -> &'static str {
        match self {
            Self::Refresh => "Refreshed metadata",
            Self::RefreshRelink => "Refreshed metadata and relinked",
            Self::Remove => "Removed torrents",
        }
    }
}

#[cfg(feature = "server")]
fn matches_filter(t: &Torrent, field: ReplacedPageFilter, value: &str) -> bool {
    match field {
        ReplacedPageFilter::Kind => t.meta.media_type.as_str() == value,
        ReplacedPageFilter::Title => t.meta.title == value,
        ReplacedPageFilter::Author => t.meta.authors.contains(&value.to_string()),
        ReplacedPageFilter::Narrator => t.meta.narrators.contains(&value.to_string()),
        ReplacedPageFilter::Series => t.meta.series.iter().any(|s| s.name == value),
        ReplacedPageFilter::Language => {
            if value.is_empty() {
                t.meta.language.is_none()
            } else {
                t.meta.language == Language::from_str(value).ok()
            }
        }
        ReplacedPageFilter::Filetype => t.meta.filetypes.iter().any(|f| f == value),
        ReplacedPageFilter::Linked => t.library_path.is_some() == (value == "true"),
    }
}

#[cfg(feature = "server")]
fn convert_row(t: &Torrent) -> ReplacedRow {
    ReplacedRow {
        id: t.id.clone(),
        mam_id: t.mam_id,
        meta: ReplacedMeta {
            title: t.meta.title.clone(),
            media_type: t.meta.media_type.as_str().to_string(),
            authors: t.meta.authors.clone(),
            narrators: t.meta.narrators.clone(),
            series: t
                .meta
                .series
                .iter()
                .map(|series| ReplacedSeries {
                    name: series.name.clone(),
                    entries: series.entries.to_string(),
                })
                .collect(),
            language: t.meta.language.map(|l| l.to_str().to_string()),
            size: t.meta.size.to_string(),
            filetypes: t.meta.filetypes.clone(),
        },
        linked: t.library_path.is_some(),
        created_at: format_timestamp_db(&t.created_at),
        replaced_at: t
            .replaced_with
            .as_ref()
            .map(|(_, ts)| format_timestamp_db(ts)),
        abs_id: t.meta.ids.get(ids::ABS).cloned(),
    }
}

#[server]
pub async fn get_replaced_data(
    sort: Option<ReplacedPageSort>,
    asc: bool,
    filters: Vec<(ReplacedPageFilter, String)>,
    from: Option<usize>,
    page_size: Option<usize>,
    _show: ReplacedPageColumns,
) -> Result<ReplacedData, ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_server_err("Context not found in extensions")?;

    let mut from_val = from.unwrap_or(0);
    let page_size_val = page_size.unwrap_or(500);

    let r = context
        .db()
        .r_transaction()
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut replaced = r
        .scan()
        .secondary::<Torrent>(TorrentKey::created_at)
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .all()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .rev()
        .filter_map(Result::ok)
        .filter(|t| t.replaced_with.is_some())
        .filter(|t| {
            filters
                .iter()
                .all(|(field, value)| matches_filter(t, *field, value))
        })
        .collect::<Vec<_>>();

    if let Some(sort_by) = sort {
        replaced.sort_by(|a, b| {
            let ord = match sort_by {
                ReplacedPageSort::Kind => a.meta.media_type.cmp(&b.meta.media_type),
                ReplacedPageSort::Title => a.meta.title.cmp(&b.meta.title),
                ReplacedPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                ReplacedPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                ReplacedPageSort::Series => a.meta.series.cmp(&b.meta.series),
                ReplacedPageSort::Language => a.meta.language.cmp(&b.meta.language),
                ReplacedPageSort::Size => a.meta.size.cmp(&b.meta.size),
                ReplacedPageSort::Replaced => a
                    .replaced_with
                    .as_ref()
                    .map(|r| r.1)
                    .cmp(&b.replaced_with.as_ref().map(|r| r.1)),
                ReplacedPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if asc { ord.reverse() } else { ord }
        });
    }

    let total = replaced.len();
    if page_size_val > 0 && from_val >= total && total > 0 {
        from_val = ((total - 1) / page_size_val) * page_size_val;
    }

    let limit = if page_size_val == 0 {
        usize::MAX
    } else {
        page_size_val
    };

    let mut rows = Vec::new();
    for torrent in replaced.into_iter().skip(from_val).take(limit) {
        let Some((replacement_id, _)) = &torrent.replaced_with else {
            continue;
        };
        let Some(replacement) = r
            .get()
            .primary::<Torrent>(replacement_id.clone())
            .map_err(|e| ServerFnError::new(e.to_string()))?
        else {
            continue;
        };
        rows.push(ReplacedPairRow {
            torrent: convert_row(&torrent),
            replacement: convert_row(&replacement),
        });
    }

    let abs_url = context
        .config()
        .await
        .audiobookshelf
        .as_ref()
        .map(|abs| abs.url.clone());

    Ok(ReplacedData {
        torrents: rows,
        total,
        from: from_val,
        page_size: page_size_val,
        abs_url,
    })
}

#[server]
pub async fn apply_replaced_action(
    action: ReplacedBulkAction,
    torrent_ids: Vec<String>,
) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    if torrent_ids.is_empty() {
        return Err(ServerFnError::new("No torrents selected"));
    }

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_server_err("Context not found in extensions")?;

    match action {
        ReplacedBulkAction::Refresh => {
            let config = context.config().await;
            let mam = context
                .mam()
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            for id in torrent_ids {
                refresh_mam_metadata(&config, context.db(), &mam, id, &context.events)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            }
        }
        ReplacedBulkAction::RefreshRelink => {
            let config = context.config().await;
            let mam = context
                .mam()
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            for id in torrent_ids {
                refresh_metadata_relink(&config, context.db(), &mam, id, &context.events)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            }
        }
        ReplacedBulkAction::Remove => {
            let (_guard, rw) = context
                .db()
                .rw_async()
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            for id in torrent_ids {
                let Some(torrent) = rw
                    .get()
                    .primary::<Torrent>(id)
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                else {
                    continue;
                };
                rw.remove(torrent)
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            }
            rw.commit().map_err(|e| ServerFnError::new(e.to_string()))?;
        }
    }

    Ok(())
}

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

fn show_to_query_value(show: ReplacedPageColumns) -> String {
    let mut values = Vec::new();
    if show.authors {
        values.push("author");
    }
    if show.narrators {
        values.push("narrator");
    }
    if show.series {
        values.push("series");
    }
    if show.language {
        values.push("language");
    }
    if show.size {
        values.push("size");
    }
    if show.filetypes {
        values.push("filetype");
    }
    values.join(",")
}

fn show_from_query_value(value: &str) -> ReplacedPageColumns {
    let mut show = ReplacedPageColumns {
        authors: false,
        narrators: false,
        series: false,
        language: false,
        size: false,
        filetypes: false,
    };
    for item in value.split(',') {
        match item {
            "author" => show.authors = true,
            "narrator" => show.narrators = true,
            "series" => show.series = true,
            "language" => show.language = true,
            "size" => show.size = true,
            "filetype" => show.filetypes = true,
            _ => {}
        }
    }
    show
}

#[derive(Clone)]
struct LegacyQueryState {
    sort: Option<ReplacedPageSort>,
    asc: bool,
    filters: Vec<(ReplacedPageFilter, String)>,
    from: usize,
    page_size: usize,
    show: ReplacedPageColumns,
}

impl Default for LegacyQueryState {
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

fn parse_legacy_query_state() -> LegacyQueryState {
    let mut state = LegacyQueryState::default();
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
            "show" => state.show = show_from_query_value(&value),
            _ => {
                if let Some(field) = parse_query_enum::<ReplacedPageFilter>(&key) {
                    state.filters.push((field, value));
                }
            }
        }
    }
    state
}

fn build_legacy_query_string(
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
        params.push(("show".to_string(), show_to_query_value(show)));
    }
    for (field, value) in filters {
        if let Some(name) = encode_query_enum(*field) {
            params.push((name, value.clone()));
        }
    }
    build_query_string(&params)
}

#[derive(Clone, Copy)]
enum ReplacedColumn {
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Filetypes,
}

const COLUMN_OPTIONS: &[(ReplacedColumn, &str)] = &[
    (ReplacedColumn::Authors, "Authors"),
    (ReplacedColumn::Narrators, "Narrators"),
    (ReplacedColumn::Series, "Series"),
    (ReplacedColumn::Language, "Language"),
    (ReplacedColumn::Size, "Size"),
    (ReplacedColumn::Filetypes, "Filetypes"),
];

fn column_enabled(show: ReplacedPageColumns, column: ReplacedColumn) -> bool {
    match column {
        ReplacedColumn::Authors => show.authors,
        ReplacedColumn::Narrators => show.narrators,
        ReplacedColumn::Series => show.series,
        ReplacedColumn::Language => show.language,
        ReplacedColumn::Size => show.size,
        ReplacedColumn::Filetypes => show.filetypes,
    }
}

fn set_column_enabled(show: &mut ReplacedPageColumns, column: ReplacedColumn, enabled: bool) {
    match column {
        ReplacedColumn::Authors => show.authors = enabled,
        ReplacedColumn::Narrators => show.narrators = enabled,
        ReplacedColumn::Series => show.series = enabled,
        ReplacedColumn::Language => show.language = enabled,
        ReplacedColumn::Size => show.size = enabled,
        ReplacedColumn::Filetypes => show.filetypes = enabled,
    }
}

#[component]
pub fn ReplacedPage() -> Element {
    let initial_state = parse_legacy_query_state();
    let initial_sort = initial_state.sort;
    let initial_asc = initial_state.asc;
    let initial_filters = initial_state.filters.clone();
    let initial_from = initial_state.from;
    let initial_page_size = initial_state.page_size;
    let initial_show = initial_state.show;
    let initial_request_key = build_legacy_query_string(
        initial_state.sort,
        initial_state.asc,
        &initial_state.filters,
        initial_state.from,
        initial_state.page_size,
        initial_state.show,
    );

    let sort = use_signal(move || initial_sort);
    let asc = use_signal(move || initial_asc);
    let mut filters = use_signal(move || initial_filters.clone());
    let mut from = use_signal(move || initial_from);
    let mut page_size = use_signal(move || initial_page_size);
    let show = use_signal(move || initial_show);
    let mut selected = use_signal(BTreeSet::<String>::new);
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
        let query_string = build_legacy_query_string(
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

    let sort_header = |label: &'static str, key: ReplacedPageSort| {
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
                    "{label}{arrow}"
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
        div { class: "replaced-page",
            div { class: "row",
                h1 { "Replaced Torrents" }
                div { class: "actions actions_torrent",
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
                                    {sort_header("Type", ReplacedPageSort::Kind)}
                                    {sort_header("Title", ReplacedPageSort::Title)}
                                    if show.read().authors {
                                        {sort_header("Authors", ReplacedPageSort::Authors)}
                                    }
                                    if show.read().narrators {
                                        {sort_header("Narrators", ReplacedPageSort::Narrators)}
                                    }
                                    if show.read().series {
                                        {sort_header("Series", ReplacedPageSort::Series)}
                                    }
                                    if show.read().language {
                                        {sort_header("Language", ReplacedPageSort::Language)}
                                    }
                                    if show.read().size {
                                        {sort_header("Size", ReplacedPageSort::Size)}
                                    }
                                    if show.read().filetypes {
                                        div { class: "header", "Filetypes" }
                                    }
                                    {sort_header("Replaced", ReplacedPageSort::Replaced)}
                                    {sort_header("Added At", ReplacedPageSort::CreatedAt)}
                                    div { class: "header", "" }
                                }
                            }
                        }

                        for pair in data.torrents.clone() {
                            {
                                let row_id = pair.torrent.id.clone();
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
                                                onclick: {
                                                    let value = pair.torrent.meta.media_type.clone();
                                                    let mut from = from;
                                                    move |_| {
                                                        apply_click_filter(&mut filters, ReplacedPageFilter::Kind, value.clone());
                                                        from.set(0);
                                                    }
                                                },
                                                "{pair.torrent.meta.media_type}"
                                            }
                                        }
                                        div {
                                            button {
                                                r#type: "button",
                                                class: "link",
                                                onclick: {
                                                    let value = pair.torrent.meta.title.clone();
                                                    let mut from = from;
                                                    move |_| {
                                                        apply_click_filter(&mut filters, ReplacedPageFilter::Title, value.clone());
                                                        from.set(0);
                                                    }
                                                },
                                                "{pair.torrent.meta.title}"
                                            }
                                        }
                                        if show.read().authors {
                                            div {
                                                for author in pair.torrent.meta.authors.clone() {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let author = author.clone();
                                                            let mut from = from;
                                                            move |_| {
                                                                apply_click_filter(&mut filters, ReplacedPageFilter::Author, author.clone());
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
                                                for narrator in pair.torrent.meta.narrators.clone() {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let narrator = narrator.clone();
                                                            let mut from = from;
                                                            move |_| {
                                                                apply_click_filter(&mut filters, ReplacedPageFilter::Narrator, narrator.clone());
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
                                                for series in pair.torrent.meta.series.clone() {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let series_name = series.name.clone();
                                                            let mut from = from;
                                                            move |_| {
                                                                apply_click_filter(&mut filters, ReplacedPageFilter::Series, series_name.clone());
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
                                                        let value = pair.torrent.meta.language.clone().unwrap_or_default();
                                                        let mut from = from;
                                                        move |_| {
                                                            apply_click_filter(&mut filters, ReplacedPageFilter::Language, value.clone());
                                                            from.set(0);
                                                        }
                                                    },
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
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let filetype = filetype.clone();
                                                            let mut from = from;
                                                            move |_| {
                                                                apply_click_filter(&mut filters, ReplacedPageFilter::Filetype, filetype.clone());
                                                                from.set(0);
                                                            }
                                                        },
                                                        "{filetype}"
                                                    }
                                                }
                                            }
                                        }
                                        div { "{pair.torrent.replaced_at.clone().unwrap_or_default()}" }
                                        div { "{pair.torrent.created_at}" }
                                        div {
                                            a { href: "/dioxus/torrents/{pair.torrent.id}", "open" }
                                            if let Some(mam_id) = pair.torrent.mam_id {
                                                a { href: "https://www.myanonamouse.net/t/{mam_id}", target: "_blank", "MaM" }
                                            }
                                            if let (Some(abs_url), Some(abs_id)) = (&data.abs_url, &pair.torrent.abs_id) {
                                                a {
                                                    href: "{abs_url}/audiobookshelf/item/{abs_id}",
                                                    target: "_blank",
                                                    "ABS"
                                                }
                                            }
                                        }

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
                                        div {
                                            a { href: "/dioxus/torrents/{pair.replacement.id}", "open" }
                                            if let Some(mam_id) = pair.replacement.mam_id {
                                                a { href: "https://www.myanonamouse.net/t/{mam_id}", target: "_blank", "MaM" }
                                            }
                                            if let (Some(abs_url), Some(abs_id)) = (&data.abs_url, &pair.replacement.abs_id) {
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
                    p { "Loading replaced torrents..." }
                }
            } else {
                p { "Loading replaced torrents..." }
            }
        }
    }
}
