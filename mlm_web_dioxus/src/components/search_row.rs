use super::{DownloadButtonMode, SimpleDownloadButtons, flag_icon};
use crate::app::Route;
use crate::search::SearchTorrent;
use dioxus::prelude::*;
use lucide_dioxus::{BookText, Mic, UserPen};

fn media_icon_src(mediatype: u8, main_cat: u8) -> Option<&'static str> {
    match (mediatype, main_cat) {
        (1, 1) => Some("/assets/icons/mediatypes/AudiobookF.png"),
        (1, 2) => Some("/assets/icons/mediatypes/AudiobookNF.png"),
        (1, _) => Some("/assets/icons/mediatypes/Audiobook.png"),
        (2, 1) => Some("/assets/icons/mediatypes/EBookF.png"),
        (2, 2) => Some("/assets/icons/mediatypes/EBookNF.png"),
        (2, _) => Some("/assets/icons/mediatypes/EBook.png"),
        (3, 1) => Some("/assets/icons/mediatypes/MusicF.png"),
        (3, 2) => Some("/assets/icons/mediatypes/MusicNF.png"),
        (3, _) => Some("/assets/icons/mediatypes/Music.png"),
        (4, 1) => Some("/assets/icons/mediatypes/RadioF.png"),
        (4, 2) => Some("/assets/icons/mediatypes/RadioNF.png"),
        (4, _) => Some("/assets/icons/mediatypes/Radio.png"),
        (5, 1) => Some("/assets/icons/mediatypes/MangaF.png"),
        (5, 2) => Some("/assets/icons/mediatypes/MangaNF.png"),
        (5, _) => Some("/assets/icons/mediatypes/Manga.png"),
        (6, 1) => Some("/assets/icons/mediatypes/ComicsF.png"),
        (6, 2) => Some("/assets/icons/mediatypes/ComicsNF.png"),
        (6, _) => Some("/assets/icons/mediatypes/Comics.png"),
        (7, 1) => Some("/assets/icons/mediatypes/PeriodicalsF.png"),
        (7, 2) => Some("/assets/icons/mediatypes/PeriodicalsNF.png"),
        (7, _) => Some("/assets/icons/mediatypes/Periodicals.png"),
        (8, 1) => Some("/assets/icons/mediatypes/PeriodicalsAudioF.png"),
        (8, 2) => Some("/assets/icons/mediatypes/PeriodicalsAudioNF.png"),
        (8, _) => Some("/assets/icons/mediatypes/PeriodicalsAudio.png"),
        _ => None,
    }
}

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
fn TorrentIcons(vip: bool, personal_freeleech: bool, free: bool, flags: Vec<String>) -> Element {
    rsx! {
        div { class: "Torrenticons", grid_area: "icons",
            if vip {
                img { src: "/assets/icons/vip.png", alt: "VIP", title: "VIP" }
            } else if personal_freeleech {
                img {
                    src: "/assets/icons/freedownload.png",
                    alt: "Personal Freeleech",
                    title: "Personal Freeleech",
                    style: "filter:hue-rotate(180deg)",
                }
            } else if free {
                img {
                    src: "/assets/icons/freedownload.png",
                    alt: "Freeleech",
                    title: "Freeleech",
                }
            }
            for flag in &flags {
                if let Some((src, title)) = flag_icon(flag) {
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
                        to: Route::TorrentDetail {
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
                        src: asset!("/assets/icons/upBig3.png"),
                    }
                }
                span { class: "icon-row",
                    "{torrent.leechers}"
                    img {
                        alt: "leechers",
                        title: "Leechers",
                        src: asset!("/assets/icons/downBig3.png"),
                    }
                }
                span { class: "icon-row",
                    "{torrent.snatches}"
                    img {
                        alt: "snatches",
                        title: "Snatches",
                        src: asset!("/assets/icons/snatched.png"),
                    }
                }
            }
        }
    }
}
