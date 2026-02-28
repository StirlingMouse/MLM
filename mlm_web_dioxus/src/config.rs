use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::parse_location_query_pairs;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ConfigPageData {
    pub header_html: String,
    pub qbittorrent: Vec<QbitBlock>,
    pub autograbs: Vec<AutoGrabBlock>,
    pub goodreads_lists: Vec<GoodreadsListBlock>,
    pub tags: Vec<TagBlock>,
    pub libraries: Vec<LibraryBlock>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct QbitBlock {
    pub html: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AutoGrabBlock {
    pub mam_search: String,
    pub html: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GoodreadsListBlock {
    pub html: String,
    pub grabs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TagBlock {
    pub html: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LibraryBlock {
    pub html: String,
}

#[server]
pub async fn get_config_page_data() -> Result<ConfigPageData, ServerFnError> {
    let context = crate::error::get_context()?;
    let config = context.config().await;

    let mut header_html = String::new();
    push_num(&mut header_html, "unsat_buffer", config.unsat_buffer);
    push_num(&mut header_html, "wedge_buffer", config.wedge_buffer);
    if config.add_torrents_stopped {
        push_num(
            &mut header_html,
            "add_torrents_stopped",
            config.add_torrents_stopped,
        );
    }
    if config.exclude_narrator_in_library_dir {
        push_num(
            &mut header_html,
            "exclude_narrator_in_library_dir",
            config.exclude_narrator_in_library_dir,
        );
    }
    push_num(&mut header_html, "search_interval", config.search_interval);
    push_num(&mut header_html, "link_interval", config.link_interval);
    push_num(&mut header_html, "import_interval", config.import_interval);
    if !config.ignore_torrents.is_empty() {
        push_yaml_nums(&mut header_html, "ignore_torrents", &config.ignore_torrents);
    }
    push_yaml_items(&mut header_html, "audio_types", &config.audio_types);
    push_yaml_items(&mut header_html, "ebook_types", &config.ebook_types);
    push_yaml_items(&mut header_html, "music_types", &config.music_types);
    push_yaml_items(&mut header_html, "radio_types", &config.radio_types);

    let qbittorrent = config
        .qbittorrent
        .iter()
        .map(|qbit| {
            let mut html = String::new();
            push_str_json(&mut html, "url", &qbit.url);
            if !qbit.username.is_empty() {
                push_str_json(&mut html, "username", &qbit.username);
            }
            if !qbit.password.is_empty() {
                html.push_str(
                    "<span class=key>password</span> = <span class=string>\"\"</span> # hidden<br>",
                );
            }
            if let Some(on_cleaned) = &qbit.on_cleaned {
                html.push_str("<div class=row><h4>[qbittorrent.on_cleaned]</h4></div>");
                if let Some(category) = &on_cleaned.category {
                    push_str_json(&mut html, "category", category);
                }
                if !on_cleaned.tags.is_empty() {
                    push_yaml_items(&mut html, "tags", &on_cleaned.tags);
                }
            }
            QbitBlock { html }
        })
        .collect::<Vec<_>>();

    let autograbs = config
        .autograbs
        .iter()
        .map(|autograb| {
            let mut html = String::new();
            if let Some(name) = autograb.filter.name.as_ref() {
                push_str_json(&mut html, "name", name);
            }
            push_str_json(&mut html, "type", &autograb.kind);
            push_str_json(&mut html, "cost", &autograb.cost);
            if let Some(query) = autograb.query.as_ref() {
                push_str_json(&mut html, "query", query);
            }
            if !autograb.search_in.is_empty() {
                push_yaml_items(&mut html, "search_in", &autograb.search_in);
            }
            if let Some(sort_by) = autograb.sort_by.as_ref() {
                push_str_json(&mut html, "sort_by", sort_by);
            }
            html.push_str(&render_filter_html(&autograb.filter));
            if let Some(search_interval) = autograb.search_interval {
                push_num(&mut html, "search_interval", search_interval);
            }
            if let Some(unsat_buffer) = autograb.unsat_buffer {
                push_num(&mut html, "unsat_buffer", unsat_buffer);
            }
            if let Some(max_active_downloads) = autograb.max_active_downloads {
                push_num(&mut html, "max_active_downloads", max_active_downloads);
            }
            if let Some(wedge_buffer) = autograb.wedge_buffer {
                push_num(&mut html, "wedge_buffer", wedge_buffer);
            }
            if autograb.dry_run {
                push_num(&mut html, "dry_run", autograb.dry_run);
            }
            if let Some(category) = autograb.category.as_ref() {
                push_str_json(&mut html, "category", category);
            }
            AutoGrabBlock {
                mam_search: autograb.mam_search(),
                html,
            }
        })
        .collect::<Vec<_>>();

    let goodreads_lists = config
        .goodreads_lists
        .iter()
        .map(|list| {
            let mut html = String::new();
            if let Some(name) = list.name.as_ref() {
                push_str_json(&mut html, "name", name);
            }
            if let Some(search_interval) = list.search_interval {
                push_num(&mut html, "search_interval", search_interval);
            }
            if let Some(unsat_buffer) = list.unsat_buffer {
                push_num(&mut html, "unsat_buffer", unsat_buffer);
            }
            if let Some(wedge_buffer) = list.wedge_buffer {
                push_num(&mut html, "wedge_buffer", wedge_buffer);
            }
            if list.dry_run {
                push_num(&mut html, "dry_run", list.dry_run);
            }

            let grabs = list
                .grab
                .iter()
                .map(|grab| {
                    let mut grab_html = String::new();
                    push_str_json(&mut grab_html, "cost", &grab.cost);
                    grab_html.push_str(&render_filter_html(&grab.filter));
                    grab_html
                })
                .collect::<Vec<_>>();

            GoodreadsListBlock { html, grabs }
        })
        .collect::<Vec<_>>();

    let tags = config
        .tags
        .iter()
        .map(|tag| {
            let mut html = String::new();
            if let Some(name) = tag.filter.name.as_ref() {
                push_str_json(&mut html, "name", name);
            }
            html.push_str(&render_filter_html(&tag.filter));
            if let Some(category) = tag.category.as_ref() {
                push_str_json(&mut html, "category", category);
            }
            if !tag.tags.is_empty() {
                push_yaml_items(&mut html, "tags", &tag.tags);
            }
            TagBlock { html }
        })
        .collect::<Vec<_>>();

    let libraries = config
        .libraries
        .iter()
        .map(|library| {
            let mut html = String::new();
            if let Some(name) = library.options().name.as_ref() {
                push_str_json(&mut html, "name", name);
            }
            match library {
                mlm_core::config::Library::ByRipDir(l) => {
                    push_str_json(&mut html, "rip_dir", &l.rip_dir)
                }
                mlm_core::config::Library::ByDownloadDir(l) => {
                    push_str_json(&mut html, "download_dir", &l.download_dir)
                }
                mlm_core::config::Library::ByCategory(l) => {
                    push_str_json(&mut html, "category", &l.category)
                }
            }
            push_str_json(&mut html, "library_dir", &library.options().library_dir);
            if !library.tag_filters().allow_tags.is_empty() {
                push_yaml_items(&mut html, "allow_tags", &library.tag_filters().allow_tags);
            }
            if !library.tag_filters().deny_tags.is_empty() {
                push_yaml_items(&mut html, "deny_tags", &library.tag_filters().deny_tags);
            }
            if library.options().method != Default::default() {
                push_str_json(&mut html, "method", &library.options().method);
            }
            if let Some(audio_types) = library.options().audio_types.as_ref()
                && !audio_types.is_empty()
            {
                push_yaml_items(&mut html, "audio_types", audio_types);
            }
            if let Some(ebook_types) = library.options().ebook_types.as_ref()
                && !ebook_types.is_empty()
            {
                push_yaml_items(&mut html, "ebook_types", ebook_types);
            }
            LibraryBlock { html }
        })
        .collect::<Vec<_>>();

    Ok(ConfigPageData {
        header_html,
        qbittorrent,
        autograbs,
        goodreads_lists,
        tags,
        libraries,
    })
}

#[server]
pub async fn apply_tag_filter_action(
    qbit_index: usize,
    tag_filter: usize,
) -> Result<(), ServerFnError> {
    use std::ops::Deref;

    use mlm_core::ContextExt as _;
    use mlm_core::autograbber::update_torrent_meta;
    use mlm_core::qbittorrent::ensure_category_exists;
    use mlm_db::{DatabaseExt as _, Torrent};

    let context = crate::error::get_context()?;
    let config = context.config().await;

    let tag_filter = config
        .tags
        .get(tag_filter)
        .ok_or_else(|| ServerFnError::new("invalid tag_filter"))?;
    let qbit_conf = config
        .qbittorrent
        .get(qbit_index)
        .ok_or_else(|| ServerFnError::new("requires a qbit config"))?;
    let qbit = qbit::Api::new_login_username_password(
        &qbit_conf.url,
        &qbit_conf.username,
        &qbit_conf.password,
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    let torrents = context
        .db()
        .r_transaction()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .scan()
        .primary::<Torrent>()
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    for torrent in torrents
        .all()
        .map_err(|e| ServerFnError::new(e.to_string()))?
    {
        let torrent = torrent.map_err(|e| ServerFnError::new(e.to_string()))?;
        match tag_filter.filter.matches_lib(&torrent) {
            Ok(matches) => {
                if !matches {
                    continue;
                }
            }
            Err(_) => {
                let Some(mam_id) = torrent.mam_id else {
                    continue;
                };
                let mam = context
                    .mam()
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
                let Some(mam_torrent) = mam
                    .get_torrent_info_by_id(mam_id)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                else {
                    continue;
                };
                let new_meta = mam_torrent
                    .as_meta()
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
                if new_meta != torrent.meta {
                    update_torrent_meta(
                        &config,
                        context.db(),
                        context
                            .db()
                            .rw_async()
                            .await
                            .map_err(|e| ServerFnError::new(e.to_string()))?,
                        Some(&mam_torrent),
                        torrent.clone(),
                        new_meta,
                        false,
                        false,
                        &context.events,
                    )
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
                }
                if !tag_filter.filter.matches(&mam_torrent) {
                    continue;
                }
            }
        };

        if let Some(category) = &tag_filter.category {
            ensure_category_exists(&qbit, &qbit_conf.url, category)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            qbit.set_category(Some(vec![torrent.id.as_str()]), category)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        if !tag_filter.tags.is_empty() {
            qbit.add_tags(
                Some(vec![torrent.id.as_str()]),
                tag_filter.tags.iter().map(Deref::deref).collect(),
            )
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }
    }

    Ok(())
}

#[component]
pub fn ConfigPage() -> Element {
    let show_apply_tags = parse_location_query_pairs()
        .into_iter()
        .any(|(k, v)| k == "show_apply_tags" && matches!(v.as_str(), "1" | "true" | "yes"));

    let config_data = use_server_future(move || async move { get_config_page_data().await })?;
    let data = config_data.suspend()?;

    match &*data.read() {
        Ok(data) => rsx! { ConfigPageContent { data: data.clone(), show_apply_tags } },
        Err(e) => rsx! {
            div { class: "config-page",
                h1 { "Config" }
                p { class: "error", "Error loading config page: {e}" }
            }
        },
    }
}

#[component]
fn ConfigPageContent(data: ConfigPageData, show_apply_tags: bool) -> Element {
    let mut qbit_index = use_signal(|| "0".to_string());
    let status_msg = use_signal(|| None::<(String, bool)>);
    let applying = use_signal(|| None::<usize>);

    rsx! {
        document::Title { "MLM - Config" }

        h1 { "Config" }

        if let Some((msg, is_error)) = status_msg.read().as_ref() {
            p { class: if *is_error { "error" } else { "loading-indicator" },
                "{msg}"
            }
        }

        div { class: "infoboxes",
            div {
                class: "configbox",
                dangerous_inner_html: "{data.header_html}"
            }
        }

        for qbit in data.qbittorrent.iter() {
            div { class: "infoboxes",
                div { class: "configbox",
                    div { class: "row",
                        h3 { "[[qbittorrent]]" }
                    }
                    div { dangerous_inner_html: "{qbit.html}" }
                }
            }
        }

        for autograb in data.autograbs.iter() {
            div { class: "infoboxes",
                div { class: "configbox",
                    div { class: "row",
                        h3 { "[[autograb]]" }
                        a { href: "{autograb.mam_search}", target: "_blank", "search on MaM" }
                    }
                    div { dangerous_inner_html: "{autograb.html}" }
                }
            }
        }

        for list in data.goodreads_lists.iter() {
            div { class: "infoboxes",
                div { class: "configbox",
                    div { class: "row",
                        h3 { "[[goodreads_list]]" }
                    }
                    div { dangerous_inner_html: "{list.html}" }

                    for grab in list.grabs.iter() {
                        div { class: "infoboxes",
                            div { class: "configbox",
                                div { class: "row",
                                    h4 { "[[goodreads_list.grab]]" }
                                }
                                div { dangerous_inner_html: "{grab}" }
                            }
                        }
                    }
                }
            }
        }

        for (i, tag) in data.tags.iter().enumerate() {
            div { class: "infoboxes",
                div { class: "configbox",
                    div { class: "row",
                        h3 { "[[tag]]" }
                        if show_apply_tags {
                            div {
                                label {
                                    "Client: "
                                    input {
                                        r#type: "number",
                                        value: "{qbit_index.read()}",
                                        oninput: move |ev| qbit_index.set(ev.value()),
                                    }
                                }
                                button {
                                    r#type: "button",
                                    disabled: applying.read().is_some(),
                                    onclick: {
                                        let qbit_index_signal = qbit_index;
                                        let mut status_msg = status_msg;
                                        let mut applying = applying;
                                        move |_| {
                                            let qbit_index = qbit_index_signal
                                                .read()
                                                .parse::<usize>()
                                                .unwrap_or_default();
                                            applying.set(Some(i));
                                            status_msg.set(None);
                                            spawn(async move {
                                                match apply_tag_filter_action(qbit_index, i).await {
                                                    Ok(_) => {
                                                        status_msg.set(Some((
                                                            "Applied tags to matching torrents".to_string(),
                                                            false,
                                                        )));
                                                    }
                                                    Err(e) => {
                                                        status_msg.set(Some((format!("Apply failed: {e}"), true)));
                                                    }
                                                }
                                                applying.set(None);
                                            });
                                        }
                                    },
                                    "apply to all"
                                }
                            }
                        }
                    }
                    div { dangerous_inner_html: "{tag.html}" }
                }
            }
        }

        for library in data.libraries.iter() {
            div { class: "infoboxes",
                div { class: "configbox",
                    div { class: "row",
                        h3 { "[[library]]" }
                    }
                    div { dangerous_inner_html: "{library.html}" }
                }
            }
        }
    }
}

#[cfg(feature = "server")]
fn push_num<T: std::fmt::Display>(html: &mut String, key: &str, value: T) {
    html.push_str(&format!(
        "<span class=key>{key}</span> = <span class=num>{value}</span><br>"
    ));
}

#[cfg(feature = "server")]
fn push_str_json<T: Serialize>(html: &mut String, key: &str, value: &T) {
    html.push_str(&format!(
        "<span class=key>{}</span> = <span class=string>{}</span><br>",
        key,
        to_json(value)
    ));
}

#[cfg(feature = "server")]
fn push_yaml_items<T: Serialize>(html: &mut String, key: &str, values: &[T]) {
    html.push_str(&format!(
        "<span class=key>{}</span> = {}<br>",
        key,
        yaml_items(values, "string")
    ));
}

#[cfg(feature = "server")]
fn push_yaml_nums<T: Serialize>(html: &mut String, key: &str, values: &[T]) {
    html.push_str(&format!(
        "<span class=key>{}</span> = {}<br>",
        key,
        yaml_items(values, "num")
    ));
}

#[cfg(feature = "server")]
fn yaml_items<T: Serialize>(values: &[T], class: &str) -> String {
    if values.len() > 5 {
        let mut s = String::from("[<br>");
        for v in values {
            s.push_str(&format!(
                "&nbsp;&nbsp;<span class={}>{}</span>,<br>",
                class,
                to_json(v)
            ));
        }
        s.push(']');
        s
    } else {
        let items = values
            .iter()
            .map(|v| format!("<span class={}>{}</span>", class, to_json(v)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[ {} ]", items)
    }
}

#[cfg(feature = "server")]
fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

#[cfg(feature = "server")]
fn render_filter_html(filter: &mlm_core::config::TorrentFilter) -> String {
    use mlm_db::{AudiobookCategory, EbookCategory};
    use mlm_mam::serde::DATE_FORMAT;

    let mut html = String::new();

    if filter.edition.categories.audio.is_some() || filter.edition.categories.ebook.is_some() {
        html.push_str("<span class=key>categories</span> = {<br>");

        match filter.edition.categories.audio.as_ref() {
            Some(cats) if cats.is_empty() => {
                html.push_str("&nbsp;&nbsp;audio = <span class=num>false</span><br>")
            }
            Some(cats) if cats == &AudiobookCategory::all() => {
                html.push_str("&nbsp;&nbsp;audio = <span class=num>true</span><br>")
            }
            Some(cats) => {
                html.push_str(&format!(
                    "&nbsp;&nbsp;audio = {}<br>",
                    yaml_items(cats, "string")
                ));
            }
            None => html.push_str("&nbsp;&nbsp;audio = <span class=num>true</span><br>"),
        }

        match filter.edition.categories.ebook.as_ref() {
            Some(cats) if cats.is_empty() => {
                html.push_str("&nbsp;&nbsp;ebook = <span class=num>false</span><br>")
            }
            Some(cats) if cats == &EbookCategory::all() => {
                html.push_str("&nbsp;&nbsp;ebook = <span class=num>true</span><br>")
            }
            Some(cats) => {
                html.push_str(&format!(
                    "&nbsp;&nbsp;ebook = {}<br>",
                    yaml_items(cats, "string")
                ));
            }
            None => html.push_str("&nbsp;&nbsp;ebook = <span class=num>true</span><br>"),
        }

        html.push_str("}<br>");
    }

    if !filter.edition.languages.is_empty() {
        push_yaml_items(&mut html, "languages", &filter.edition.languages);
    }

    if filter.edition.flags.as_bitfield() > 0 {
        html.push_str("<span class=key>flags</span> = {");
        let flag_count = filter.edition.flags.as_search_bitfield().1.len();
        if flag_count > 3 {
            html.push_str("<br>");
            push_flag_line_multi(
                &mut html,
                "crude_language",
                filter.edition.flags.crude_language,
            );
            push_flag_line_multi(&mut html, "violence", filter.edition.flags.violence);
            push_flag_line_multi(
                &mut html,
                "some_explicit",
                filter.edition.flags.some_explicit,
            );
            push_flag_line_multi(&mut html, "explicit", filter.edition.flags.explicit);
            push_flag_line_multi(&mut html, "abridged", filter.edition.flags.abridged);
            push_flag_line_multi(&mut html, "lgbt", filter.edition.flags.lgbt);
        } else {
            push_flag_line_inline(
                &mut html,
                "crude_language",
                filter.edition.flags.crude_language,
            );
            push_flag_line_inline(&mut html, "violence", filter.edition.flags.violence);
            push_flag_line_inline(
                &mut html,
                "some_explicit",
                filter.edition.flags.some_explicit,
            );
            push_flag_line_inline(&mut html, "explicit", filter.edition.flags.explicit);
            push_flag_line_inline(&mut html, "abridged", filter.edition.flags.abridged);
            push_flag_line_inline(&mut html, "lgbt", filter.edition.flags.lgbt);
        }
        html.push_str("}<br>");
    }

    if filter.edition.min_size.bytes() > 0 {
        html.push_str(&format!(
            "<span class=key>min_size</span> = <span class=string>\"{}\"</span><br>",
            filter.edition.min_size
        ));
    }

    if filter.edition.max_size.bytes() > 0 {
        html.push_str(&format!(
            "<span class=key>max_size</span> = <span class=string>\"{}\"</span><br>",
            filter.edition.max_size
        ));
    }

    if !filter.exclude_uploader.is_empty() {
        push_yaml_items(&mut html, "exclude_uploader", &filter.exclude_uploader);
    }

    if let Some(uploaded_after) = filter.uploaded_after {
        let date = uploaded_after.format(&DATE_FORMAT).unwrap_or_default();
        html.push_str(&format!(
            "<span class=key>uploaded_after</span> = <span class=string>\"{}\"</span><br>",
            date
        ));
    }
    if let Some(uploaded_before) = filter.uploaded_before {
        let date = uploaded_before.format(&DATE_FORMAT).unwrap_or_default();
        html.push_str(&format!(
            "<span class=key>uploaded_before</span> = <span class=string>\"{}\"</span><br>",
            date
        ));
    }

    if let Some(v) = filter.min_seeders {
        push_num(&mut html, "min_seeders", v);
    }
    if let Some(v) = filter.max_seeders {
        push_num(&mut html, "max_seeders", v);
    }
    if let Some(v) = filter.min_leechers {
        push_num(&mut html, "min_leechers", v);
    }
    if let Some(v) = filter.max_leechers {
        push_num(&mut html, "max_leechers", v);
    }
    if let Some(v) = filter.min_snatched {
        push_num(&mut html, "min_snatched", v);
    }
    if let Some(v) = filter.max_snatched {
        push_num(&mut html, "max_snatched", v);
    }

    html
}

#[cfg(feature = "server")]
fn push_flag_line_multi(html: &mut String, key: &str, value: Option<bool>) {
    if let Some(v) = value {
        html.push_str(&format!(
            "&nbsp;&nbsp;{} = <span class=num>{}</span><br>",
            key, v
        ));
    }
}

#[cfg(feature = "server")]
fn push_flag_line_inline(html: &mut String, key: &str, value: Option<bool>) {
    if let Some(v) = value {
        html.push_str(&format!("{} = <span class=num>{}</span> ", key, v));
    }
}
