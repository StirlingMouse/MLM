use crate::components::{DownloadButtonMode, SimpleDownloadButtons};
use crate::dto::Series;
#[cfg(feature = "server")]
use crate::error::{IntoServerFnError, OptionIntoServerFnError};
#[cfg(feature = "server")]
use crate::utils::format_series;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use mlm_core::{Context, ContextExt, Torrent as DbTorrent, TorrentKey};

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
    pub categories: Vec<String>,
    pub old_category: Option<String>,
    pub cat_icon_id: Option<u8>,
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
    use dioxus_fullstack::FullstackContext;
    use mlm_mam::{
        enums::SearchTarget,
        search::{SearchFields, SearchQuery, Tor},
    };

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_server_err("Context not found in extensions")?;

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
            let cat_icon_id = meta.cat.as_ref().map(|cat| cat.as_id());

            Ok(SearchTorrent {
                mam_id: mam_torrent.id,
                mediatype_id: mam_torrent.mediatype,
                main_cat_id: mam_torrent.main_cat,
                lang_code: mam_torrent.lang_code,
                title: meta.title,
                edition: meta.edition.as_ref().map(|(ed, _)| ed.clone()),
                authors: meta.authors,
                narrators: meta.narrators,
                series: meta
                    .series
                    .iter()
                    .map(|s| Series {
                        name: s.name.clone(),
                        entries: format_series(s),
                    })
                    .collect(),
                tags: mam_torrent.tags,
                categories: meta.categories,
                old_category,
                cat_icon_id,
                media_type: meta.media_type.as_str().to_string(),
                filetypes: meta.filetypes,
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
                is_downloaded: torrent.is_some(),
                is_selected: selected_torrent.is_some(),
                can_wedge,
            })
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

fn media_icon_src(mediatype: u8, _main_cat: u8) -> Option<&'static str> {
    match mediatype {
        1 => Some("/assets/icons/new/abooks_main2.png"),
        2 => Some("/assets/icons/new/ebooks_main4.png"),
        3 => Some("/assets/icons/new/music_main.png"),
        4 => Some("/assets/icons/new/radiogeneral2.png"),
        _ => None,
    }
}

fn search_filter_query(prefix: &str, value: &str) -> String {
    let escaped = value.replace('"', "\\\"");
    format!("@{prefix} \"{escaped}\"")
}

#[component]
pub fn SearchPage() -> Element {
    let mut query_input = use_signal(String::new);
    let mut sort_input = use_signal(String::new);
    let mut uploader_input = use_signal(String::new);
    let mut submitted_query = use_signal(String::new);
    let mut submitted_sort = use_signal(String::new);
    let mut submitted_uploader = use_signal(|| None::<u64>);
    let status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached = use_signal(|| None::<SearchData>);

    let mut data_res = use_server_future(move || async move {
        get_search_data(
            submitted_query.read().clone(),
            submitted_sort.read().clone(),
            *submitted_uploader.read(),
        )
        .await
    })?;

    let current_value = data_res.value();
    let pending = data_res.pending();

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
            div { class: "row",
                h1 { "Search (Dioxus)" }
                form {
                    class: "search-controls",
                    onsubmit: move |ev: Event<FormData>| {
                        ev.prevent_default();
                        submitted_query.set(query_input.read().clone());
                        submitted_sort.set(sort_input.read().clone());
                        let uploader = uploader_input.read().trim().parse::<u64>().ok();
                        submitted_uploader.set(uploader);
                        data_res.restart();
                    },
                    input {
                        r#type: "text",
                        value: "{query_input}",
                        placeholder: "Search torrents...",
                        oninput: move |ev| query_input.set(ev.value())
                    }
                    select {
                        value: "{sort_input}",
                        onchange: move |ev| sort_input.set(ev.value()),
                        option { value: "", "Default" }
                        option { value: "series", "Series" }
                    }
                    input {
                        r#type: "number",
                        value: "{uploader_input}",
                        placeholder: "Uploader ID",
                        oninput: move |ev| uploader_input.set(ev.value())
                    }
                    button { r#type: "submit", "Search" }
                }
            }

            if let Some((msg, is_error)) = status_msg.read().as_ref() {
                p { class: if *is_error { "error" } else { "loading-indicator" }, "{msg}" }
            }

            if pending && cached.read().is_some() {
                p { class: "loading-indicator", "Refreshing..." }
            }

            if let Some(data) = data_to_show {
                p { class: "faint", "Showing {data.total} torrents" }
                if data.torrents.is_empty() {
                    p { i { "No torrents found" } }
                } else {
                    div { class: "Torrents",
                        for torrent in data.torrents {
                            SearchTorrentRow {
                                torrent: torrent,
                                status_msg: status_msg,
                                on_refresh: move |_| data_res.restart(),
                                on_filter: move |(query, sort): (String, String)| {
                                    query_input.set(query.clone());
                                    submitted_query.set(query);
                                    sort_input.set(sort.clone());
                                    submitted_sort.set(sort);
                                    uploader_input.set(String::new());
                                    submitted_uploader.set(None);
                                    data_res.restart();
                                }
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

#[component]
fn SearchTorrentRow(
    torrent: SearchTorrent,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
    on_filter: EventHandler<(String, String)>,
) -> Element {
    let mam_id = torrent.mam_id;
    let uploaded_parts = torrent
        .uploaded_at
        .split_once(' ')
        .map(|(d, t)| (d.to_string(), t.to_string()));

    rsx! {
        div { class: "TorrentRow",
            div { class: "category", grid_area: "category",
                if let Some(src) = media_icon_src(torrent.mediatype_id, torrent.main_cat_id) {
                    img {
                        class: "media-icon",
                        src: "{src}",
                        alt: "{torrent.media_type}",
                        title: "{torrent.media_type}"
                    }
                } else if let Some(cat_id) = torrent.cat_icon_id {
                    img {
                        src: "/assets/icons/cats/{cat_id}_b.png",
                        alt: "{torrent.media_type}",
                        title: "{torrent.media_type}"
                    }
                } else {
                    span { class: "faint", "{torrent.media_type}" }
                }
            }
            div { class: "icons", grid_area: "icons",
                if torrent.is_selected {
                    span { class: "pill", "Queued" }
                } else if torrent.is_downloaded {
                    span { class: "pill", "Downloaded" }
                } else {
                    SimpleDownloadButtons {
                        mam_id: mam_id,
                        can_wedge: torrent.can_wedge,
                        disabled: false,
                        mode: DownloadButtonMode::Compact,
                        on_status: move |(msg, is_error)| {
                            status_msg.set(Some((msg, is_error)));
                        },
                        on_refresh: move |_| {
                            on_refresh.call(());
                        }
                    }
                }
            }
            div { class: "main", grid_area: "main",
                div {
                    if torrent.lang_code != "ENG" {
                        span { class: "faint", "[{torrent.lang_code}] " }
                    }
                    a { href: "/dioxus/torrents/{mam_id}",
                        b { "{torrent.title}" }
                    }
                    if let Some(edition) = &torrent.edition {
                        i { class: "faint", " {edition}" }
                    }
                }
                if !torrent.authors.is_empty() {
                    div { class: "icon-row",
                        "by "
                        for (i, author) in torrent.authors.iter().enumerate() {
                            if i > 0 {
                                ", "
                            }
                            button {
                                class: "filter-link",
                                onclick: {
                                    let query = search_filter_query("author", author);
                                    move |_| on_filter.call((query.clone(), String::new()))
                                },
                                "{author}"
                            }
                        }
                    }
                }
                if !torrent.narrators.is_empty() {
                    div { class: "icon-row",
                        "narrated by "
                        for (i, narrator) in torrent.narrators.iter().enumerate() {
                            if i > 0 {
                                ", "
                            }
                            button {
                                class: "filter-link",
                                onclick: {
                                    let query = search_filter_query("narrator", narrator);
                                    move |_| on_filter.call((query.clone(), String::new()))
                                },
                                "{narrator}"
                            }
                        }
                    }
                }
                if !torrent.series.is_empty() {
                    div { class: "icon-row",
                        "series "
                        for (i, series) in torrent.series.iter().enumerate() {
                            if i > 0 {
                                ", "
                            }
                            button {
                                class: "filter-link",
                                onclick: {
                                    let query = search_filter_query("series", &series.name);
                                    move |_| on_filter.call((query.clone(), "series".to_string()))
                                },
                                if series.entries.is_empty() {
                                    "{series.name}"
                                } else {
                                    "{series.name} ({series.entries})"
                                }
                            }
                        }
                    }
                }
                if !torrent.tags.is_empty() {
                    div { i { "{torrent.tags}" } }
                }
                div { class: "faint",
                    "{torrent.filetypes.join(\", \")}"
                    if let Some(duration) = &torrent.media_duration {
                        " | {duration}"
                    }
                    if let Some(format) = &torrent.media_format {
                        " | {format}"
                    }
                    if let Some(bitrate) = &torrent.audio_bitrate {
                        " | {bitrate}"
                    }
                    " | {torrent.comments} comments"
                }
                if torrent.old_category.is_some() || !torrent.categories.is_empty() {
                    div { class: "CategoryPills",
                        if let Some(old_category) = &torrent.old_category {
                            span { class: "CategoryPill old", "{old_category}" }
                        }
                        for category in &torrent.categories {
                            if torrent.old_category.as_ref() != Some(category) {
                                span { class: "CategoryPill", "{category}" }
                            }
                        }
                    }
                }
            }
            div { class: "files", grid_area: "files",
                span { "{torrent.num_files}" }
                span { "{torrent.size}" }
                span { "{torrent.filetypes.first().map(|t| t.as_str()).unwrap_or_default()}" }
            }
            div { class: "uploaded", grid_area: "uploaded",
                if let Some((date, time)) = uploaded_parts {
                    span { "{date}" }
                    span { "{time}" }
                } else {
                    span { "{torrent.uploaded_at}" }
                }
                span { "{torrent.owner_name}" }
            }
            div { class: "stats", grid_area: "stats",
                span { class: "icon-row",
                    "{torrent.seeders}"
                    img { alt: "seeders", title: "Seeders", src: "/assets/icons/upBig3.png" }
                }
                span { class: "icon-row",
                    "{torrent.leechers}"
                    img { alt: "leechers", title: "Leechers", src: "/assets/icons/downBig3.png" }
                }
                span { class: "icon-row",
                    "{torrent.snatches}"
                    img { alt: "snatches", title: "Snatches", src: "/assets/icons/snatched.png" }
                }
            }
        }
    }
}
