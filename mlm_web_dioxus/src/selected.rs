use std::collections::BTreeSet;
#[cfg(feature = "server")]
use std::str::FromStr;

use crate::components::{
    ActiveFilterChip, ActiveFilters, ColumnSelector, ColumnToggleOption, FilterLink, PageColumns,
    SortHeader, TorrentGridTable, build_query_string, encode_query_enum, flag_icon,
    parse_location_query_pairs, parse_query_enum, set_location_query_string,
};
use crate::sse::{QBIT_PROGRESS, SELECTED_UPDATE_TRIGGER};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::error::IntoServerFnError;
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
#[cfg(feature = "server")]
use mlm_core::ContextExt;
#[cfg(feature = "server")]
use mlm_db::{DatabaseExt as _, Flags, Language, OldCategory, SelectedTorrent, Timestamp};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SelectedPageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Cost,
    Buffer,
    Grabber,
    CreatedAt,
    StartedAt,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SelectedPageFilter {
    Kind,
    Category,
    Flags,
    Title,
    Author,
    Narrator,
    Series,
    Language,
    Filetype,
    Cost,
    Grabber,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectedPageColumns {
    pub category: bool,
    pub flags: bool,
    pub authors: bool,
    pub narrators: bool,
    pub series: bool,
    pub language: bool,
    pub size: bool,
    pub filetypes: bool,
    pub grabber: bool,
    pub created_at: bool,
    pub started_at: bool,
    pub removed_at: bool,
}

impl Default for SelectedPageColumns {
    fn default() -> Self {
        Self {
            category: false,
            flags: false,
            authors: true,
            narrators: false,
            series: true,
            language: false,
            size: true,
            filetypes: true,
            grabber: true,
            created_at: true,
            started_at: true,
            removed_at: false,
        }
    }
}

impl SelectedPageColumns {
    fn table_grid_template(self) -> String {
        let mut cols = vec!["30px", if self.category { "130px" } else { "84px" }];
        if self.flags {
            cols.push("60px");
        }
        cols.push("2fr");
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
        cols.push("80px");
        cols.push("120px");
        if self.grabber {
            cols.push("130px");
        }
        if self.created_at {
            cols.push("157px");
        }
        if self.started_at {
            cols.push("157px");
        }
        if self.removed_at {
            cols.push("157px");
        }
        cols.push("44px");
        cols.join(" ")
    }

    pub fn get(self, col: SelectedColumn) -> bool {
        match col {
            SelectedColumn::Category => self.category,
            SelectedColumn::Flags => self.flags,
            SelectedColumn::Authors => self.authors,
            SelectedColumn::Narrators => self.narrators,
            SelectedColumn::Series => self.series,
            SelectedColumn::Language => self.language,
            SelectedColumn::Size => self.size,
            SelectedColumn::Filetypes => self.filetypes,
            SelectedColumn::Grabber => self.grabber,
            SelectedColumn::CreatedAt => self.created_at,
            SelectedColumn::StartedAt => self.started_at,
            SelectedColumn::RemovedAt => self.removed_at,
        }
    }

    pub fn set(&mut self, col: SelectedColumn, enabled: bool) {
        match col {
            SelectedColumn::Category => self.category = enabled,
            SelectedColumn::Flags => self.flags = enabled,
            SelectedColumn::Authors => self.authors = enabled,
            SelectedColumn::Narrators => self.narrators = enabled,
            SelectedColumn::Series => self.series = enabled,
            SelectedColumn::Language => self.language = enabled,
            SelectedColumn::Size => self.size = enabled,
            SelectedColumn::Filetypes => self.filetypes = enabled,
            SelectedColumn::Grabber => self.grabber = enabled,
            SelectedColumn::CreatedAt => self.created_at = enabled,
            SelectedColumn::StartedAt => self.started_at = enabled,
            SelectedColumn::RemovedAt => self.removed_at = enabled,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectedSeries {
    pub name: String,
    pub entries: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectedMeta {
    pub title: String,
    pub media_type: String,
    pub cat_name: String,
    pub cat_id: Option<String>,
    pub flags: Vec<String>,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<SelectedSeries>,
    pub language: Option<String>,
    pub size: String,
    pub filetypes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectedRow {
    pub mam_id: u64,
    pub meta: SelectedMeta,
    pub cost: String,
    pub required_unsats: u64,
    pub grabber: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub removed_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectedUserInfo {
    pub unsat_count: u64,
    pub unsat_limit: u64,
    pub wedges: u64,
    pub bonus: i64,
    pub remaining_buffer: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct SelectedData {
    pub torrents: Vec<SelectedRow>,
    pub user_info: Option<SelectedUserInfo>,
    pub queued: usize,
    pub downloading: usize,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SelectedBulkAction {
    Remove,
    Update,
}

impl SelectedBulkAction {
    fn label(self) -> &'static str {
        match self {
            Self::Remove => "unselect for download",
            Self::Update => "set required unsats to",
        }
    }

    fn success_label(self) -> &'static str {
        match self {
            Self::Remove => "Updated selected torrents",
            Self::Update => "Updated required unsats",
        }
    }
}

#[server]
pub async fn get_selected_data(
    sort: Option<SelectedPageSort>,
    asc: bool,
    filters: Vec<(SelectedPageFilter, String)>,
    show: SelectedPageColumns,
) -> Result<SelectedData, ServerFnError> {
    let context = crate::error::get_context()?;
    let config = context.config().await;

    let mut torrents = context
        .db()
        .r_transaction()
        .server_err()?
        .scan()
        .primary::<SelectedTorrent>()
        .server_err()?
        .all()
        .server_err()?
        .filter_map(Result::ok)
        .filter(|t| show.removed_at || t.removed_at.is_none())
        .filter(|t| {
            filters.iter().all(|(field, value)| match field {
                SelectedPageFilter::Kind => t.meta.media_type.as_str() == value,
                SelectedPageFilter::Category => {
                    if value.is_empty() {
                        t.meta.cat.is_none()
                    } else if let Some(cat) = &t.meta.cat {
                        let cats = value
                            .split(',')
                            .filter_map(|id| id.parse().ok())
                            .filter_map(OldCategory::from_one_id)
                            .collect::<Vec<_>>();
                        cats.contains(cat) || cat.as_str() == value
                    } else {
                        false
                    }
                }
                SelectedPageFilter::Flags => {
                    if value.is_empty() {
                        t.meta.flags.is_none_or(|f| f.0 == 0)
                    } else if let Some(flags) = &t.meta.flags {
                        let flags = Flags::from_bitfield(flags.0);
                        match value.as_str() {
                            "violence" => flags.violence == Some(true),
                            "explicit" => flags.explicit == Some(true),
                            "some_explicit" => flags.some_explicit == Some(true),
                            "language" => flags.crude_language == Some(true),
                            "abridged" => flags.abridged == Some(true),
                            "lgbt" => flags.lgbt == Some(true),
                            _ => false,
                        }
                    } else {
                        false
                    }
                }
                SelectedPageFilter::Title => t.meta.title == *value,
                SelectedPageFilter::Author => t.meta.authors.contains(value),
                SelectedPageFilter::Narrator => t.meta.narrators.contains(value),
                SelectedPageFilter::Series => t.meta.series.iter().any(|s| &s.name == value),
                SelectedPageFilter::Language => {
                    if value.is_empty() {
                        t.meta.language.is_none()
                    } else {
                        t.meta.language == Language::from_str(value).ok()
                    }
                }
                SelectedPageFilter::Filetype => t.meta.filetypes.contains(value),
                SelectedPageFilter::Cost => t.cost.as_str() == value,
                SelectedPageFilter::Grabber => {
                    if value.is_empty() {
                        t.grabber.is_none()
                    } else {
                        t.grabber.as_deref() == Some(value)
                    }
                }
            })
        })
        .collect::<Vec<_>>();

    if let Some(sort_by) = sort {
        torrents.sort_by(|a, b| {
            let ord = match sort_by {
                SelectedPageSort::Kind => a.meta.media_type.cmp(&b.meta.media_type),
                SelectedPageSort::Title => a.meta.title.cmp(&b.meta.title),
                SelectedPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                SelectedPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                SelectedPageSort::Series => a.meta.series.cmp(&b.meta.series),
                SelectedPageSort::Language => a.meta.language.cmp(&b.meta.language),
                SelectedPageSort::Size => a.meta.size.cmp(&b.meta.size),
                SelectedPageSort::Cost => a.cost.cmp(&b.cost),
                SelectedPageSort::Buffer => a
                    .unsat_buffer
                    .unwrap_or(config.unsat_buffer)
                    .cmp(&b.unsat_buffer.unwrap_or(config.unsat_buffer)),
                SelectedPageSort::Grabber => a.grabber.cmp(&b.grabber),
                SelectedPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
                SelectedPageSort::StartedAt => a.started_at.cmp(&b.started_at),
            };
            if asc { ord.reverse() } else { ord }
        });
    }

    let queued = torrents.iter().filter(|t| t.started_at.is_none()).count();
    let downloading = torrents.iter().filter(|t| t.started_at.is_some()).count();

    let downloading_size: f64 = context
        .db()
        .r_transaction()
        .server_err()?
        .scan()
        .primary::<SelectedTorrent>()
        .server_err()?
        .all()
        .server_err()?
        .filter_map(Result::ok)
        .filter(|t| t.removed_at.is_none() && t.started_at.is_some())
        .map(|t| t.meta.size.bytes() as f64)
        .sum();

    let user_info = match context.mam() {
        Ok(mam) => mam.user_info().await.ok().map(|user_info| {
            let remaining_buffer = mlm_db::Size::from_bytes(
                ((user_info.uploaded_bytes - user_info.downloaded_bytes - downloading_size)
                    / config.min_ratio) as u64,
            )
            .to_string();
            SelectedUserInfo {
                unsat_count: user_info.unsat.count,
                unsat_limit: user_info.unsat.limit,
                wedges: user_info.wedges,
                bonus: user_info.seedbonus,
                remaining_buffer: Some(remaining_buffer),
            }
        }),
        Err(_) => None,
    };

    Ok(SelectedData {
        torrents: torrents
            .into_iter()
            .map(|t| convert_selected_row(&t, config.unsat_buffer))
            .collect(),
        user_info,
        queued,
        downloading,
    })
}

#[server]
pub async fn apply_selected_action(
    action: SelectedBulkAction,
    mam_ids: Vec<u64>,
    unsats: Option<u64>,
) -> Result<(), ServerFnError> {
    if mam_ids.is_empty() {
        return Err(ServerFnError::new("No torrents selected"));
    }

    let context = crate::error::get_context()?;

    match action {
        SelectedBulkAction::Remove => {
            let (_guard, rw) = context.db().rw_async().await.server_err()?;
            for mam_id in mam_ids {
                let Some(mut torrent) = rw.get().primary::<SelectedTorrent>(mam_id).server_err()?
                else {
                    continue;
                };
                if torrent.removed_at.is_none() {
                    torrent.removed_at = Some(Timestamp::now());
                    rw.upsert(torrent).server_err()?;
                } else {
                    rw.remove(torrent).server_err()?;
                }
            }
            rw.commit().server_err()?;
        }
        SelectedBulkAction::Update => {
            let (_guard, rw) = context.db().rw_async().await.server_err()?;
            for mam_id in mam_ids {
                let Some(mut torrent) = rw.get().primary::<SelectedTorrent>(mam_id).server_err()?
                else {
                    continue;
                };
                torrent.unsat_buffer = Some(unsats.unwrap_or_default());
                torrent.removed_at = None;
                rw.upsert(torrent).server_err()?;
            }
            rw.commit().server_err()?;
        }
    }

    Ok(())
}

#[cfg(feature = "server")]
fn convert_selected_row(t: &SelectedTorrent, default_unsat: u64) -> SelectedRow {
    let flags = Flags::from_bitfield(t.meta.flags.map_or(0, |f| f.0));
    let mut flag_values = Vec::new();
    if flags.crude_language == Some(true) {
        flag_values.push("language".to_string());
    }
    if flags.violence == Some(true) {
        flag_values.push("violence".to_string());
    }
    if flags.some_explicit == Some(true) {
        flag_values.push("some_explicit".to_string());
    }
    if flags.explicit == Some(true) {
        flag_values.push("explicit".to_string());
    }
    if flags.abridged == Some(true) {
        flag_values.push("abridged".to_string());
    }
    if flags.lgbt == Some(true) {
        flag_values.push("lgbt".to_string());
    }

    let (cat_name, cat_id) = if let Some(cat) = &t.meta.cat {
        (cat.as_str().to_string(), Some(cat.as_id().to_string()))
    } else {
        ("N/A".to_string(), None)
    };

    SelectedRow {
        mam_id: t.mam_id,
        meta: SelectedMeta {
            title: t.meta.title.clone(),
            media_type: t.meta.media_type.as_str().to_string(),
            cat_name,
            cat_id,
            flags: flag_values,
            authors: t.meta.authors.clone(),
            narrators: t.meta.narrators.clone(),
            series: t
                .meta
                .series
                .iter()
                .map(|series| SelectedSeries {
                    name: series.name.clone(),
                    entries: series.entries.to_string(),
                })
                .collect(),
            language: t.meta.language.map(|l| l.to_str().to_string()),
            size: t.meta.size.to_string(),
            filetypes: t.meta.filetypes.clone(),
        },
        cost: t.cost.as_str().to_string(),
        required_unsats: t.unsat_buffer.unwrap_or(default_unsat),
        grabber: t.grabber.clone(),
        created_at: format_timestamp_db(&t.created_at),
        started_at: t.started_at.as_ref().map(format_timestamp_db),
        removed_at: t.removed_at.as_ref().map(format_timestamp_db),
    }
}

fn filter_name(filter: SelectedPageFilter) -> &'static str {
    match filter {
        SelectedPageFilter::Kind => "Type",
        SelectedPageFilter::Category => "Category",
        SelectedPageFilter::Flags => "Flags",
        SelectedPageFilter::Title => "Title",
        SelectedPageFilter::Author => "Authors",
        SelectedPageFilter::Narrator => "Narrators",
        SelectedPageFilter::Series => "Series",
        SelectedPageFilter::Language => "Language",
        SelectedPageFilter::Filetype => "Filetypes",
        SelectedPageFilter::Cost => "Cost",
        SelectedPageFilter::Grabber => "Grabber",
    }
}

impl PageColumns for SelectedPageColumns {
    fn to_query_value(&self) -> String {
        let mut values = Vec::new();
        if self.category {
            values.push("category");
        }
        if self.flags {
            values.push("flags");
        }
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
        if self.grabber {
            values.push("grabber");
        }
        if self.created_at {
            values.push("created_at");
        }
        if self.started_at {
            values.push("started_at");
        }
        if self.removed_at {
            values.push("removed_at");
        }
        values.join(",")
    }

    fn from_query_value(value: &str) -> Self {
        let mut show = SelectedPageColumns {
            category: false,
            flags: false,
            authors: false,
            narrators: false,
            series: false,
            language: false,
            size: false,
            filetypes: false,
            grabber: false,
            created_at: false,
            started_at: false,
            removed_at: false,
        };
        for item in value.split(',') {
            match item {
                "category" => show.category = true,
                "flags" => show.flags = true,
                "author" => show.authors = true,
                "narrator" => show.narrators = true,
                "series" => show.series = true,
                "language" => show.language = true,
                "size" => show.size = true,
                "filetype" => show.filetypes = true,
                "grabber" => show.grabber = true,
                "created_at" => show.created_at = true,
                "started_at" => show.started_at = true,
                "removed_at" => show.removed_at = true,
                _ => {}
            }
        }
        show
    }
}

#[derive(Clone, Default)]
struct PageQueryState {
    sort: Option<SelectedPageSort>,
    asc: bool,
    filters: Vec<(SelectedPageFilter, String)>,
    show: SelectedPageColumns,
}

fn parse_query_state() -> PageQueryState {
    let mut state = PageQueryState::default();
    for (key, value) in parse_location_query_pairs() {
        match key.as_str() {
            "sort_by" => state.sort = parse_query_enum::<SelectedPageSort>(&value),
            "asc" => state.asc = value == "true",
            "show" => state.show = SelectedPageColumns::from_query_value(&value),
            _ => {
                if let Some(field) = parse_query_enum::<SelectedPageFilter>(&key) {
                    state.filters.push((field, value));
                }
            }
        }
    }
    state
}

fn build_query_url(
    sort: Option<SelectedPageSort>,
    asc: bool,
    filters: &[(SelectedPageFilter, String)],
    show: SelectedPageColumns,
) -> String {
    let mut params = Vec::new();
    if let Some(sort) = sort.and_then(encode_query_enum) {
        params.push(("sort_by".to_string(), sort));
    }
    if asc {
        params.push(("asc".to_string(), "true".to_string()));
    }
    if show != SelectedPageColumns::default() {
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
enum SelectedColumn {
    Category,
    Flags,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Filetypes,
    Grabber,
    CreatedAt,
    StartedAt,
    RemovedAt,
}

const COLUMN_OPTIONS: &[(SelectedColumn, &str)] = &[
    (SelectedColumn::Category, "Category"),
    (SelectedColumn::Flags, "Flags"),
    (SelectedColumn::Authors, "Authors"),
    (SelectedColumn::Narrators, "Narrators"),
    (SelectedColumn::Series, "Series"),
    (SelectedColumn::Language, "Language"),
    (SelectedColumn::Size, "Size"),
    (SelectedColumn::Filetypes, "Filetypes"),
    (SelectedColumn::Grabber, "Grabber"),
    (SelectedColumn::CreatedAt, "Added At"),
    (SelectedColumn::StartedAt, "Started At"),
    (SelectedColumn::RemovedAt, "Removed At"),
];

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

    rsx! {
        div { class: "selected-page",
            div { class: "row",
                h1 { "Selected Torrents" }
                div { class: "actions actions_torrent",
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

            if let Some(data) = data_to_show.clone() {
                if let Some(user_info) = &data.user_info {
                    p {
                        if let Some(buffer) = &user_info.remaining_buffer {
                            "Buffer: {buffer}"
                            br {}
                        }
                        "Unsats: {user_info.unsat_count} / {user_info.unsat_limit}"
                        br {}
                        "Wedges: {user_info.wedges}"
                        br {}
                        "Bonus: {user_info.bonus}"
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
                                    div { class: "header", "" }
                                }
                            }
                        }

                        for torrent in data.torrents {
                            {
                                let row_id = torrent.mam_id;
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
                                            FilterLink {
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
                                        div {
                                            a {
                                                href: "https://www.myanonamouse.net/t/{torrent.mam_id}",
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
            } else if let Some(value) = &value {
                if let Some(Err(e)) = &*value.read() {
                    p { class: "error", "Error: {e}" }
                } else {
                    p { "Loading selected torrents..." }
                }
            } else {
                p { "Loading selected torrents..." }
            }
        }
    }
}
