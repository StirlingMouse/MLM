use crate::components::Pagination;
use crate::dto::{Event, EventType, Torrent, TorrentCost};
use crate::sse::EVENTS_UPDATE_TRIGGER;
use crate::utils::format_size;
use dioxus::prelude::*;

use super::server_fns::get_events_data;
use super::types::EventData;

#[component]
pub fn EventsPage() -> Element {
    let show = use_signal(|| None::<String>);
    let grabber = use_signal(|| None::<String>);
    let linker = use_signal(|| None::<String>);
    let category = use_signal(|| None::<String>);
    let has_updates = use_signal(|| None::<String>);
    let field = use_signal(|| None::<String>);
    let from = use_signal(|| 0usize);
    let page_size = use_signal(|| 500usize);

    let mut cached_data = use_signal(|| None::<EventData>);

    let mut event_data = match use_server_future(move || async move {
        get_events_data(
            show.read().clone(),
            grabber.read().clone(),
            linker.read().clone(),
            category.read().clone(),
            has_updates.read().clone(),
            field.read().clone(),
            Some(*from.read()),
            Some(*page_size.read()),
        )
        .await
    }) {
        Ok(resource) => resource,
        Err(_) => {
            return rsx! {
                div { class: "events-page",
                    EventsHeader {
                        show: show,
                        grabber: grabber,
                        linker: linker,
                        category: category,
                        has_updates: has_updates,
                        page_size: page_size,
                        from: from,
                    }
                    p { "Loading..." }
                }
            };
        }
    };

    use_effect(move || {
        let _ = *EVENTS_UPDATE_TRIGGER.read();
        event_data.restart();
    });

    let current_value = event_data.value();
    let is_loading = event_data.pending();

    {
        let val = current_value.read();
        if let Some(Ok(data)) = &*val {
            cached_data.set(Some(data.clone()));
        }
    }

    let data_to_show = {
        let val = current_value.read();
        match &*val {
            Some(Ok(data)) => data.clone(),
            Some(Err(_)) | None => cached_data.read().clone().unwrap_or_default(),
        }
    };
    let has_error = matches!(&*current_value.read(), Some(Err(_)));
    let show_loading = is_loading && cached_data.read().is_some();

    rsx! {
        div { class: "events-page",
            EventsHeader {
                show: show,
                grabber: grabber,
                linker: linker,
                category: category,
                has_updates: has_updates,
                page_size: page_size,
                from: from,
            }

            if has_error {
                if let Some(Err(e)) = &*current_value.read() {
                    p { class: "error", "Error: {e}" }
                }
            } else {
                EventsTable {
                    data: data_to_show,
                    from: from,
                    loading: show_loading
                }
            }
        }
    }
}

#[component]
fn EventsHeader(
    mut show: Signal<Option<String>>,
    mut grabber: Signal<Option<String>>,
    mut linker: Signal<Option<String>>,
    mut category: Signal<Option<String>>,
    mut has_updates: Signal<Option<String>>,
    mut page_size: Signal<usize>,
    mut from: Signal<usize>,
) -> Element {
    rsx! {
        div { class: "row",
            h1 { "Events (Dioxus)" }
            div { class: "option_group query",
                "Show: "
                button {
                    class: if show.read().is_none() { "active" },
                    onclick: move |_| {
                        show.set(None);
                        from.set(0);
                    },
                    "All"
                }
                button {
                    class: if show.read().as_deref() == Some("grabber") { "active" },
                    onclick: move |_| {
                        show.set(Some("grabber".to_string()));
                        from.set(0);
                    },
                    "Grabber"
                }
                button {
                    class: if show.read().as_deref() == Some("linker") { "active" },
                    onclick: move |_| {
                        show.set(Some("linker".to_string()));
                        from.set(0);
                    },
                    "Linker"
                }
                button {
                    class: if show.read().as_deref() == Some("cleaner") { "active" },
                    onclick: move |_| {
                        show.set(Some("cleaner".to_string()));
                        from.set(0);
                    },
                    "Cleaner"
                }
                button {
                    class: if show.read().as_deref() == Some("updated") { "active" },
                    onclick: move |_| {
                        show.set(Some("updated".to_string()));
                        from.set(0);
                    },
                    "Updated"
                }
                button {
                    class: if show.read().as_deref() == Some("removed") { "active" },
                    onclick: move |_| {
                        show.set(Some("removed".to_string()));
                        from.set(0);
                    },
                    "Removed"
                }
            }
            div { class: "option_group query",
                "Filters: "
                button {
                    class: if has_updates.read().is_some() { "active" },
                    onclick: move |_| {
                        if has_updates.read().is_some() {
                            has_updates.set(None);
                        } else {
                            has_updates.set(Some("true".to_string()));
                        }
                        from.set(0);
                    },
                    "Has Updates"
                }
                if let Some(l) = linker.read().clone() {
                    label { class: "active",
                        "Linker: {l} "
                        button {
                            r#type: "button",
                            "aria-label": "Remove linker filter",
                            onclick: move |_| {
                                linker.set(None);
                                from.set(0);
                            },
                            "×"
                        }
                    }
                }
                if let Some(g) = grabber.read().clone() {
                    label { class: "active",
                        "Grabber: {g} "
                        button {
                            r#type: "button",
                            "aria-label": "Remove grabber filter",
                            onclick: move |_| {
                                grabber.set(None);
                                from.set(0);
                            },
                            "×"
                        }
                    }
                }
                if let Some(c) = category.read().clone() {
                    label { class: "active",
                        "Category: {c} "
                        button {
                            r#type: "button",
                            "aria-label": "Remove category filter",
                            onclick: move |_| {
                                category.set(None);
                                from.set(0);
                            },
                            "×"
                        }
                    }
                }
            }
            div { class: "option_group query",
                "Page size: "
                select {
                    value: "{page_size}",
                    onchange: move |ev| {
                        if let Ok(v) = ev.value().parse::<usize>() {
                            page_size.set(v);
                            from.set(0);
                        }
                    },
                    option { value: "100", "100" }
                    option { value: "500", "500" }
                    option { value: "1000", "1000" }
                    option { value: "5000", "5000" }
                }
            }
        }
    }
}

#[component]
fn EventsTable(data: EventData, mut from: Signal<usize>, loading: bool) -> Element {
    rsx! {
        div { id: "events-table-container",
            if loading {
                div { class: "loading-indicator", "Updating..." }
            }
            if data.events.is_empty() {
                p { i { "No events yet" } }
            } else {
                div { id: "events-list", class: "EventsTable table",
                    for item in data.events.clone() {
                        EventListItem {
                            event: item.event,
                            torrent: item.torrent,
                            replacement: item.replacement,
                            show_created_at: true,
                        }
                    }
                }
                Pagination {
                    total: data.total,
                    from: *from.read(),
                    page_size: data.page_size,
                    on_change: move |new_from| {
                        from.set(new_from);
                    }
                }
            }
        }
    }
}

#[component]
pub fn EventListItem(
    event: Event,
    torrent: Option<Torrent>,
    replacement: Option<Torrent>,
    show_created_at: bool,
) -> Element {
    rsx! {
        if show_created_at {
            div { "{event.created_at}" }
        }
        div {
            EventContent {
                event,
                torrent,
                replacement,
            }
        }
    }
}

#[component]
pub fn EventContent(
    event: Event,
    torrent: Option<Torrent>,
    replacement: Option<Torrent>,
) -> Element {
    let media_type = torrent
        .as_ref()
        .map(|t| t.meta.media_type.clone())
        .unwrap_or_default();
    let title = torrent.as_ref().map(|t| t.meta.title.clone());
    let torrent_id = torrent.as_ref().map(|t| t.id.clone());
    let category = torrent.as_ref().and_then(|t| t.category.clone());

    let render_torrent_link = |id: Option<String>, title: Option<String>| {
        if let (Some(id), Some(title)) = (id, title) {
            rsx! { a { href: "/dioxus/torrents/{id}", "{title}" } }
        } else {
            rsx! { "" }
        }
    };

    match event.event {
        EventType::Grabbed {
            grabber,
            cost,
            wedged,
        } => {
            let cost_text = if wedged {
                " using a wedge".to_string()
            } else {
                match cost {
                    Some(TorrentCost::Vip) => " as VIP".to_string(),
                    Some(TorrentCost::GlobalFreeleech) => " as Freeleech".to_string(),
                    Some(TorrentCost::PersonalFreeleech) => " as Personal Freeleech".to_string(),
                    Some(TorrentCost::Ratio) => " using ratio".to_string(),
                    _ => "".to_string(),
                }
            };

            rsx! {
                "Grabbed {media_type} Torrent "
                {render_torrent_link(torrent_id, title)}
                "{cost_text}"
                if let Some(g) = grabber {
                    " with grabber {g}"
                }
                if let Some(c) = category {
                    " (category: {c})"
                }
                br {}
            }
        }
        EventType::Linked {
            linker,
            library_path,
        } => {
            let files = torrent
                .as_ref()
                .map(|t| t.library_files.clone())
                .unwrap_or_default();

            rsx! {
                "Linked {media_type} Torrent "
                {render_torrent_link(torrent_id, title)}
                if let Some(l) = linker {
                    " with linker {l}"
                }
                if let Some(c) = category {
                    " (category: {c})"
                }
                br {}
                "to: {library_path.to_string_lossy()}"
                br {}
                if !files.is_empty() {
                    details {
                        summary { "Files" }
                        ul {
                            for f in files {
                                li { "{f.to_string_lossy()}" }
                            }
                        }
                    }
                }
            }
        }
        EventType::Cleaned {
            library_path,
            files,
        } => {
            let size = torrent
                .as_ref()
                .map(|t| format_size(t.meta.size))
                .unwrap_or_default();
            let formats = torrent
                .as_ref()
                .map(|t| t.meta.filetypes.join(", "))
                .unwrap_or_default();

            let r_id = replacement.as_ref().map(|t| t.id.clone());
            let r_title = replacement.as_ref().map(|t| t.meta.title.clone());
            let r_size = replacement
                .as_ref()
                .map(|t| format_size(t.meta.size))
                .unwrap_or_default();
            let r_formats = replacement
                .as_ref()
                .map(|t| t.meta.filetypes.join(", "))
                .unwrap_or_default();
            let r_path = replacement
                .as_ref()
                .and_then(|t| t.library_path.as_ref())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            rsx! {
                "Cleaned {media_type} Torrent "
                {render_torrent_link(torrent_id, title)}
                if let Some(c) = category {
                    " (category: {c})"
                }
                br {}
                if torrent.is_some() {
                    "size: {size}" br {}
                    "formats: {formats}" br {}
                }
                "from: {library_path.to_string_lossy()}"
                br {}
                if replacement.is_some() {
                    br {} "replaced with: " {render_torrent_link(r_id, r_title)} br {}
                    "size: {r_size}" br {}
                    "formats: {r_formats}" br {}
                    if !r_path.is_empty() {
                        "in: {r_path}" br {}
                    }
                }
                br {}
                details {
                    summary { "Removed files" }
                    ul {
                        for f in files {
                            li { "{f.to_string_lossy()}" }
                        }
                    }
                }
            }
        }
        EventType::Updated { fields, source } => {
            let source_text = format!("{} {}", source.0, source.1);
            rsx! {
                "Updated {media_type} Torrent "
                {render_torrent_link(torrent_id, title)}
                " from {source_text}"
                if let Some(c) = category {
                    " (category: {c})"
                }
                br {}
                ul {
                    for f in fields {
                        li { "{f.field}: {f.from} → {f.to}" }
                    }
                }
            }
        }
        EventType::RemovedFromTracker => rsx! {
            "{media_type} Torrent "
            {render_torrent_link(torrent_id, title)}
            " was removed from Tracker"
            if let Some(c) = category {
                " (category: {c})"
            }
            br {}
        },
    }
}
