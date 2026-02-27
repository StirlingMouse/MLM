use crate::components::SearchTorrentRow;
use crate::components::StatusMessage;
use crate::components::parse_location_query_pairs;
use crate::dto::Series;
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
        description: mam_torrent.description,
        categories: meta.categories.clone(),
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

fn search_state_from_params(params: &[(String, String)]) -> (String, String, String, Option<u64>) {
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
    (query, sort, uploader_input, uploader)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SearchData {
    pub torrents: Vec<SearchTorrent>,
    pub total: usize,
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
    pub description: Option<String>,
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

#[server]
pub async fn get_search_data(
    q: String,
    sort: String,
    uploader: Option<u64>,
) -> Result<SearchData, ServerFnError> {
    #[cfg(feature = "server")]
    {
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
                tor: Tor {
                    target: uploader.map(SearchTarget::Uploader),
                    text: q,
                    ..Default::default()
                },
                ..Default::default()
            })
            .await
            .server_err()?;

        let search_config = context.config().await.search.clone();
        let r = context.db().r_transaction().server_err()?;

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
                let a_series = a
                    .series
                    .iter()
                    .map(|s| format!("{}|{}", s.name, s.entries))
                    .collect::<Vec<_>>()
                    .join(";");
                let b_series = b
                    .series
                    .iter()
                    .map(|s| format!("{}|{}", s.name, s.entries))
                    .collect::<Vec<_>>()
                    .join(";");
                a_series
                    .cmp(&b_series)
                    .then(a.media_type.cmp(&b.media_type))
            });
        }

        let total = torrents.len();
        Ok(SearchData { torrents, total })
    }
    #[cfg(not(feature = "server"))]
    {
        unreachable!()
    }
}

#[component]
pub fn SearchPage() -> Element {
    let _route: crate::app::Route = use_route();
    let params = parse_location_query_pairs();
    let (initial_query, initial_sort, initial_uploader_input, initial_submitted_uploader) =
        search_state_from_params(&params);

    let query_input_initial = initial_query.clone();
    let sort_input_initial = initial_sort.clone();
    let request_query_initial = initial_query;
    let request_sort_initial = initial_sort;
    let route_state_initial = (
        request_query_initial.clone(),
        request_sort_initial.clone(),
        initial_uploader_input.clone(),
    );

    let mut query_input = use_signal(move || query_input_initial.clone());
    let mut sort_input = use_signal(move || sort_input_initial.clone());
    let mut uploader_input = use_signal(move || initial_uploader_input.clone());
    let mut request_query = use_signal(move || request_query_initial.clone());
    let mut request_sort = use_signal(move || request_sort_initial.clone());
    let mut request_uploader = use_signal(move || initial_submitted_uploader);
    let mut last_route_state = use_signal(move || route_state_initial.clone());
    let status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached = use_signal(|| None::<SearchData>);

    let mut data_res = use_server_future(move || async move {
        get_search_data(
            request_query.read().clone(),
            request_sort.read().clone(),
            *request_uploader.read(),
        )
        .await
    })?;

    let current_value = data_res.value();
    let pending = data_res.pending();

    {
        let params = parse_location_query_pairs();
        let (route_query, route_sort, route_uploader_input, route_uploader) =
            search_state_from_params(&params);
        let next_route_state = (
            route_query.clone(),
            route_sort.clone(),
            route_uploader_input.clone(),
        );
        if *last_route_state.read() != next_route_state {
            query_input.set(route_query.clone());
            sort_input.set(route_sort.clone());
            uploader_input.set(route_uploader_input);
            request_query.set(route_query);
            request_sort.set(route_sort);
            request_uploader.set(route_uploader);
            last_route_state.set(next_route_state);
            data_res.restart();
        }
    }

    {
        let value = current_value.read();
        if let Some(Ok(data)) = &*value {
            cached.set(Some(data.clone()));
        }
    }

    let data_to_show = {
        let value = current_value.read();
        match &*value {
            Some(Ok(data)) => Some(data.clone()),
            _ => cached.read().clone(),
        }
    };

    rsx! {
        div { class: "search-page",
            form {
                class: "row",
                onsubmit: move |ev: Event<FormData>| {
                    ev.prevent_default();
                    request_query.set(query_input.read().clone());
                    request_sort.set(sort_input.read().clone());
                    let uploader = uploader_input.read().trim().parse::<u64>().ok();
                    request_uploader.set(uploader);
                    data_res.restart();
                },
                h1 { "MaM Search" }
                input {
                    r#type: "text",
                    value: "{query_input}",
                    placeholder: "Search torrents...",
                    oninput: move |ev| query_input.set(ev.value()),
                }
                button { r#type: "submit", "Search" }
            }

            StatusMessage { status_msg }

            if pending && cached.read().is_some() {
                p { class: "loading-indicator", "Refreshing..." }
            }

            if let Some(data) = data_to_show {
                p { class: "faint", "Showing {data.total} torrents" }
                if data.torrents.is_empty() {
                    p {
                        i { "No torrents found" }
                    }
                } else {
                    div { class: "Torrents",
                        for torrent in data.torrents {
                            SearchTorrentRow {
                                torrent,
                                status_msg,
                                on_refresh: move |_| data_res.restart(),
                            }
                        }
                    }
                }
            } else if let Some(Err(e)) = &*current_value.read() {
                p { class: "error", "Error: {e}" }
            } else {
                p { "Loading search results..." }
            }
        }
    }
}
