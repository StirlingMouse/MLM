use crate::components::{
    ActiveFilterChip, ActiveFilters, ColumnSelector, ColumnToggleOption, FilterLink, PageColumns,
    PageSizeSelector, Pagination, SortHeader, TorrentGridTable, build_query_string,
    encode_query_enum, parse_location_query_pairs, parse_query_enum, set_location_query_string,
};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[cfg(feature = "server")]
use std::str::FromStr;

#[cfg(feature = "server")]
use crate::error::IntoServerFnError;
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
#[cfg(feature = "server")]
use mlm_core::{
    ContextExt, Torrent,
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

    pub fn get(self, col: ReplacedColumn) -> bool {
        match col {
            ReplacedColumn::Authors => self.authors,
            ReplacedColumn::Narrators => self.narrators,
            ReplacedColumn::Series => self.series,
            ReplacedColumn::Language => self.language,
            ReplacedColumn::Size => self.size,
            ReplacedColumn::Filetypes => self.filetypes,
        }
    }

    pub fn set(&mut self, col: ReplacedColumn, enabled: bool) {
        match col {
            ReplacedColumn::Authors => self.authors = enabled,
            ReplacedColumn::Narrators => self.narrators = enabled,
            ReplacedColumn::Series => self.series = enabled,
            ReplacedColumn::Language => self.language = enabled,
            ReplacedColumn::Size => self.size = enabled,
            ReplacedColumn::Filetypes => self.filetypes = enabled,
        }
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
    let context = crate::error::get_context()?;

    let mut from_val = from.unwrap_or(0);
    let page_size_val = page_size.unwrap_or(500);

    let r = context.db().r_transaction().server_err()?;

    let mut replaced = r
        .scan()
        .secondary::<Torrent>(TorrentKey::created_at)
        .server_err()?
        .all()
        .server_err()?
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
            .server_err()?
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
    if torrent_ids.is_empty() {
        return Err(ServerFnError::new("No torrents selected"));
    }

    let context = crate::error::get_context()?;

    match action {
        ReplacedBulkAction::Refresh => {
            let config = context.config().await;
            let mam = context.mam().server_err()?;
            for id in torrent_ids {
                refresh_mam_metadata(&config, context.db(), &mam, id, &context.events)
                    .await
                    .server_err()?;
            }
        }
        ReplacedBulkAction::RefreshRelink => {
            let config = context.config().await;
            let mam = context.mam().server_err()?;
            for id in torrent_ids {
                refresh_metadata_relink(&config, context.db(), &mam, id, &context.events)
                    .await
                    .server_err()?;
            }
        }
        ReplacedBulkAction::Remove => {
            let (_guard, rw) = context.db().rw_async().await.server_err()?;
            for id in torrent_ids {
                let Some(torrent) = rw.get().primary::<Torrent>(id).server_err()? else {
                    continue;
                };
                rw.remove(torrent).server_err()?;
            }
            rw.commit().server_err()?;
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

impl PageColumns for ReplacedPageColumns {
    fn to_query_value(&self) -> String {
        let mut values = Vec::new();
        if self.authors {
            values.push("author");
        }
        if self.narrators {
            values.push("narrator");
        }
        if self.series {
            values.push("series");
        }
        if self.language {
            values.push("language");
        }
        if self.size {
            values.push("size");
        }
        if self.filetypes {
            values.push("filetype");
        }
        values.join(",")
    }

    fn from_query_value(value: &str) -> Self {
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
                                            FilterLink {
                                                field: ReplacedPageFilter::Kind,
                                                value: pair.torrent.meta.media_type.clone(),
                                                reset_from: true,
                                                "{pair.torrent.meta.media_type}"
                                            }
                                        }
                                        div {
                                            FilterLink {
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
