#[cfg(feature = "server")]
use crate::error::{IntoServerFnError, OptionIntoServerFnError};
use crate::sse::STATS_UPDATE_TRIGGER;
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ListItemTorrentDto {
    pub id: String,
    pub mam_id: Option<u64>,
    pub status: String,
    pub at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ListItemDto {
    pub guid: (String, String),
    pub title: String,
    pub authors: Vec<String>,
    pub series: Vec<(String, f64)>,
    pub cover_url: String,
    pub book_url: Option<String>,
    pub want_audio: bool,
    pub want_ebook: bool,
    pub audio_torrent: Option<ListItemTorrentDto>,
    pub ebook_torrent: Option<ListItemTorrentDto>,
    pub marked_done_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ListDto {
    pub id: String,
    pub title: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ListPageData {
    pub list: ListDto,
    pub items: Vec<ListItemDto>,
    pub show: Option<String>,
}

#[cfg(feature = "server")]
fn torrent_status_to_string(status: mlm_db::TorrentStatus) -> String {
    use mlm_db::TorrentStatus;
    match status {
        TorrentStatus::Selected => "Selected".to_string(),
        TorrentStatus::Wanted => "Wanted".to_string(),
        TorrentStatus::NotWanted => "NotWanted".to_string(),
        TorrentStatus::Existing => "Existing".to_string(),
    }
}

#[cfg(feature = "server")]
fn item_wants_audio(item: &mlm_db::ListItem) -> bool {
    use mlm_db::TorrentStatus;
    item.allow_audio
        && item
            .audio_torrent
            .as_ref()
            .map(|t| matches!(t.status, TorrentStatus::Wanted))
            .unwrap_or(true)
}

#[cfg(feature = "server")]
fn item_wants_ebook(item: &mlm_db::ListItem) -> bool {
    use mlm_db::TorrentStatus;
    item.allow_ebook
        && item
            .ebook_torrent
            .as_ref()
            .map(|t| matches!(t.status, TorrentStatus::Wanted))
            .unwrap_or(true)
}

#[server]
pub async fn get_list_data(
    list_id: String,
    show: Option<String>,
) -> Result<ListPageData, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use dioxus_fullstack::FullstackContext;
        use mlm_core::{Context, ContextExt};
        use mlm_db::{DatabaseExt as _, List, ListItem, ListItemKey};

        let ctx = FullstackContext::current().ok_or_server_err("FullstackContext not found")?;
        let context: Context = ctx
            .extension()
            .ok_or_server_err("Context not found in extensions")?;

        let db = context.db();

        let list = db
            .r_transaction()
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .get()
            .primary::<List>(list_id.as_str())
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("List not found"))?;

        let items: Vec<ListItem> = db
            .r_transaction()
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .scan()
            .secondary::<ListItem>(ListItemKey::created_at)
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .all()
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .filter_map(|t| t.ok())
            .filter(|item| item.list_id == list.id)
            .filter(|item| {
                if let Some(ref show) = show {
                    match show.as_str() {
                        "any" => item_wants_audio(item) || item_wants_ebook(item),
                        "audio" => item_wants_audio(item),
                        "ebook" => item_wants_ebook(item),
                        _ => true,
                    }
                } else {
                    true
                }
            })
            .collect();

        let items_dto: Vec<ListItemDto> = items
            .into_iter()
            .map(|item| {
                let want_audio = item_wants_audio(&item);
                let want_ebook = item_wants_ebook(&item);
                ListItemDto {
                    guid: item.guid,
                    title: item.title,
                    authors: item.authors,
                    series: item.series,
                    cover_url: item.cover_url,
                    book_url: item.book_url,
                    want_audio,
                    want_ebook,
                    audio_torrent: item.audio_torrent.map(|t| ListItemTorrentDto {
                        id: t.torrent_id.unwrap_or_default(),
                        mam_id: t.mam_id,
                        status: torrent_status_to_string(t.status),
                        at: format_timestamp_db(&t.at),
                    }),
                    ebook_torrent: item.ebook_torrent.map(|t| ListItemTorrentDto {
                        id: t.torrent_id.unwrap_or_default(),
                        mam_id: t.mam_id,
                        status: torrent_status_to_string(t.status),
                        at: format_timestamp_db(&t.at),
                    }),
                    marked_done_at: item.marked_done_at.map(|ts| format_timestamp_db(&ts)),
                }
            })
            .collect();

        Ok(ListPageData {
            list: ListDto {
                id: list.id,
                title: list.title,
            },
            items: items_dto,
            show,
        })
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server feature not enabled"))
    }
}

#[server]
pub async fn mark_list_item_done(list_id: String, item_id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use dioxus_fullstack::FullstackContext;
        use mlm_core::{Context, ContextExt};
        use mlm_db::{DatabaseExt as _, ListItem, Timestamp};

        let ctx = FullstackContext::current().ok_or_server_err("FullstackContext not found")?;
        let context: Context = ctx
            .extension()
            .ok_or_server_err("Context not found in extensions")?;

        let db = context.db();

        let (_guard, rw) = db
            .rw_async()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let mut item = rw
            .get()
            .primary::<ListItem>((list_id.as_str(), item_id.as_str()))
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("Could not find item"))?;
        item.marked_done_at = Some(Timestamp::now());
        rw.upsert(item)
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        rw.commit().map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(())
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server feature not enabled"))
    }
}

#[component]
pub fn ListPage(id: String) -> Element {
    let list_id = id.clone();
    let mut cached_data = use_signal(|| None::<ListPageData>);

    let mut list_data = match use_server_future(move || {
        let list_id = list_id.clone();
        async move { get_list_data(list_id, None).await }
    }) {
        Ok(resource) => resource,
        Err(_) => {
            return rsx! {
                p { "Loading..." }
            };
        }
    };

    use_effect(move || {
        let _ = *STATS_UPDATE_TRIGGER.read();
        list_data.restart();
    });

    let current_value = list_data.value();

    {
        let val = current_value.read();
        if let Some(Ok(data)) = &*val {
            cached_data.set(Some(data.clone()));
        }
    }

    let data_to_show = {
        let val = current_value.read();
        match &*val {
            Some(Ok(data)) => Some(data.clone()),
            Some(Err(_)) | None => cached_data.read().clone(),
        }
    };

    rsx! {
        if let Some(data) = data_to_show {
            ListPageContent { list_id: id, data }
        } else {
            p { "Loading..." }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ListPageContentProps {
    list_id: String,
    data: ListPageData,
}

#[component]
fn ListPageContent(props: ListPageContentProps) -> Element {
    let list_id = props.list_id.clone();
    let mut show = use_signal(|| None::<String>);

    let items: Vec<ListItemDto> = props
        .data
        .items
        .iter()
        .filter(|item| match show.read().as_deref() {
            Some("any") => item.want_audio || item.want_ebook,
            Some("audio") => item.want_audio,
            Some("ebook") => item.want_ebook,
            _ => true,
        })
        .cloned()
        .collect();

    rsx! {
        div { class: "list-page",
            div { class: "row",
                h1 { "{props.data.list.title}" }
                div { class: "option_group query",
                    "Show: "
                    label {
                        "All"
                        input {
                            r#type: "radio",
                            name: "show",
                            checked: show.read().is_none(),
                            onclick: move |_| {
                                show.set(None);
                            },
                        }
                    }
                    label {
                        "Any Missing"
                        input {
                            r#type: "radio",
                            name: "show",
                            checked: show.read().as_deref() == Some("any"),
                            value: "any",
                            onclick: move |_| {
                                show.set(Some("any".to_string()));
                            },
                        }
                    }
                    label {
                        "Audio Missing"
                        input {
                            r#type: "radio",
                            name: "show",
                            checked: show.read().as_deref() == Some("audio"),
                            value: "audio",
                            onclick: move |_| {
                                show.set(Some("audio".to_string()));
                            },
                        }
                    }
                    label {
                        "Ebook Missing"
                        input {
                            r#type: "radio",
                            name: "show",
                            checked: show.read().as_deref() == Some("ebook"),
                            value: "ebook",
                            onclick: move |_| {
                                show.set(Some("ebook".to_string()));
                            },
                        }
                    }
                }
            }

            for item in &items {
                ListItemComponent { list_id: list_id.clone(), item: item.clone() }
            }

            if items.is_empty() {
                p {
                    i { "The list is empty" }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ListItemComponentProps {
    list_id: String,
    item: ListItemDto,
}

#[component]
fn ListItemComponent(props: ListItemComponentProps) -> Element {
    let list_id = props.list_id.clone();
    let item = props.item.clone();
    let guid = props.item.guid.clone();
    let item_id = guid.1.clone();
    let can_mark_done = props.item.want_audio || props.item.want_ebook;

    let mut marking_done = use_signal(|| false);

    let authors_str = props.item.authors.join(", ");

    rsx! {
        div { class: "list_item",
            img { src: "{item.cover_url}" }
            div {
                div { class: "row",
                    h3 { "{item.title}" }
                    div {
                        a { href: "{mam_search_url(&item)}", target: "_blank", "search on MaM" }
                        if let Some(url) = &item.book_url {
                            a { href: "{url}", target: "_blank", "goodreads" }
                        }
                    }
                }
                p { class: "author", "by {authors_str}" }

                if !item.series.is_empty() {
                    p {
                        for (i , (name , num)) in item.series.iter().enumerate() {
                            "{name} #{num}"
                            if i < item.series.len() - 1 {
                                ", "
                            }
                        }
                    }
                }

                if let Some(torrent) = &item.audio_torrent {
                    match torrent.status.as_str() {
                        "Selected" => rsx! {
                            span { "Downloaded audiobook " }
                            a { href: "/dioxus/torrents/{torrent.id}", target: "_blank", "torrent" }
                            span { " at {torrent.at}" }
                            br {}
                        },
                        "Wanted" => rsx! {
                            span { "Suggest wedge audiobook " }
                            a { href: "/dioxus/torrents/{torrent.id}", target: "_blank", "torrent" }
                            span { " at {torrent.at}" }
                            br {}
                        },
                        "NotWanted" => rsx! {
                            span { "Skipped audiobook " }
                            a { href: "/dioxus/torrents/{torrent.id}", target: "_blank", "torrent" }
                            span { " as an ebook was found at {torrent.at}" }
                            br {}
                        },
                        "Existing" => rsx! {
                            span { "Found matching audiobook " }
                            a { href: "/dioxus/torrents/{torrent.id}", target: "_blank", "torrent" }
                            span { " in library at {torrent.at}" }
                            br {}
                        },
                        _ => rsx! {},
                    }
                } else if item.want_audio {
                    span { class: "missing", "Audiobook missing" }
                    br {}
                }

                if let Some(torrent) = &item.ebook_torrent {
                    match torrent.status.as_str() {
                        "Selected" => rsx! {
                            span { "Downloaded ebook " }
                            a { href: "/dioxus/torrents/{torrent.id}", target: "_blank", "torrent" }
                            span { " at {torrent.at}" }
                            br {}
                        },
                        "Wanted" => rsx! {
                            span { "Suggest wedge ebook " }
                            a { href: "/dioxus/torrents/{torrent.id}", target: "_blank", "torrent" }
                            span { " at {torrent.at}" }
                            br {}
                        },
                        "NotWanted" => rsx! {
                            span { "Skipped ebook " }
                            a { href: "/dioxus/torrents/{torrent.id}", target: "_blank", "torrent" }
                            span { " as an ebook was found at {torrent.at}" }
                            br {}
                        },
                        "Existing" => rsx! {
                            span { "Found matching ebook " }
                            a { href: "/dioxus/torrents/{torrent.id}", target: "_blank", "torrent" }
                            span { " in library at {torrent.at}" }
                            br {}
                        },
                        _ => rsx! {},
                    }
                } else if item.want_ebook {
                    span { class: "missing", "Ebook missing" }
                    br {}
                }

                if can_mark_done {
                    button {
                        disabled: *marking_done.read(),
                        onclick: move |_| {
                            let list_id = list_id.clone();
                            let item_id = item_id.clone();
                            marking_done.set(true);
                            spawn(async move {
                                match mark_list_item_done(list_id, item_id).await {
                                    // Trigger refresh - could use a signal for this
                                    Ok(_) => {}
                                    Err(e) => {
                                        tracing::error!("Failed to mark done: {}", e);
                                    }
                                }
                                marking_done.set(false);
                            });
                        },
                        "mark done"
                    }
                }
            }
        }
    }
}

fn mam_search_url(item: &ListItemDto) -> String {
    let base = "https://www.myanonamouse.net/tor/browse.php?thumbnail=true&tor[srchIn][title]=true&tor[srchIn][author]=true&tor[searchType]=all&tor[searchIn]=torrents";
    let search_text = format!("{} {}", item.title, item.authors.join(" "));
    let search_term = urlencoding::encode(&search_text);
    let mut result = base.to_string();
    result.push_str("&tor[text]=");
    result.push_str(&search_term);
    result
}
