use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "server")]
use mlm_core::ContextExt;
#[cfg(feature = "server")]
use mlm_core::{
    Context, Event as DbEvent, EventKey, EventType as DbEventType,
    MetadataSource as DbMetadataSource, Torrent as DbTorrent, TorrentCost as DbTorrentCost,
    TorrentKey,
};

// Global trigger for SSE updates
pub static EVENTS_UPDATE_TRIGGER: GlobalSignal<u32> = Signal::global(|| 0);

pub fn trigger_events_update() {
    #[cfg(not(feature = "server"))]
    {
        let mut val = EVENTS_UPDATE_TRIGGER.write();
        *val += 1;
    }
}

// Client-side DTOs
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: String,
    pub created_at: String,
    pub event: EventType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    Grabbed {
        grabber: Option<String>,
        cost: Option<TorrentCost>,
        wedged: bool,
    },
    Linked {
        linker: Option<String>,
        library_path: PathBuf,
    },
    Cleaned {
        library_path: PathBuf,
        files: Vec<PathBuf>,
    },
    Updated {
        fields: Vec<TorrentMetaDiff>,
        source: (MetadataSource, String),
    },
    RemovedFromTracker,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentMetaDiff {
    pub field: String,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Torrent {
    pub id: String,
    pub meta: TorrentMeta,
    pub library_path: Option<PathBuf>,
    pub library_files: Vec<PathBuf>,
    pub linker: Option<String>,
    pub category: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentMeta {
    pub title: String,
    pub media_type: String,
    pub size: u64,
    pub filetypes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TorrentCost {
    GlobalFreeleech,
    PersonalFreeleech,
    Vip,
    UseWedge,
    TryWedge,
    Ratio,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MetadataSource {
    Mam,
    Manual,
    File,
    Match,
}

impl std::fmt::Display for MetadataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataSource::Mam => write!(f, "MaM"),
            MetadataSource::Manual => write!(f, "Manual"),
            MetadataSource::File => write!(f, "File"),
            MetadataSource::Match => write!(f, "Match"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EventWithTorrentData {
    pub event: Event,
    pub torrent: Option<Torrent>,
    pub replacement: Option<Torrent>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct EventData {
    pub events: Vec<EventWithTorrentData>,
    pub total: usize,
    pub from: usize,
    pub page_size: usize,
}

#[server]
#[allow(clippy::too_many_arguments)]
pub async fn get_events_data(
    show: Option<String>,
    grabber: Option<String>,
    linker: Option<String>,
    category: Option<String>,
    has_updates: Option<String>,
    field: Option<String>,
    from: Option<usize>,
    page_size: Option<usize>,
) -> Result<EventData, ServerFnError> {
    use anyhow::Context as _;
    use dioxus_fullstack::FullstackContext;
    use time::UtcOffset;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let db = context.db();

    let from_val = from.unwrap_or(0);
    let page_size_val = page_size.unwrap_or(500);

    let r = db
        .r_transaction()
        .context("r_transaction")
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Helper to format timestamp
    let format_timestamp = |ts: &mlm_core::Timestamp| -> String {
        let format =
            time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                .unwrap();
        ts.0.to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC))
            .replace_nanosecond(0)
            .unwrap()
            .format(&format)
            .unwrap_or_default()
    };

    // Helper to convert DB event to client DTO
    let convert_event = |db_event: &DbEvent| -> Event {
        Event {
            id: db_event.id.0.to_string(),
            created_at: format_timestamp(&db_event.created_at),
            event: match &db_event.event {
                DbEventType::Grabbed {
                    grabber,
                    cost,
                    wedged,
                } => EventType::Grabbed {
                    grabber: grabber.clone(),
                    cost: cost.as_ref().map(|c| match c {
                        DbTorrentCost::Vip => TorrentCost::Vip,
                        DbTorrentCost::GlobalFreeleech => TorrentCost::GlobalFreeleech,
                        DbTorrentCost::PersonalFreeleech => TorrentCost::PersonalFreeleech,
                        DbTorrentCost::UseWedge => TorrentCost::UseWedge,
                        DbTorrentCost::TryWedge => TorrentCost::TryWedge,
                        DbTorrentCost::Ratio => TorrentCost::Ratio,
                    }),
                    wedged: *wedged,
                },
                DbEventType::Linked {
                    linker,
                    library_path,
                } => EventType::Linked {
                    linker: linker.clone(),
                    library_path: library_path.clone(),
                },
                DbEventType::Cleaned {
                    library_path,
                    files,
                } => EventType::Cleaned {
                    library_path: library_path.clone(),
                    files: files.clone(),
                },
                DbEventType::Updated { fields, source } => EventType::Updated {
                    fields: fields
                        .iter()
                        .map(|f| TorrentMetaDiff {
                            field: f.field.to_string(),
                            from: f.from.clone(),
                            to: f.to.clone(),
                        })
                        .collect(),
                    source: (
                        match source.0 {
                            DbMetadataSource::Mam => MetadataSource::Mam,
                            DbMetadataSource::Manual => MetadataSource::Manual,
                            DbMetadataSource::File => MetadataSource::File,
                            DbMetadataSource::Match => MetadataSource::Match,
                        },
                        source.1.clone(),
                    ),
                },
                DbEventType::RemovedFromTracker => EventType::RemovedFromTracker,
            },
        }
    };

    // Helper to convert DB torrent to client DTO
    let convert_torrent = |db_torrent: &DbTorrent| -> Torrent {
        Torrent {
            id: db_torrent.id.clone(),
            meta: TorrentMeta {
                title: db_torrent.meta.title.clone(),
                media_type: db_torrent.meta.media_type.as_str().to_string(),
                size: db_torrent.meta.size.bytes(),
                filetypes: db_torrent.meta.filetypes.clone(),
            },
            library_path: db_torrent.library_path.clone(),
            library_files: db_torrent.library_files.clone(),
            linker: db_torrent.linker.clone(),
            category: db_torrent.category.clone(),
        }
    };

    let no_filters = show.is_none()
        && grabber.is_none()
        && linker.is_none()
        && category.is_none()
        && has_updates.is_none()
        && field.is_none();

    if no_filters {
        let total = r
            .len()
            .secondary::<DbEvent>(EventKey::created_at)
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let events_iter = r
            .scan()
            .secondary::<DbEvent>(EventKey::created_at)
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let events = events_iter
            .all()
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .rev()
            .skip(from_val)
            .take(page_size_val);

        let mut result_events = Vec::new();
        for event_res in events {
            let db_event = event_res.map_err(|e| ServerFnError::new(e.to_string()))?;
            let db_torrent: Option<DbTorrent> = if let Some(id) = &db_event.torrent_id {
                r.get().primary(id.clone()).ok().flatten()
            } else if let Some(mam_id) = &db_event.mam_id {
                r.get()
                    .secondary(TorrentKey::mam_id, *mam_id)
                    .ok()
                    .flatten()
            } else {
                None
            };

            let mut db_replacement = None;
            if let Some(ref t) = db_torrent {
                db_replacement = t
                    .replaced_with
                    .clone()
                    .and_then(|(id, _)| r.get().primary(id).ok().flatten());
            }

            result_events.push(EventWithTorrentData {
                event: convert_event(&db_event),
                torrent: db_torrent.as_ref().map(convert_torrent),
                replacement: db_replacement.as_ref().map(convert_torrent),
            });
        }

        return Ok(EventData {
            events: result_events,
            total: total as usize,
            from: from_val,
            page_size: page_size_val,
        });
    }

    let events_iter = r
        .scan()
        .secondary::<DbEvent>(EventKey::created_at)
        .context("scan")
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let events = events_iter
        .all()
        .context("all")
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .rev();

    let mut result_events = Vec::new();
    let mut total_matching = 0;

    let needs_torrent_for_filter = linker.is_some() || category.is_some();

    for event_res in events {
        let db_event = event_res.map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut event_matches = true;

        if let Some(ref val) = show {
            match &db_event.event {
                DbEventType::Grabbed { .. } => {
                    if val != "grabber" {
                        event_matches = false;
                    }
                }
                DbEventType::Linked { .. } => {
                    if val != "linker" {
                        event_matches = false;
                    }
                }
                DbEventType::Cleaned { .. } => {
                    if val != "cleaner" {
                        event_matches = false;
                    }
                }
                DbEventType::Updated { .. } => {
                    if val != "updated" {
                        event_matches = false;
                    }
                }
                DbEventType::RemovedFromTracker => {
                    if val != "removed" {
                        event_matches = false;
                    }
                }
            }
        }

        if event_matches && let Some(ref val) = grabber {
            match &db_event.event {
                DbEventType::Grabbed { grabber, .. } => {
                    if val.is_empty() {
                        if grabber.is_some() {
                            event_matches = false;
                        }
                    } else if grabber.as_ref() != Some(val) {
                        event_matches = false;
                    }
                }
                _ => {
                    event_matches = false;
                }
            }
        }

        if event_matches && has_updates.is_some() {
            match &db_event.event {
                DbEventType::Updated { fields, .. } => {
                    if !fields.iter().any(|f| !f.from.is_empty()) {
                        event_matches = false;
                    }
                }
                _ => {
                    event_matches = false;
                }
            }
        }

        if event_matches && let Some(ref val) = field {
            match &db_event.event {
                DbEventType::Updated { fields, .. } => {
                    if !fields.iter().any(|f| &f.field.to_string() == val) {
                        event_matches = false;
                    }
                }
                _ => {
                    event_matches = false;
                }
            }
        }

        if !event_matches {
            continue;
        }

        let mut torrent_matches = true;
        let mut db_torrent: Option<DbTorrent> = None;
        let mut db_replacement = None;

        let in_page = total_matching >= from_val && total_matching < from_val + page_size_val;

        if needs_torrent_for_filter || in_page {
            db_torrent = if let Some(id) = &db_event.torrent_id {
                r.get().primary(id.clone()).ok().flatten()
            } else if let Some(mam_id) = &db_event.mam_id {
                r.get()
                    .secondary(TorrentKey::mam_id, *mam_id)
                    .ok()
                    .flatten()
            } else {
                None
            };

            if let Some(ref t) = db_torrent {
                if let Some(ref val) = linker {
                    if t.linker.as_ref() != Some(val) {
                        torrent_matches = false;
                    }
                }
                if let Some(ref val) = category {
                    let cat_matches = if val.is_empty() {
                        t.category.is_none()
                    } else {
                        t.category.as_ref() == Some(val)
                    };
                    if !cat_matches {
                        torrent_matches = false;
                    }
                }

                if torrent_matches && in_page {
                    db_replacement = t
                        .replaced_with
                        .clone()
                        .and_then(|(id, _)| r.get().primary(id).ok().flatten());
                }
            } else if needs_torrent_for_filter {
                torrent_matches = false;
            }
        }

        if torrent_matches {
            if in_page {
                result_events.push(EventWithTorrentData {
                    event: convert_event(&db_event),
                    torrent: db_torrent.as_ref().map(convert_torrent),
                    replacement: db_replacement.as_ref().map(convert_torrent),
                });
            }
            total_matching += 1;
        }
    }

    Ok(EventData {
        events: result_events,
        total: total_matching,
        from: from_val,
        page_size: page_size_val,
    })
}

#[component]
pub fn EventsPage() -> Element {
    let mut show = use_signal(|| None::<String>);
    let mut grabber = use_signal(|| None::<String>);
    let mut linker = use_signal(|| None::<String>);
    let mut category = use_signal(|| None::<String>);
    let mut has_updates = use_signal(|| None::<String>);
    let field = use_signal(|| None::<String>);
    let mut from = use_signal(|| 0usize);
    let mut page_size = use_signal(|| 500usize);

    let mut event_data = use_server_future(move || async move {
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
    })?;

    use_effect(move || {
        let _ = *EVENTS_UPDATE_TRIGGER.read();
        event_data.restart();
    });

    let data = event_data.suspend()?;
    let data = data.read();

    rsx! {
        div { class: "events-page",
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
                                onclick: move |_| {
                                    linker.set(None);
                                    from.set(0);
                                },
                                "[x]"
                            }
                        }
                    }
                    if let Some(g) = grabber.read().clone() {
                        label { class: "active",
                            "Grabber: {g} "
                            button {
                                onclick: move |_| {
                                    grabber.set(None);
                                    from.set(0);
                                },
                                "[x]"
                            }
                        }
                    }
                    if let Some(c) = category.read().clone() {
                        label { class: "active",
                            "Category: {c} "
                            button {
                                onclick: move |_| {
                                    category.set(None);
                                    from.set(0);
                                },
                                "[x]"
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

            match &*data {
                Ok(data) => rsx! {
                    EventsTable {
                        data: data.clone(),
                        from: from
                    }
                },
                Err(e) => rsx! { p { class: "error", "Error: {e}" } },
            }
        }
    }
}

#[component]
fn EventsTable(data: EventData, mut from: Signal<usize>) -> Element {
    rsx! {
        div { id: "events-table-container",
            if data.events.is_empty() {
                p { i { "No events yet" } }
            } else {
                div { id: "events-list", class: "EventsTable table",
                    for item in data.events.clone() {
                        div { "{item.event.created_at}" }
                        div {
                            EventContent {
                                event: item.event,
                                torrent: item.torrent,
                                replacement: item.replacement
                            }
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

#[component]
fn Pagination(
    total: usize,
    from: usize,
    page_size: usize,
    on_change: EventHandler<usize>,
) -> Element {
    if page_size == 0 || total <= page_size {
        return rsx! { "" };
    }

    let max_pages = 7;
    let num_pages = (total as f64 / page_size as f64).ceil() as usize;
    let current_page = from / page_size + 1;

    let pages = {
        if num_pages > max_pages {
            let half = max_pages / 2;
            if current_page <= half {
                1..=max_pages
            } else if current_page >= num_pages - half {
                (num_pages - max_pages + 1)..=num_pages
            } else {
                (current_page - half)..=(current_page + half)
            }
        } else {
            1..=num_pages
        }
    };

    rsx! {
        div { class: "pagination",
            if num_pages > max_pages {
                button {
                    class: if current_page == 1 { "disabled" },
                    onclick: move |_| on_change.call(0),
                    "«"
                }
            }
            button {
                class: if current_page == 1 { "disabled" },
                onclick: move |_| on_change.call(from.saturating_sub(page_size)),
                "‹"
            }
            div {
                for p in pages {
                    {
                        let p_from = (p - 1) * page_size;
                        let active = p == current_page;
                        rsx! {
                            button {
                                class: if active { "active" },
                                onclick: move |_| on_change.call(p_from),
                                "{p}"
                            }
                        }
                    }
                }
            }
            button {
                class: if current_page == num_pages { "disabled" },
                onclick: move |_| on_change.call((from + page_size).min((num_pages - 1) * page_size)),
                "›"
            }
            if num_pages > max_pages {
                button {
                    class: if current_page == num_pages { "disabled" },
                    onclick: move |_| on_change.call((num_pages - 1) * page_size),
                    "»"
                }
            }
        }
    }
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
