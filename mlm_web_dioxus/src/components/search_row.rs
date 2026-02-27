use super::{
    CategoryPills, DownloadButtonMode, SimpleDownloadButtons, TorrentIcons, media_icon_src,
};
use crate::app::Route;
use crate::search::SearchTorrent;
use dioxus::prelude::*;
use lucide_dioxus::{BookText, Mic, UserPen};

fn search_filter_query(prefix: &str, value: &str) -> String {
    let escaped = value.replace('"', "\\\"");
    format!("@{prefix} \"{escaped}\"")
}

pub fn search_filter_href(prefix: &str, value: &str, sort: &str) -> String {
    let mut params = vec![("q".to_string(), search_filter_query(prefix, value))];
    if !sort.is_empty() {
        params.push(("sort".to_string(), sort.to_string()));
    }

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("/dioxus/search?{query}")
}

#[derive(Clone, PartialEq)]
pub struct SearchMetadataFilterItem {
    pub label: String,
    pub href: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SearchMetadataKind {
    Authors,
    Narrators,
    Series,
}

#[component]
pub fn SearchMetadataFilterRow(
    kind: SearchMetadataKind,
    items: Vec<SearchMetadataFilterItem>,
) -> Element {
    if items.is_empty() {
        return rsx! {};
    }

    let title = match kind {
        SearchMetadataKind::Authors => "Authors",
        SearchMetadataKind::Narrators => "Narrators",
        SearchMetadataKind::Series => "Series",
    };

    rsx! {
        div { class: "icon-row",
            span { title: "{title}",
                match kind {
                    SearchMetadataKind::Authors => rsx! {
                        UserPen { size: 16 }
                    },
                    SearchMetadataKind::Narrators => rsx! {
                        Mic { size: 16 }
                    },
                    SearchMetadataKind::Series => rsx! {
                        BookText { size: 16 }
                    },
                }
            }
            for (i , item) in items.iter().enumerate() {
                if i > 0 {
                    ", "
                }
                Link { class: "filter-link", to: item.href.clone(), "{item.label}" }
            }
        }
    }
}

#[component]
pub fn SearchTorrentRow(
    torrent: SearchTorrent,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    let mam_id = torrent.mam_id;
    let uploaded_parts = torrent
        .uploaded_at
        .split_once(' ')
        .map(|(d, t)| (d.to_string(), t.to_string()));
    let authors = torrent
        .authors
        .iter()
        .map(|author| SearchMetadataFilterItem {
            label: author.clone(),
            href: search_filter_href("author", author, ""),
        })
        .collect::<Vec<_>>();
    let narrators = torrent
        .narrators
        .iter()
        .map(|narrator| SearchMetadataFilterItem {
            label: narrator.clone(),
            href: search_filter_href("narrator", narrator, ""),
        })
        .collect::<Vec<_>>();
    let series = torrent
        .series
        .iter()
        .map(|series| SearchMetadataFilterItem {
            label: if series.entries.is_empty() {
                series.name.clone()
            } else {
                format!("{} {}", series.name, series.entries)
            },
            href: search_filter_href("series", &series.name, "series"),
        })
        .collect::<Vec<_>>();
    let torrent_detail_id = mam_id.to_string();

    rsx! {
        div { class: "TorrentRow",
            div { class: "category", grid_area: "category",
                if let Some(src) = media_icon_src(torrent.mediatype_id, torrent.main_cat_id) {
                    img {
                        class: "media-icon",
                        src: "{src}",
                        alt: "{torrent.media_type}",
                        title: "{torrent.media_type}",
                    }
                } else {
                    span { class: "faint", "{torrent.media_type}" }
                }
            }
            TorrentIcons {
                vip: torrent.vip,
                personal_freeleech: torrent.personal_freeleech,
                free: torrent.free,
                flags: torrent.flags.clone(),
            }
            div { grid_area: "main",
                div {
                    if torrent.lang_code != "ENG" {
                        span { class: "faint", "[{torrent.lang_code}] " }
                    }
                    Link {
                        to: Route::TorrentDetailPage {
                            id: torrent_detail_id,
                        },
                        b { "{torrent.title}" }
                    }
                    if let Some(edition) = &torrent.edition {
                        i { class: "faint", " {edition}" }
                    }
                }
                SearchMetadataFilterRow { kind: SearchMetadataKind::Authors, items: authors }
                SearchMetadataFilterRow { kind: SearchMetadataKind::Narrators, items: narrators }
                SearchMetadataFilterRow { kind: SearchMetadataKind::Series, items: series }
                if !torrent.tags.is_empty() {
                    div {
                        i { "{torrent.tags}" }
                    }
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
                CategoryPills {
                    categories: torrent.categories.clone(),
                    old_category: torrent.old_category.clone(),
                }
            }
            div { class: "download", grid_area: "download",
                if torrent.is_selected {
                    span { class: "pill", "Queued" }
                } else if torrent.is_downloaded {
                    span { class: "pill", "Downloaded" }
                } else {
                    SimpleDownloadButtons {
                        mam_id,
                        can_wedge: torrent.can_wedge,
                        disabled: false,
                        mode: DownloadButtonMode::Compact,
                        on_status: move |(msg, is_error)| {
                            status_msg.set(Some((msg, is_error)));
                        },
                        on_refresh: move |_| {
                            on_refresh.call(());
                        },
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
                    img {
                        alt: "seeders",
                        title: "Seeders",
                        src: "/assets/icons/upBig3.png",
                    }
                }
                span { class: "icon-row",
                    "{torrent.leechers}"
                    img {
                        alt: "leechers",
                        title: "Leechers",
                        src: "/assets/icons/downBig3.png",
                    }
                }
                span { class: "icon-row",
                    "{torrent.snatches}"
                    img {
                        alt: "snatches",
                        title: "Snatches",
                        src: "/assets/icons/snatched.png",
                    }
                }
            }
        }
    }
}
