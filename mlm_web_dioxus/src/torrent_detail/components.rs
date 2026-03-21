use super::server_fns::{
    apply_match_metadata_action, clean_torrent_action, clear_replacement_action,
    get_metadata_providers, get_other_torrents, get_qbit_data, get_torrent_detail,
    match_metadata_action, preview_mam_metadata, preview_match_metadata, refresh_and_relink_action,
    refresh_metadata_action, relink_torrent_action, remove_seeding_files_action,
    remove_torrent_action, set_qbit_category_tags_action, torrent_start_action,
    torrent_stop_action,
};
use super::types::*;
use crate::app::Route;
use crate::components::{
    CategoryPills, Details, DownloadButtonMode, DownloadButtons, SearchMetadataFilterItem,
    SearchMetadataFilterRow, SearchMetadataKind, SearchTorrentRow, StatusMessage, TorrentIcons,
    flag_icon, media_icon_src, search_filter_href,
};
use crate::events::EventListItem;
use crate::search::SearchTorrent;
use dioxus::prelude::*;
use std::pin::Pin;

fn spawn_action(
    name: String,
    mut loading: Signal<bool>,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
    fut: std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ServerFnError>>>>,
) {
    spawn(async move {
        loading.set(true);
        status_msg.set(None);
        match fut.await {
            Ok(_) => {
                status_msg.set(Some((format!("{name} succeeded"), false)));
                on_refresh.call(());
                loading.set(false);
            }
            Err(e) => {
                status_msg.set(Some((format!("{name} failed: {e}"), true)));
                loading.set(false);
            }
        }
    });
}

fn series_label(name: &str, entries: &str) -> String {
    if entries.is_empty() {
        name.to_string()
    } else {
        format!("{name} {entries}")
    }
}

#[component]
fn DetailSidebarStrip(
    media_type: String,
    mediatype_id: u8,
    main_cat_id: u8,
    categories: Vec<String>,
    old_category: Option<String>,
    vip: bool,
    personal_freeleech: bool,
    free: bool,
    flags: Vec<String>,
) -> Element {
    rsx! {
        div { class: "detail-side-strip",
            div { class: "detail-side-copy",
                if let Some(src) = media_icon_src(mediatype_id, main_cat_id) {
                    img {
                        class: "media-icon",
                        src: "{src}",
                        alt: "{media_type}",
                        title: "{media_type}",
                    }
                } else {
                    span { class: "faint", "{media_type}" }
                }
                div { class: "detail-side-copy-body",
                    span { class: "detail-media-pill", "{media_type}" }
                    CategoryPills {
                        categories,
                        old_category,
                    }
                }
            }
            div { class: "detail-side-icons",
                TorrentIcons {
                    vip,
                    personal_freeleech,
                    free,
                    flags,
                }
            }
        }
    }
}

#[component]
fn DetailMetadataRows(
    author_filters: Vec<SearchMetadataFilterItem>,
    narrator_filters: Vec<SearchMetadataFilterItem>,
    series_filters: Vec<SearchMetadataFilterItem>,
    local_tags: Vec<String>,
) -> Element {
    rsx! {
        div { class: "detail-meta-stack",
            if !author_filters.is_empty() {
                div { class: "detail-meta-row",
                    SearchMetadataFilterRow {
                        kind: SearchMetadataKind::Authors,
                        items: author_filters,
                    }
                }
            }
            if !narrator_filters.is_empty() {
                div { class: "detail-meta-row",
                    SearchMetadataFilterRow {
                        kind: SearchMetadataKind::Narrators,
                        items: narrator_filters,
                    }
                }
            }
            if !series_filters.is_empty() {
                div { class: "detail-meta-row",
                    SearchMetadataFilterRow {
                        kind: SearchMetadataKind::Series,
                        items: series_filters,
                    }
                }
            }
            if !local_tags.is_empty() {
                div { class: "detail-local-tags",
                    strong { "MLM Tags" }
                    div { class: "detail-tag-pills",
                        for tag in local_tags {
                            span { class: "pill", "{tag}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DescriptionSection(description_html: String) -> Element {
    rsx! {
        div { class: "torrent-description detail-section",
            Details {
                label: "Description".to_string(),
                open: Some(true),
                div { class: "detail-description-body",
                    div { class: "detail-description-block",
                        div { dangerous_inner_html: "{description_html}" }
                    }

                }
            }
        }
    }
}

#[component]
fn EventHistorySection(events: Vec<crate::dto::Event>) -> Element {
    if events.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "detail-section",
            Details {
                label: "Event History".to_string(),
                open: Some(false),
                div { class: "detail-event-history",
                    for event in events {
                        div { class: "event-item",
                            EventListItem {
                                event,
                                torrent: None,
                                replacement: None,
                                show_created_at: true,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn TorrentDetailPage(id: String) -> Element {
    let status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached_data = use_signal(|| None::<(TorrentPageData, Vec<String>)>);

    let mut data_res = use_server_future(move || {
        let id = id.clone();
        async move {
            // tokio::join! isn't available in WASM, so we run the two fetches
            // concurrently on the server and sequentially on the client.
            #[cfg(feature = "server")]
            {
                tokio::join!(get_torrent_detail(id.clone()), get_metadata_providers())
            }
            #[cfg(not(feature = "server"))]
            {
                let detail = get_torrent_detail(id.clone()).await;
                let providers = get_metadata_providers().await;
                (detail, providers)
            }
        }
    })?;

    let current_value = data_res.value();
    let is_loading = data_res.pending();
    let next_cache = {
        let value = current_value.read();
        match &*value {
            Some((Ok(detail), Ok(providers))) => Some((detail.clone(), providers.clone())),
            _ => None,
        }
    };
    if let Some(next_cache) = next_cache {
        let should_update = cached_data.read().as_ref() != Some(&next_cache);
        if should_update {
            cached_data.set(Some(next_cache));
        }
    }
    let rendered_data = {
        let value = current_value.read();
        match &*value {
            Some((Ok(detail), Ok(providers))) => Some((detail.clone(), providers.clone())),
            _ => cached_data.read().clone(),
        }
    };
    let render_error = if cached_data.read().is_none() {
        let value = current_value.read();
        if let Some((detail, providers)) = &*value {
            detail
                .as_ref()
                .err()
                .or_else(|| providers.as_ref().err())
                .map(|e| e.to_string())
        } else {
            None
        }
    } else {
        None
    };

    rsx! {
        div { class: "torrent-detail-page",
            StatusMessage { status_msg }
            if is_loading && cached_data.read().is_some() {
                p { class: "loading-indicator", "Refreshing..." }
            }
            if let Some((detail, providers)) = rendered_data {
                match detail {
                    TorrentPageData::Downloaded(data) => {
                        rsx! {
                            TorrentDetailContent {
                                data,
                                providers,
                                status_msg,
                                on_refresh: move |_| data_res.restart(),
                            }
                        }
                    }
                    TorrentPageData::MamOnly(data) => {
                        rsx! {
                            TorrentMamContent { data, status_msg, on_refresh: move |_| data_res.restart() }
                        }
                    }
                }
            } else if let Some(e) = render_error {
                p { class: "error", "Error: {e}" }
            } else {
                p { "Loading torrent details..." }
            }
        }
    }
}

#[component]
fn TorrentDetailContent(
    data: TorrentDetailData,
    providers: Vec<String>,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    let TorrentDetailData {
        torrent,
        events,
        replacement_torrent,
        replacement_missing,
        abs_item_url,
        abs_cover_url,
    } = data;

    let library_files = torrent
        .library_files
        .iter()
        .map(|file| {
            let file_name = file.to_string_lossy().to_string();
            let encoded = urlencoding::encode(&file_name).to_string();
            (file_name, encoded)
        })
        .collect::<Vec<_>>();

    let filetypes_text = torrent.filetypes.join(", ");
    let author_filters = torrent
        .authors
        .iter()
        .map(|author| SearchMetadataFilterItem {
            label: author.clone(),
            href: search_filter_href("author", author, ""),
        })
        .collect::<Vec<_>>();
    let narrator_filters = torrent
        .narrators
        .iter()
        .map(|narrator| SearchMetadataFilterItem {
            label: narrator.clone(),
            href: search_filter_href("narrator", narrator, ""),
        })
        .collect::<Vec<_>>();
    let series_filters = torrent
        .series
        .iter()
        .map(|series| SearchMetadataFilterItem {
            label: series_label(&series.name, &series.entries),
            href: search_filter_href("series", &series.name, "series"),
        })
        .collect::<Vec<_>>();
    let library_path = torrent
        .library_path
        .as_ref()
        .map(|path| path.display().to_string());
    let mam_url = torrent
        .mam_id
        .map(|mam_id| format!("https://www.myanonamouse.net/t/{mam_id}"));
    let goodreads_url = torrent
        .goodreads_id
        .as_ref()
        .map(|goodreads_id| format!("https://www.goodreads.com/book/show/{goodreads_id}"));
    let mut qbit_refresh = use_signal(|| 0u32);
    let torrent_id = torrent.id.clone();
    let qbit_data = use_resource(move || {
        let _ = *qbit_refresh.read();
        let torrent_id = torrent_id.clone();
        async move { get_qbit_data(torrent_id).await }
    });
    let qbit_result = qbit_data.read().clone();
    let qbit_wanted_path = qbit_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .and_then(|maybe_qbit| maybe_qbit.as_ref())
        .and_then(|qbit| qbit.wanted_path.as_ref())
        .map(|path| path.display().to_string());
    let qbit_no_longer_wanted = qbit_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .and_then(|maybe_qbit| maybe_qbit.as_ref())
        .is_some_and(|qbit| qbit.no_longer_wanted);
    let on_qbit_refresh = EventHandler::new(move |_| {
        *qbit_refresh.write() += 1;
        on_refresh.call(());
    });

    rsx! {
        div { class: "torrent-detail-grid",
            div { class: "torrent-side",
                if let Some(abs_cover_url) = abs_cover_url.as_ref() {
                    div { class: "abs-cover detail-card",
                        img {
                            src: "{abs_cover_url}",
                            alt: "ABS cover for {torrent.title}",
                            loading: "lazy",
                        }
                    }
                }
                div { class: "detail-card detail-sidebar-card",
                    DetailSidebarStrip {
                        media_type: torrent.media_type.clone(),
                        mediatype_id: torrent.mediatype_id,
                        main_cat_id: torrent.main_cat_id,
                        categories: torrent.categories.clone(),
                        old_category: torrent.old_category.clone(),
                        vip: false,
                        personal_freeleech: false,
                        free: false,
                        flags: torrent.flags.clone(),
                    }

                    h3 { class: "detail-section-title", "Metadata" }
                    dl { class: "metadata-table detail-metadata-table",
                        if let Some(lang) = &torrent.language {
                            dt { "Language" }
                            dd { "{lang}" }
                        }
                        if let Some(ed) = &torrent.edition {
                            dt { "Edition" }
                            dd { "{ed}" }
                        }
                        if let Some(mam_id) = torrent.mam_id {
                            dt { "MaM ID" }
                            dd {
                                a {
                                    href: "https://www.myanonamouse.net/t/{mam_id}",
                                    target: "_blank",
                                    "{mam_id}"
                                }
                            }
                        }
                        dt { "Size" }
                        dd { "{torrent.size}" }
                        dt { "Files" }
                        dd { "{torrent.num_files}" }
                        if !torrent.filetypes.is_empty() {
                            dt { "File Types" }
                            dd { "{filetypes_text}" }
                        }
                        dt { "Uploaded" }
                        dd { "{torrent.uploaded_at}" }
                        dt { "Source" }
                        dd { "{torrent.source}" }
                        if let Some(vip) = &torrent.vip_status {
                            dt { "VIP" }
                            dd { "{vip}" }
                        }
                        if let Some(linker) = &torrent.linker {
                            dt { "Linker" }
                            dd { "{linker}" }
                        }
                        if let Some(cat) = &torrent.category {
                            dt { "Category" }
                            dd { "{cat}" }
                        }
                        if let Some(status) = &torrent.client_status {
                            dt { "Client Status" }
                            dd { "{status}" }
                        }
                    }
                }
                match qbit_result {
                    None => rsx! {
                        p { class: "loading-indicator", "Loading qBittorrent data..." }
                    },
                    Some(Err(_)) | Some(Ok(None)) => rsx! {},
                    Some(Ok(Some(qbit))) => rsx! {
                        QbitControls {
                            torrent_id: torrent.id.clone(),
                            qbit,
                            status_msg,
                            on_refresh: on_qbit_refresh,
                        }
                    },
                }
            }

            div { class: "torrent-main",
                div { class: "detail-hero",
                    h1 { "{torrent.title}" }
                    if let Some(replacement) = replacement_torrent {
                        div { class: "warn detail-alert",
                            strong { "Replaced with: " }
                            a { href: "/torrents/{replacement.id}", "{replacement.title}" }
                        }
                    }
                    if replacement_missing {
                        div { class: "warn detail-alert",
                            "This torrent had a stale replacement link and it was cleared."
                        }
                    }

                    DetailMetadataRows {
                        author_filters,
                        narrator_filters,
                        series_filters,
                        local_tags: torrent.tags.clone(),
                    }

                    TorrentActions {
                        torrent_id: torrent.id.clone(),
                        providers,
                        mam_id: torrent.mam_id,
                        has_replacement: torrent.replaced_with.is_some(),
                        library_path,
                        abs_item_url,
                        mam_url,
                        goodreads_url,
                        qbit_wanted_path,
                        qbit_no_longer_wanted,
                        status_msg,
                        on_refresh,
                    }

                    DescriptionSection {
                        description_html: torrent.description_html.clone(),
                    }

                    EventHistorySection { events }
                }
            }

            div { class: "torrent-below",
                if !library_files.is_empty() {
                    div { class: "detail-section",
                        Details {
                            label: format!("Library Files ({})", library_files.len()),
                            open: Some(false),
                            ul {
                                for file in &library_files {
                                    li {
                                        a {
                                            href: "/torrents/{torrent.id}/files/{file.1}",
                                            target: "_blank",
                                            "{file.0}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                OtherTorrentsSection { id: torrent.id.clone(), status_msg, on_refresh }
            }
        }
    }
}

#[component]
fn TorrentMamContent(
    data: TorrentMamData,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    let torrent = data.meta;
    let mam = data.mam_torrent;

    let filetypes_text = torrent.filetypes.join(", ");
    let author_filters = torrent
        .authors
        .iter()
        .map(|author| SearchMetadataFilterItem {
            label: author.clone(),
            href: search_filter_href("author", author, ""),
        })
        .collect::<Vec<_>>();
    let narrator_filters = torrent
        .narrators
        .iter()
        .map(|narrator| SearchMetadataFilterItem {
            label: narrator.clone(),
            href: search_filter_href("narrator", narrator, ""),
        })
        .collect::<Vec<_>>();
    let series_filters = torrent
        .series
        .iter()
        .map(|series| SearchMetadataFilterItem {
            label: series_label(&series.name, &series.entries),
            href: search_filter_href("series", &series.name, "series"),
        })
        .collect::<Vec<_>>();
    let goodreads_url = torrent
        .goodreads_id
        .as_ref()
        .map(|goodreads_id| format!("https://www.goodreads.com/book/show/{goodreads_id}"));
    let description_html = mam
        .description_html
        .clone()
        .unwrap_or_else(|| torrent.description_html.clone());

    rsx! {
        div { class: "torrent-detail-grid",
            div { class: "torrent-side",
                div { class: "detail-card detail-sidebar-card",
                    DetailSidebarStrip {
                        media_type: torrent.media_type.clone(),
                        mediatype_id: torrent.mediatype_id,
                        main_cat_id: torrent.main_cat_id,
                        categories: torrent.categories.clone(),
                        old_category: torrent.old_category.clone(),
                        vip: mam.vip,
                        personal_freeleech: mam.personal_freeleech,
                        free: mam.free,
                        flags: torrent.flags.clone(),
                    }
                    h3 { class: "detail-section-title", "Metadata" }
                    dl { class: "metadata-table detail-metadata-table",
                        if !torrent.flags.is_empty() {
                            dt { "Flags" }
                            dd {
                                for flag in &torrent.flags {
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
                        dt { "MaM ID" }
                        dd {
                            a {
                                href: "https://www.myanonamouse.net/t/{mam.mam_id}",
                                target: "_blank",
                                "{mam.mam_id}"
                            }
                        }
                        dt { "Uploader" }
                        dd { "{mam.owner_name}" }
                        dt { "Size" }
                        dd { "{torrent.size}" }
                        dt { "Files" }
                        dd { "{torrent.num_files}" }
                        if !torrent.filetypes.is_empty() {
                            dt { "File Types" }
                            dd { "{filetypes_text}" }
                        }
                        dt { "Uploaded" }
                        dd { "{torrent.uploaded_at}" }
                    }
                }
            }
            div { class: "torrent-main",
                div { class: "detail-hero",
                    h1 { "{torrent.title}" }
                    if let Some(ed) = &torrent.edition {
                        p { class: "detail-edition", "{ed}" }
                    }
                    DetailMetadataRows {
                        author_filters,
                        narrator_filters,
                        series_filters,
                        local_tags: torrent.tags.clone(),
                    }
                    div { class: "detail-action-stack",
                        div { class: "detail-action-group",
                            p { class: "detail-action-label", "External" }
                            div { class: "detail-action-row",
                                a {
                                    class: "btn",
                                    href: "https://www.myanonamouse.net/t/{mam.mam_id}",
                                    target: "_blank",
                                    "Open in MaM"
                                }
                                if let Some(goodreads_url) = goodreads_url {
                                    a {
                                        class: "btn",
                                        href: "{goodreads_url}",
                                        target: "_blank",
                                        "Open in Goodreads"
                                    }
                                }
                            }
                        }
                        div { class: "detail-action-group",
                            p { class: "detail-action-label", "Download" }
                            div { class: "detail-action-row",
                                DownloadButtons {
                                    mam_id: mam.mam_id,
                                    is_vip: mam.vip,
                                    is_free: mam.free,
                                    is_personal_freeleech: mam.personal_freeleech,
                                    can_wedge: true,
                                    disabled: false,
                                    mode: DownloadButtonMode::Full,
                                    on_status: move |(msg, is_error)| {
                                        status_msg.set(Some((msg, is_error)));
                                    },
                                    on_refresh: move |_| {
                                        on_refresh.call(());
                                    },
                                }
                            }
                        }
                    }
                    DescriptionSection {
                        description_html,
                    }
                }
            }
            div { class: "torrent-below",
                OtherTorrentsSection { id: torrent.id.clone(), status_msg, on_refresh }
            }
        }
    }
}

#[component]
fn OtherTorrentsSection(
    id: String,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    let mut data: Signal<Option<Result<Vec<SearchTorrent>, ServerFnError>>> = use_signal(|| None);
    let mut refresh_trigger = use_signal(|| 0u32);

    use_effect(move || {
        let _ = *refresh_trigger.read();
        let id = id.clone();
        data.set(None);
        spawn(async move {
            data.set(Some(get_other_torrents(id).await));
        });
    });

    let inner_refresh = move |_| {
        *refresh_trigger.write() += 1;
        on_refresh.call(());
    };

    rsx! {
        div { class: "detail-card detail-related-card",
            h3 { class: "detail-section-title", "Other Torrents" }
            match data.read().clone() {
                None => rsx! {
                    p { class: "loading-indicator", "Loading other torrents..." }
                },
                Some(Err(e)) => rsx! {
                    p { class: "error", "Error loading other torrents: {e}" }
                },
                Some(Ok(torrents)) if torrents.is_empty() => rsx! {
                    p {
                        i { "No other torrents found for this book" }
                    }
                },
                Some(Ok(torrents)) => rsx! {
                    div { class: "Torrents",
                        for torrent in torrents {
                            SearchTorrentRow { torrent, status_msg, on_refresh: inner_refresh }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn TorrentActions(
    torrent_id: String,
    providers: Vec<String>,
    mam_id: Option<u64>,
    has_replacement: bool,
    library_path: Option<String>,
    abs_item_url: Option<String>,
    mam_url: Option<String>,
    goodreads_url: Option<String>,
    qbit_wanted_path: Option<String>,
    qbit_no_longer_wanted: bool,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    let loading = use_signal(|| false);
    let mut dialog_open = use_signal(|| false);

    let handle_action = move |name: String,
                              fut: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), ServerFnError>>>,
    >| {
        spawn_action(name, loading, status_msg, on_refresh, fut);
    };

    rsx! {
        div { class: "detail-action-stack",
            div { class: "detail-action-group",
                p { class: "detail-action-label", "Metadata" }
                div { class: "detail-action-row",
                    Link {
                        class: "btn",
                        to: Route::TorrentEditPage {
                            id: torrent_id.clone(),
                        },
                        "Edit Metadata"
                    }
                    button {
                        class: "btn",
                        disabled: *loading.read(),
                        onclick: move |_| dialog_open.set(true),
                        "Match Metadata"
                    }
                }
            }

            if abs_item_url.is_some() || mam_url.is_some() || goodreads_url.is_some() {
                div { class: "detail-action-group",
                    p { class: "detail-action-label", "External" }
                    div { class: "detail-action-row",
                        if let Some(abs_item_url) = abs_item_url {
                            a {
                                class: "btn",
                                href: "{abs_item_url}",
                                target: "_blank",
                                "Open in ABS"
                            }
                        }
                        if let Some(mam_url) = mam_url {
                            a {
                                class: "btn",
                                href: "{mam_url}",
                                target: "_blank",
                                "Open in MaM"
                            }
                        }
                        if let Some(goodreads_url) = goodreads_url {
                            a {
                                class: "btn",
                                href: "{goodreads_url}",
                                target: "_blank",
                                "Open in Goodreads"
                            }
                        }
                    }
                }
            }

            div { class: "detail-card detail-library-card detail-section",
                Details {
                    label: "Library".to_string(),
                    open: Some(qbit_wanted_path.is_some()),
                    div { class: "detail-library-header",
                        div {
                            if let Some(library_path) = library_path {
                                p { class: "detail-library-path", "{library_path}" }
                            } else {
                                p { class: "faint detail-library-path", "Not linked into the library." }
                            }
                            if let Some(qbit_wanted_path) = qbit_wanted_path.as_ref() {
                                p { class: "detail-library-note",
                                    strong { "Torrent should be in: " }
                                    "{qbit_wanted_path}"
                                }
                            }
                            if qbit_no_longer_wanted {
                                p { class: "warn detail-library-note",
                                    strong { "Warning: " }
                                    "No longer wanted in library"
                                }
                            }
                        }
                    }

                    div { class: "detail-action-row",
                        if qbit_wanted_path.is_some() {
                            button {
                                class: "btn",
                                disabled: *loading.read(),
                                onclick: {
                                    let torrent_id = torrent_id.clone();
                                    move |_| {
                                        let id = torrent_id.clone();
                                        handle_action(
                                            "Relink to Correct Path".to_string(),
                                            Box::pin(relink_torrent_action(id)),
                                        );
                                    }
                                },
                                "Relink to Correct Path"
                            }
                        }
                        button {
                            class: "btn",
                            disabled: *loading.read(),
                            onclick: {
                                let torrent_id = torrent_id.clone();
                                move |_| {
                                    let id = torrent_id.clone();
                                    handle_action("Relink".to_string(), Box::pin(relink_torrent_action(id)));
                                }
                            },
                            "Relink"
                        }
                        button {
                            class: "btn",
                            disabled: *loading.read(),
                            onclick: {
                                let torrent_id = torrent_id.clone();
                                move |_| {
                                    let id = torrent_id.clone();
                                    handle_action(
                                        "Refresh & Relink".to_string(),
                                        Box::pin(refresh_and_relink_action(id)),
                                    );
                                }
                            },
                            "Refresh & Relink"
                        }
                        button {
                            class: "btn",
                            disabled: *loading.read(),
                            onclick: {
                                let torrent_id = torrent_id.clone();
                                move |_| {
                                    let id = torrent_id.clone();
                                    handle_action("Clean".to_string(), Box::pin(clean_torrent_action(id)));
                                }
                            },
                            "Clean"
                        }
                        button {
                            class: "btn danger",
                            disabled: *loading.read(),
                            onclick: {
                                let torrent_id = torrent_id.clone();
                                move |_| {
                                    let id = torrent_id.clone();
                                    handle_action("Remove".to_string(), Box::pin(remove_torrent_action(id)));
                                }
                            },
                            "Remove"
                        }
                    }

                    if has_replacement {
                        div { class: "detail-action-row detail-action-row-secondary",
                            button {
                                class: "btn",
                                disabled: *loading.read(),
                                onclick: {
                                    let torrent_id = torrent_id.clone();
                                    move |_| {
                                        let id = torrent_id.clone();
                                        handle_action(
                                            "Clear Replacement".to_string(),
                                            Box::pin(clear_replacement_action(id)),
                                        );
                                    }
                                },
                                "Clear Replacement"
                            }
                        }
                    }
                }
            }
        }

        if *dialog_open.read() {
            MatchDialog {
                torrent_id: torrent_id.clone(),
                providers: providers.clone(),
                mam_id,
                status_msg,
                on_close: move |_| dialog_open.set(false),
                on_refresh,
            }
        }
    }
}

#[component]
fn MatchDialog(
    torrent_id: String,
    providers: Vec<String>,
    mam_id: Option<u64>,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_close: EventHandler<()>,
    on_refresh: EventHandler<()>,
) -> Element {
    // Add MaM as first option if mam_id is Some
    let available_providers = if mam_id.is_some() {
        let mut p = vec!["MaM".to_string()];
        p.extend(providers.clone());
        p
    } else {
        providers.clone()
    };
    let default_provider = available_providers.first().cloned().unwrap_or_default();
    let mut selected_provider = use_signal(|| default_provider);
    let loading = use_signal(|| false);

    let preview_id = torrent_id.clone();
    let preview = use_resource(move || {
        let id = preview_id.clone();
        let provider = selected_provider.read().clone();
        async move {
            if provider == "MaM" {
                preview_mam_metadata(id).await
            } else {
                preview_match_metadata(id, provider).await
            }
        }
    });

    let do_match = {
        let torrent_id = torrent_id.clone();
        move |_| {
            let id = torrent_id.clone();
            let provider = selected_provider.read().clone();

            // Try to get preview data first - if available, use apply_match_metadata_action
            // Otherwise fall back to the legacy re-fetch approach
            let preview_result = preview.read();
            let action: Pin<Box<dyn Future<Output = Result<(), ServerFnError>>>> =
                if let Some(Ok(result)) = preview_result.as_ref() {
                    // Use pre-computed metadata from preview - no re-fetch needed
                    let merged_meta = result.merged_meta.clone();
                    let diffs = result.diffs.clone();
                    Box::pin(apply_match_metadata_action(id, merged_meta, diffs))
                } else {
                    // Fall back to legacy behavior - re-fetches from provider
                    if provider == "MaM" {
                        Box::pin(refresh_metadata_action(id))
                    } else {
                        Box::pin(match_metadata_action(id, provider))
                    }
                };
            spawn_action(
                "Match Metadata".to_string(),
                loading,
                status_msg,
                EventHandler::new(move |_| {
                    on_close.call(());
                    on_refresh.call(());
                }),
                action,
            );
        }
    };

    rsx! {
        div {
            class: "dialog-overlay",
            onclick: move |_| {
                if !*loading.read() {
                    on_close.call(());
                }
            },
        }
        div { class: "dialog-box",
            h3 { "Match Metadata" }

            div { class: "dialog-field",
                label { "Provider" }
                select {
                    disabled: *loading.read(),
                    onchange: move |ev| selected_provider.set(ev.value()),
                    for p in available_providers {
                        option { value: "{p}", selected: p == *selected_provider.read(), "{p}" }
                    }
                }
            }

            div { class: "dialog-preview",
                match &*preview.read() {
                    None => rsx! {
                        p { class: "loading-indicator", "Fetching preview..." }
                    },
                    Some(Err(e)) => rsx! {
                        p { class: "error", "Preview failed: {e}" }
                    },
                    Some(Ok(result)) if result.diffs.is_empty() => rsx! {
                        p {
                            i { "No changes would be made." }
                        }
                    },
                    Some(Ok(result)) => rsx! {
                        table { class: "match-diff-table",
                            thead {
                                tr {
                                    th { "Field" }
                                    th { "Current" }
                                    th { "New" }
                                }
                            }
                            tbody {
                                for diff in result.diffs.clone() {
                                    tr {
                                        td { "{diff.field}" }
                                        td { class: "diff-from", "{diff.from}" }
                                        td { class: "diff-to", "{diff.to}" }
                                    }
                                }
                            }
                        }
                    },
                }
            }

            div { class: "dialog-actions",
                button {
                    class: "btn",
                    disabled: *loading.read() || preview.read().is_none(),
                    onclick: do_match,
                    if *loading.read() {
                        "Saving..."
                    } else {
                        "Save"
                    }
                }
                button {
                    class: "btn",
                    disabled: *loading.read(),
                    onclick: move |_| on_close.call(()),
                    "Cancel"
                }
            }
        }
    }
}

#[component]
fn QbitControls(
    torrent_id: String,
    qbit: QbitData,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    let mut selected_category = use_signal(|| qbit.torrent_category.clone());
    let mut selected_tags = use_signal(|| qbit.torrent_tags.clone());
    let loading = use_signal(|| false);
    let qbit_files = qbit
        .qbit_files
        .iter()
        .map(|file| {
            let encoded = urlencoding::encode(file).to_string();
            (file.clone(), encoded)
        })
        .collect::<Vec<_>>();

    let is_paused = qbit.is_paused;

    let handle_qbit_action = move |name: String,
                                   fut: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), ServerFnError>>>,
    >| {
        spawn_action(name, loading, status_msg, on_refresh, fut);
    };

    rsx! {
        div { class: "detail-card qbit-card",
            h3 { class: "detail-section-title", "qBittorrent" }

            dl { class: "metadata-table detail-metadata-table",
                dt { "State" }
                dd { "{qbit.torrent_state}" }
                dt { "Uploaded" }
                dd { "{qbit.uploaded}" }
            }
            if qbit.no_longer_wanted {
                div { class: "detail-inline-card",
                    p {
                        strong { "Warning: " }
                        "No longer wanted in library"
                    }
                }
            }

            div { class: "detail-action-row",
                if is_paused {
                    button {
                        class: "btn",
                        disabled: *loading.read(),
                        onclick: {
                            let torrent_id = torrent_id.clone();
                            move |_| {
                                let id = torrent_id.clone();
                                handle_qbit_action("Start".to_string(), Box::pin(torrent_start_action(id)));
                            }
                        },
                        "Start"
                    }
                } else {
                    button {
                        class: "btn",
                        disabled: *loading.read(),
                        onclick: {
                            let torrent_id = torrent_id.clone();
                            move |_| {
                                let id = torrent_id.clone();
                                handle_qbit_action("Stop".to_string(), Box::pin(torrent_stop_action(id)));
                            }
                        },
                        "Stop"
                    }
                }
                button {
                    class: "btn",
                    disabled: *loading.read(),
                    onclick: {
                        let torrent_id = torrent_id.clone();
                        move |_| {
                            let id = torrent_id.clone();
                            handle_qbit_action(
                                "Remove Seeding-only Files".to_string(),
                                Box::pin(remove_seeding_files_action(id)),
                            );
                        }
                    },
                    "Remove Seeding-only Files"
                }
            }

            div { class: "option_group",
                "Category: "
                select {
                    disabled: *loading.read(),
                    onchange: {
                        let torrent_id = torrent_id.clone();
                        move |ev| {
                            let category = ev.value();
                            selected_category.set(category.clone());
                            let tags = selected_tags.read().clone();
                            handle_qbit_action(
                                "Save Category & Tags".to_string(),
                                Box::pin(set_qbit_category_tags_action(
                                    torrent_id.clone(),
                                    category,
                                    tags,
                                )),
                            );
                        }
                    },
                    for cat in &qbit.categories {
                        option {
                            value: "{cat.name}",
                            selected: cat.name == qbit.torrent_category,
                            "{cat.name}"
                        }
                    }
                }
            }

            div { class: "option_group", style: "margin-top: 0.5em;",
                "Tags: "
                for tag in &qbit.tags {
                    label {
                        input {
                            r#type: "checkbox",
                            disabled: *loading.read(),
                            checked: selected_tags.read().contains(tag),
                            onchange: {
                                let torrent_id = torrent_id.clone();
                                let tag = tag.clone();
                                move |ev| {
                                    let mut next_tags = selected_tags.read().clone();
                                    if ev.value() == "true" {
                                        if !next_tags.contains(&tag) {
                                            next_tags.push(tag.clone());
                                        }
                                    } else {
                                        next_tags.retain(|t| t != &tag);
                                    }
                                    selected_tags.set(next_tags.clone());
                                    let category = selected_category.read().clone();
                                    handle_qbit_action(
                                        "Save Category & Tags".to_string(),
                                        Box::pin(set_qbit_category_tags_action(
                                            torrent_id.clone(),
                                            category,
                                            next_tags,
                                        )),
                                    );
                                }
                            },
                        }
                        "{tag}"
                    }
                }
            }

            if !qbit_files.is_empty() {
                div { class: "detail-section",
                    Details {
                        label: format!("qBittorrent Files ({})", qbit_files.len()),
                        open: Some(false),
                        ul {
                            for file in &qbit_files {
                                li {
                                    a {
                                        href: "/torrents/{torrent_id}/files/{file.1}",
                                        target: "_blank",
                                        "{file.0}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
