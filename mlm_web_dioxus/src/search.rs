use crate::components::SearchTorrentRow;
use crate::components::StatusMessage;
use crate::components::{
    Pagination, build_query_string, parse_location_query_pairs, set_location_query_string,
};
use crate::dto::Series;
#[cfg(feature = "server")]
use crate::dto::sanitize_optional_html;
#[cfg(feature = "server")]
use crate::error::IntoServerFnError;
use dioxus::prelude::*;
#[cfg(feature = "server")]
use mlm_mam::search::MaMTorrent;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use mlm_core::{ContextExt, Torrent as DbTorrent, TorrentKey};
#[cfg(feature = "server")]
use mlm_db::Flags;

#[cfg(feature = "server")]
pub fn map_search_torrent(
    mam_torrent: MaMTorrent,
    meta: &mlm_db::TorrentMeta,
    search_config: mlm_core::config::SearchConfig,
    is_downloaded: bool,
    is_selected: bool,
) -> SearchTorrent {
    let can_wedge = search_config
        .wedge_over
        .is_some_and(|wedge_over| meta.size >= wedge_over && !mam_torrent.is_free());
    let media_duration = mam_torrent
        .media_info
        .as_ref()
        .map(|m| m.general.duration.clone());
    let media_format = mam_torrent
        .media_info
        .as_ref()
        .map(|m| format!("{} {}", m.general.format, m.audio.format));
    let audio_bitrate = mam_torrent
        .media_info
        .as_ref()
        .map(|m| format!("{} {}", m.audio.bitrate, m.audio.mode));
    let old_category = meta.cat.as_ref().map(|cat| cat.to_string());
    let flags = Flags::from_bitfield(meta.flags.map_or(0, |f| f.0));
    let flag_values = crate::utils::flags_to_strings(&flags);

    SearchTorrent {
        mam_id: mam_torrent.id,
        mediatype_id: mam_torrent.mediatype,
        main_cat_id: mam_torrent.main_cat,
        lang_code: mam_torrent.lang_code,
        title: meta.title.clone(),
        edition: meta.edition.as_ref().map(|(ed, _)| ed.clone()),
        authors: meta.authors.clone(),
        narrators: meta.narrators.clone(),
        series: meta
            .series
            .iter()
            .map(|s| Series {
                name: s.name.clone(),
                entries: s.entries.to_string(),
            })
            .collect(),
        tags: mam_torrent.tags,
        description_html: sanitize_optional_html(mam_torrent.description),
        categories: meta
            .categories
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
        flags: flag_values,
        old_category,
        media_type: meta.media_type.as_str().to_string(),
        filetypes: meta.filetypes.clone(),
        size: meta.size.to_string(),
        num_files: mam_torrent.numfiles,
        uploaded_at: mam_torrent.added,
        owner_name: mam_torrent.owner_name,
        seeders: mam_torrent.seeders,
        leechers: mam_torrent.leechers,
        snatches: mam_torrent.times_completed,
        comments: mam_torrent.comments,
        media_duration,
        media_format,
        audio_bitrate,
        vip: mam_torrent.vip,
        personal_freeleech: mam_torrent.personal_freeleech,
        free: mam_torrent.free,
        is_downloaded,
        is_selected,
        can_wedge,
    }
}

const SEARCH_PAGE_SIZE: usize = 100;

fn search_state_from_params(
    params: &[(String, String)],
) -> (String, String, String, Option<u64>, usize) {
    let query = params
        .iter()
        .find_map(|(k, v)| (k == "q").then_some(v.clone()))
        .unwrap_or_default();
    let sort = params
        .iter()
        .find_map(|(k, v)| (k == "sort").then_some(v.clone()))
        .unwrap_or_default();
    let uploader_input = params
        .iter()
        .find_map(|(k, v)| (k == "uploader").then_some(v.clone()))
        .unwrap_or_default();
    let uploader = uploader_input.trim().parse::<u64>().ok();
    let page = params
        .iter()
        .find_map(|(k, v)| (k == "page").then_some(v.clone()))
        .and_then(|page| page.parse::<usize>().ok())
        .unwrap_or_default();
    let from = page.saturating_sub(1) * SEARCH_PAGE_SIZE;
    (query, sort, uploader_input, uploader, from)
}

fn search_query_string(query: &str, sort: &str, uploader_input: &str, from: usize) -> String {
    let mut params = Vec::new();
    if !query.is_empty() {
        params.push(("q".to_string(), query.to_string()));
    }
    if !sort.is_empty() {
        params.push(("sort".to_string(), sort.to_string()));
    }
    if !uploader_input.trim().is_empty() {
        params.push(("uploader".to_string(), uploader_input.trim().to_string()));
    }
    let page = from / SEARCH_PAGE_SIZE + 1;
    if page > 1 {
        params.push(("page".to_string(), page.to_string()));
    }
    build_query_string(&params)
}

fn form_text_value(ev: &Event<FormData>, name: &str, fallback: &str) -> String {
    match ev.data().get_first(name) {
        Some(dioxus::html::FormValue::Text(value)) => value,
        _ => fallback.to_string(),
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SearchData {
    pub torrents: Vec<SearchTorrent>,
    pub total: usize,
}

#[derive(Clone, Debug, PartialEq, Props)]
struct SearchResultsProps {
    query: Signal<String>,
    sort: Signal<String>,
    uploader_input: Signal<String>,
    uploader: Signal<Option<u64>>,
    from: Signal<usize>,
    status_msg: Signal<Option<(String, bool)>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchTorrent {
    pub mam_id: u64,
    pub mediatype_id: u8,
    pub main_cat_id: u8,
    pub lang_code: String,
    pub title: String,
    pub edition: Option<String>,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<Series>,
    pub tags: String,
    pub description_html: Option<String>,
    pub categories: Vec<String>,
    pub flags: Vec<String>,
    pub old_category: Option<String>,
    pub media_type: String,
    pub filetypes: Vec<String>,
    pub size: String,
    pub num_files: u64,
    pub uploaded_at: String,
    pub owner_name: String,
    pub seeders: u64,
    pub leechers: u64,
    pub snatches: u64,
    pub comments: u64,
    pub media_duration: Option<String>,
    pub media_format: Option<String>,
    pub audio_bitrate: Option<String>,
    pub vip: bool,
    pub personal_freeleech: bool,
    pub free: bool,
    pub is_downloaded: bool,
    pub is_selected: bool,
    pub can_wedge: bool,
}

#[component]
fn SearchResults(props: SearchResultsProps) -> Element {
    let query = props.query;
    let sort = props.sort;
    let uploader_input = props.uploader_input;
    let uploader = props.uploader;
    let mut from = props.from;
    let status_msg = props.status_msg;
    let mut data_res = use_server_future(move || {
        let query = query.read().clone();
        let sort = sort.read().clone();
        let uploader = *uploader.read();
        let from = *from.read();
        async move { get_search_data(query, sort, uploader, from).await }
    })?;

    let current_value = data_res.value();

    rsx! {
        if let Some(Ok(data)) = &*current_value.read() {
            p { class: "faint", "Found {data.total} torrents" }
            if data.torrents.is_empty() {
                p {
                    i { "No torrents found" }
                }
            } else {
                {
                    let total = data.total;
                    let current_from = if total == 0 {
                        0
                    } else {
                        (*from.read()).min(((total - 1) / SEARCH_PAGE_SIZE) * SEARCH_PAGE_SIZE)
                    };
                    let torrents = data.torrents.clone();

                    rsx! {
                        Pagination {
                            total: total,
                            from: current_from,
                            page_size: SEARCH_PAGE_SIZE,
                            on_change: move |next_from| {
                                set_location_query_string(&search_query_string(
                                    &query.read(),
                                    &sort.read(),
                                    &uploader_input.read(),
                                    next_from,
                                ));
                                from.set(next_from);
                            },
                        }
                        div { class: "Torrents",
                            for torrent in torrents {
                                SearchTorrentRow {
                                    torrent,
                                    status_msg,
                                    on_refresh: move |_| data_res.restart(),
                                }
                            }
                        }
                        Pagination {
                            total: total,
                            from: current_from,
                            page_size: SEARCH_PAGE_SIZE,
                            on_change: move |next_from| {
                                set_location_query_string(&search_query_string(
                                    &query.read(),
                                    &sort.read(),
                                    &uploader_input.read(),
                                    next_from,
                                ));
                                from.set(next_from);
                            },
                        }
                    }
                }
            }
        } else if let Some(Err(e)) = &*current_value.read() {
            p { class: "error", "Error: {e}" }
        }
    }
}

#[server]
pub async fn get_search_data(
    q: String,
    sort: String,
    uploader: Option<u64>,
    from: usize,
) -> Result<SearchData, ServerFnError> {
    use mlm_mam::{
        enums::SearchTarget,
        search::{SearchFields, SearchQuery, Tor},
    };

    let context = crate::error::get_context()?;

    let mam = context.mam().server_err()?;
    let result = mam
        .search(&SearchQuery {
            fields: SearchFields {
                media_info: true,
                ..Default::default()
            },
            perpage: SEARCH_PAGE_SIZE as u64,
            tor: Tor {
                target: uploader.map(SearchTarget::Uploader),
                text: q,
                start_number: from as u64,
                ..Default::default()
            },
            ..Default::default()
        })
        .await
        .server_err()?;

    let search_config = context.config().await.search.clone();
    let r = context.db().r_transaction().server_err()?;

    let page_len = result.data.len();

    let mut torrents = result
        .data
        .into_iter()
        .map(|mam_torrent| -> Result<SearchTorrent, ServerFnError> {
            let meta = mam_torrent.as_meta().server_err()?;
            let torrent = r
                .get()
                .secondary::<DbTorrent>(TorrentKey::mam_id, meta.mam_id())
                .server_err()?;
            let selected_torrent = r
                .get()
                .primary::<mlm_db::SelectedTorrent>(mam_torrent.id)
                .server_err()?;

            Ok(map_search_torrent(
                mam_torrent,
                &meta,
                search_config.clone(),
                torrent.is_some(),
                selected_torrent.is_some(),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if sort == "series" {
        torrents.sort_by(|a, b| {
            a.series
                .iter()
                .map(|s| (s.name.as_str(), s.entries.as_str()))
                .cmp(
                    b.series
                        .iter()
                        .map(|s| (s.name.as_str(), s.entries.as_str())),
                )
                .then(a.media_type.cmp(&b.media_type))
        });
    }

    Ok(SearchData {
        torrents,
        total: result
            .found
            .max(result.total)
            .max(from.saturating_add(page_len)),
    })
}

#[component]
pub fn SearchPage() -> Element {
    let _route: crate::app::Route = use_route();
    let params = parse_location_query_pairs();
    let (
        initial_query,
        initial_sort,
        initial_uploader_input,
        initial_submitted_uploader,
        initial_from,
    ) = search_state_from_params(&params);

    let query_input_initial = initial_query.clone();
    let sort_input_initial = initial_sort.clone();
    let request_query_initial = initial_query;
    let request_sort_initial = initial_sort;
    let route_state_initial = (
        request_query_initial.clone(),
        request_sort_initial.clone(),
        initial_uploader_input.clone(),
        initial_from,
    );

    let mut query_input = use_signal(move || query_input_initial.clone());
    let mut sort_input = use_signal(move || sort_input_initial.clone());
    let mut uploader_input = use_signal(move || initial_uploader_input.clone());
    let mut request_query = use_signal(move || request_query_initial.clone());
    let mut request_sort = use_signal(move || request_sort_initial.clone());
    let mut request_uploader = use_signal(move || initial_submitted_uploader);
    let mut from = use_signal(move || initial_from);
    let mut last_route_state = use_signal(move || route_state_initial.clone());
    let status_msg = use_signal(|| None::<(String, bool)>);

    {
        let params = parse_location_query_pairs();
        let (route_query, route_sort, route_uploader_input, route_uploader, route_from) =
            search_state_from_params(&params);
        let next_route_state = (
            route_query.clone(),
            route_sort.clone(),
            route_uploader_input.clone(),
            route_from,
        );
        if *last_route_state.read() != next_route_state {
            query_input.set(route_query.clone());
            sort_input.set(route_sort.clone());
            uploader_input.set(route_uploader_input);
            request_query.set(route_query);
            request_sort.set(route_sort);
            request_uploader.set(route_uploader);
            from.set(route_from);
            last_route_state.set(next_route_state);
        }
    }

    rsx! {
        div { class: "search-page",
            form {
                class: "row",
                action: "/search",
                method: "get",
                onsubmit: move |ev: Event<FormData>| {
                    ev.prevent_default();
                    let next_query = form_text_value(&ev, "q", &query_input.read());
                    let next_sort = form_text_value(&ev, "sort", &sort_input.read());
                    let next_uploader_input =
                        form_text_value(&ev, "uploader", &uploader_input.read());
                    let uploader = next_uploader_input.trim().parse::<u64>().ok();
                    let next_route_state = (
                        next_query.clone(),
                        next_sort.clone(),
                        next_uploader_input.clone(),
                        0,
                    );

                    set_location_query_string(&search_query_string(
                        &next_query,
                        &next_sort,
                        &next_uploader_input,
                        0,
                    ));
                    last_route_state.set(next_route_state);
                    request_query.set(next_query);
                    request_sort.set(next_sort);
                    request_uploader.set(uploader);
                    from.set(0);
                },
                h1 { "MaM Search" }
                input {
                    r#type: "text",
                    name: "q",
                    value: "{query_input}",
                    placeholder: "Search torrents...",
                    oninput: move |ev| query_input.set(ev.value()),
                }
                button { r#type: "submit", "Search" }
            }

            StatusMessage { status_msg }

            SuspenseBoundary {
                fallback: |_| rsx! {
                    p { class: "loading-indicator", "Loading search results..." }
                },
                SearchResults {
                    query: request_query,
                    sort: request_sort,
                    uploader_input,
                    uploader: request_uploader,
                    from,
                    status_msg,
                }
            }
        }
    }
}
