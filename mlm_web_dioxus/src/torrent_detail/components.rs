use super::server_fns::{
    clean_torrent_action, clear_replacement_action, get_metadata_providers, get_qbit_data,
    get_torrent_detail, match_metadata_action, refresh_and_relink_action, refresh_metadata_action,
    relink_torrent_action, remove_seeding_files_action, remove_torrent_action,
    set_qbit_category_tags_action, torrent_start_action, torrent_stop_action,
};
use super::types::*;
use crate::components::{
    DownloadButtonMode, DownloadButtons, SearchMetadataFilterItem, SearchMetadataFilterRow,
    SearchMetadataKind, SearchTorrentRow, StatusMessage, flag_icon, search_filter_href,
};
use crate::events::EventListItem;
use dioxus::prelude::*;

fn series_label(name: &str, entries: &str) -> String {
    if entries.is_empty() {
        name.to_string()
    } else {
        format!("{name} {entries}")
    }
}

#[component]
pub fn TorrentDetailPage(id: String) -> Element {
    let mut status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached_data = use_signal(|| None::<(TorrentPageData, Vec<String>, Option<QbitData>)>);

    let mut data_res = use_server_future(move || {
        let id = id.clone();
        async move {
            let detail = get_torrent_detail(id.clone()).await;
            let providers = get_metadata_providers().await;
            let qbit = get_qbit_data(id).await;
            (detail, providers, qbit)
        }
    })?;

    let current_value = data_res.value();
    let is_loading = data_res.pending();
    let next_cache = {
        let value = current_value.read();
        match &*value {
            Some((Ok(detail), Ok(providers), Ok(qbit))) => {
                Some((detail.clone(), providers.clone(), qbit.clone()))
            }
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
            Some((Ok(detail), Ok(providers), Ok(qbit))) => {
                Some((detail.clone(), providers.clone(), qbit.clone()))
            }
            _ => cached_data.read().clone(),
        }
    };
    let render_error = {
        let value = current_value.read();
        match (&*value, cached_data.read().is_some()) {
            (Some((Err(e), _, _)), false) => Some(e.to_string()),
            (Some((_, Err(e), _)), false) => Some(e.to_string()),
            (Some((_, _, Err(e))), false) => Some(e.to_string()),
            _ => None,
        }
    };

    rsx! {
        div { class: "torrent-detail-page",
            StatusMessage { status_msg }
            if is_loading && cached_data.read().is_some() {
                p { class: "loading-indicator", "Refreshing..." }
            }
            if let Some((detail, providers, qbit)) = rendered_data {
                match detail {
                    TorrentPageData::Downloaded(data) => {
                        rsx! {
                            TorrentDetailContent {
                                data,
                                providers,
                                qbit_data: qbit,
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
    qbit_data: Option<QbitData>,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    let TorrentDetailData {
        torrent,
        events,
        replacement_torrent,
        replacement_missing,
        abs_item_url,
        mam_torrent,
        mam_meta_diff,
        other_torrents,
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

    rsx! {
        div { class: "torrent-detail-grid",
            div { class: "torrent-side",
                div { class: "pill", "{torrent.media_type}" }

                if !torrent.categories.is_empty() {
                    div {
                        h3 { "Categories" }
                        for cat in &torrent.categories {
                            span { class: "pill", "{cat}" }
                        }
                    }
                }

                h3 { "Metadata" }
                dl { class: "metadata-table",
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
                    if let Some(path) = &torrent.library_path {
                        dt { "Library Path" }
                        dd { "{path.display()}" }
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
                }
            }

            div { class: "torrent-main",
                h1 { "{torrent.title}" }
                if let Some(replacement) = replacement_torrent {
                    div { class: "warn",
                        strong { "Replaced with: " }
                        a { href: "/dioxus/torrents/{replacement.id}", "{replacement.title}" }
                    }
                }
                if replacement_missing {
                    div { class: "warn",
                        "This torrent had a stale replacement link and it was cleared."
                    }
                }

                SearchMetadataFilterRow { kind: SearchMetadataKind::Authors, items: author_filters }
                SearchMetadataFilterRow {
                    kind: SearchMetadataKind::Narrators,
                    items: narrator_filters,
                }
                SearchMetadataFilterRow { kind: SearchMetadataKind::Series, items: series_filters }
                if !torrent.tags.is_empty() {
                    div {
                        strong { "Tags: " }
                        for tag in &torrent.tags {
                            span { class: "pill", "{tag}" }
                        }
                    }
                }
                div {
                    class: "row",
                    style: "display:flex; flex-wrap:wrap; gap:0.5em; margin:0.6em 0;",
                    a {
                        class: "btn",
                        href: "/dioxus/torrents/{torrent.id}/edit",
                        "Edit Metadata"
                    }
                    if let Some(abs_url) = abs_item_url {
                        a {
                            class: "btn",
                            href: "{abs_url}",
                            target: "_blank",
                            "Open in ABS"
                        }
                    }
                    if let Some(mam_id) = torrent.mam_id {
                        a {
                            class: "btn",
                            href: "https://www.myanonamouse.net/t/{mam_id}",
                            target: "_blank",
                            "Open in MaM"
                        }
                    }
                    if let Some(goodreads_id) = &torrent.goodreads_id {
                        a {
                            class: "btn",
                            href: "https://www.goodreads.com/book/show/{goodreads_id}",
                            target: "_blank",
                            "Open in Goodreads"
                        }
                    }
                }

                TorrentActions {
                    torrent_id: torrent.id.clone(),
                    providers,
                    has_replacement: torrent.replaced_with.is_some(),
                    status_msg,
                    on_refresh,
                }
            }

            div { class: "torrent-description",
                h3 { "Description" }
                div { dangerous_inner_html: "{torrent.description}" }

                if let Some(mam) = mam_torrent.clone() {
                    if !mam.tags.is_empty() {
                        p { "{mam.tags}" }
                    }
                    if let Some(description) = mam.description {
                        details {
                            summary { "MaM Description" }
                            div { dangerous_inner_html: "{description}" }
                        }
                    }
                }

                if !mam_meta_diff.is_empty() {
                    h3 { "MaM Metadata Differences" }
                    ul {
                        for field in mam_meta_diff {
                            li {
                                strong { "{field.field}" }
                                ": {field.to}"
                            }
                        }
                    }
                }

                details {
                    summary {
                        h3 { "Event History" }
                    }
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

            div { class: "torrent-below",
                if !library_files.is_empty() {
                    details {
                        summary { "Library Files ({library_files.len()})" }
                        ul {
                            for file in &library_files {
                                li {
                                    a {
                                        href: "/torrents/{torrent.id}/{file.1}",
                                        target: "_blank",
                                        "{file.0}"
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(qbit) = qbit_data {
                    QbitControls {
                        torrent_id: torrent.id.clone(),
                        qbit,
                        status_msg,
                        on_refresh,
                    }
                }
                OtherTorrentsSection {
                    torrents: other_torrents,
                    status_msg,
                    on_refresh,
                }
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

    rsx! {
        div { class: "torrent-detail-grid",
            div { class: "torrent-side",
                div { class: "pill", "{torrent.media_type}" }
                h3 { "Metadata" }
                dl { class: "metadata-table",
                    dt { "MaM ID" }
                    dd {
                        a {
                            href: "https://www.myanonamouse.net/t/{mam.id}",
                            target: "_blank",
                            "{mam.id}"
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
            div { class: "torrent-main",
                h1 { "{torrent.title}" }
                if let Some(ed) = &torrent.edition {
                    p { "{ed}" }
                }
                SearchMetadataFilterRow { kind: SearchMetadataKind::Authors, items: author_filters }
                SearchMetadataFilterRow {
                    kind: SearchMetadataKind::Narrators,
                    items: narrator_filters,
                }
                SearchMetadataFilterRow { kind: SearchMetadataKind::Series, items: series_filters }
                div {
                    class: "row",
                    style: "display:flex; flex-wrap:wrap; gap:0.5em; margin:0.6em 0;",
                    if let Some(goodreads_id) = &torrent.goodreads_id {
                        a {
                            class: "btn",
                            href: "https://www.goodreads.com/book/show/{goodreads_id}",
                            target: "_blank",
                            "Open in Goodreads"
                        }
                    }
                }
                div { style: "margin-top:0.8em;",
                    DownloadButtons {
                        mam_id: mam.id,
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
            div { class: "torrent-description",
                if !mam.tags.is_empty() {
                    p { "{mam.tags}" }
                }
                if let Some(description) = mam.description {
                    h3 { "Description" }
                    div { dangerous_inner_html: "{description}" }
                }
            }
            div { class: "torrent-below",
                OtherTorrentsSection {
                    torrents: data.other_torrents,
                    status_msg,
                    on_refresh,
                }
            }
        }
    }
}

#[component]
fn OtherTorrentsSection(
    torrents: Vec<crate::search::SearchTorrent>,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    rsx! {
        div { style: "margin-top:1em;",
            h3 { "Other Torrents" }
            if torrents.is_empty() {
                p {
                    i { "No other torrents found for this book" }
                }
            } else {
                div { class: "Torrents",
                    for torrent in torrents {
                        SearchTorrentRow {
                            torrent,
                            status_msg,
                            on_refresh: move |_| on_refresh.call(()),
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TorrentActions(
    torrent_id: String,
    providers: Vec<String>,
    has_replacement: bool,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    let mut selected_provider = use_signal(|| providers.first().cloned().unwrap_or_default());
    let mut loading = use_signal(|| false);

    let handle_action = move |name: String,
                              fut: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), ServerFnError>>>,
    >| {
        spawn(async move {
            loading.set(true);
            status_msg.set(None);
            match fut.await {
                Ok(_) => {
                    status_msg.set(Some((format!("{} succeeded", name), false)));
                    on_refresh.call(());
                    loading.set(false);
                }
                Err(e) => {
                    status_msg.set(Some((format!("{} failed: {}", name, e), true)));
                    loading.set(false);
                }
            }
        });
    };

    rsx! {
        div { class: "torrent-actions-widget", style: "margin-top: 1em;",
            h3 { "Actions" }

            div { style: "display: flex; gap: 0.5em; align-items: center; margin: 0.5em;",
                select {
                    disabled: *loading.read(),
                    onchange: move |ev| selected_provider.set(ev.value()),
                    for p in providers {
                        option { value: "{p}", "{p}" }
                    }
                }
                button {
                    class: "btn",
                    disabled: *loading.read(),
                    onclick: {
                        let torrent_id = torrent_id.clone();
                        move |_| {
                            let id = torrent_id.clone();
                            let provider = selected_provider.read().clone();
                            handle_action(
                                "Match Metadata".to_string(),
                                Box::pin(match_metadata_action(id, provider)),
                            );
                        }
                    },
                    if *loading.read() {
                        "Matching..."
                    } else {
                        "Match Metadata"
                    }
                }
            }

            div { style: "display: flex; flex-wrap: wrap; gap: 0.5em;",
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
                    class: "btn",
                    disabled: *loading.read(),
                    onclick: {
                        let torrent_id = torrent_id.clone();
                        move |_| {
                            let id = torrent_id.clone();
                            handle_action("Refresh".to_string(), Box::pin(refresh_metadata_action(id)));
                        }
                    },
                    "Refresh"
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
                if has_replacement {
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
                button {
                    class: "btn",
                    style: "background: #fdd;",
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
    let mut loading = use_signal(|| false);
    let qbit_files = qbit
        .qbit_files
        .iter()
        .map(|file| {
            let encoded = urlencoding::encode(file).to_string();
            (file.clone(), encoded)
        })
        .collect::<Vec<_>>();

    let is_paused = qbit.torrent_state.to_lowercase().contains("paused")
        || qbit.torrent_state.to_lowercase().contains("stopped");

    let handle_qbit_action = move |name: String,
                                   fut: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), ServerFnError>>>,
    >| {
        spawn(async move {
            loading.set(true);
            status_msg.set(None);
            match fut.await {
                Ok(_) => {
                    status_msg.set(Some((format!("{} succeeded", name), false)));
                    on_refresh.call(());
                    loading.set(false);
                }
                Err(e) => {
                    status_msg.set(Some((format!("{} failed: {}", name, e), true)));
                    loading.set(false);
                }
            }
        });
    };

    rsx! {
        div { style: "margin-top: 1em; padding: 1em; background: var(--above); border-radius: 4px;",
            h3 { "qBittorrent" }

            dl { class: "metadata-table",
                dt { "State" }
                dd { "{qbit.torrent_state}" }
                dt { "Uploaded" }
                dd { "{qbit.uploaded}" }
            }

            if let Some(path) = qbit.wanted_path {
                div { style: "margin: 1em 0; padding: 0.5em; background: var(--bg); border-radius: 4px;",
                    p {
                        strong { "⚠️ Torrent should be in: " }
                        "{path.display()}"
                    }
                    button {
                        class: "btn",
                        disabled: *loading.read(),
                        onclick: {
                            let torrent_id = torrent_id.clone();
                            move |_| {
                                let id = torrent_id.clone();
                                handle_qbit_action(
                                    "Relink to Correct Path".to_string(),
                                    Box::pin(relink_torrent_action(id)),
                                );
                            }
                        },
                        "Relink to Correct Path"
                    }
                }
            }
            if qbit.no_longer_wanted {
                div { style: "margin: 1em 0; padding: 0.5em; background: var(--bg); border-radius: 4px;",
                    p {
                        strong { "⚠️ " }
                        "No longer wanted in library"
                    }
                }
            }

            div { style: "display: flex; gap: 0.5em; margin: 1em 0;",
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
                    onchange: move |ev| selected_category.set(ev.value()),
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
                                let tag = tag.clone();
                                move |ev| {
                                    if ev.value() == "true" {
                                        if !selected_tags.read().contains(&tag) {
                                            selected_tags.write().push(tag.clone());
                                        }
                                    } else {
                                        selected_tags.write().retain(|t| t != &tag);
                                    }
                                }
                            },
                        }
                        "{tag}"
                    }
                }
            }

            button {
                class: "btn",
                style: "margin-top: 1em;",
                disabled: *loading.read(),
                onclick: {
                    let torrent_id = torrent_id.clone();
                    move |_| {
                        let id = torrent_id.clone();
                        let cat = selected_category.read().clone();
                        let tags = selected_tags.read().clone();
                        handle_qbit_action(
                            "Save Category & Tags".to_string(),
                            Box::pin(set_qbit_category_tags_action(id, cat, tags)),
                        );
                    }
                },
                "Save Category & Tags"
            }

            if !qbit_files.is_empty() {
                details {
                    summary { "qBittorrent Files ({qbit_files.len()})" }
                    ul {
                        for file in &qbit_files {
                            li {
                                a {
                                    href: "/torrents/{torrent_id}/{file.1}",
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
