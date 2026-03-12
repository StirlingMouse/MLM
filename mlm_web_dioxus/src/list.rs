use crate::sse::STATS_UPDATE_TRIGGER;
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ListItemTorrentDto {
    pub id: Option<String>,
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
    item.want_audio()
}

#[cfg(feature = "server")]
fn item_wants_ebook(item: &mlm_db::ListItem) -> bool {
    item.want_ebook()
}

fn matches_show_filter(item: &ListItemDto, show: Option<&str>) -> bool {
    match show {
        Some("any") => item.want_audio || item.want_ebook,
        Some("audio") => item.want_audio,
        Some("ebook") => item.want_ebook,
        _ => true,
    }
}

fn render_list_torrent_link(torrent: &ListItemTorrentDto) -> Element {
    if let Some(id) = &torrent.id {
        rsx! { a { href: "/dioxus/torrents/{id}", target: "_blank", rel: "noopener noreferrer", "torrent" } }
    } else {
        rsx! { "torrent" }
    }
}

fn render_list_torrent_status(
    torrent: &ListItemTorrentDto,
    format_name: &str,
    skipped_reason: &'static str,
) -> Element {
    match torrent.status.as_str() {
        "Selected" => rsx! {
            span { "Downloaded {format_name} " }
            {render_list_torrent_link(torrent)}
            span { " at {torrent.at}" }
            br {}
        },
        "Wanted" => rsx! {
            span { "Suggest wedge {format_name} " }
            {render_list_torrent_link(torrent)}
            span { " at {torrent.at}" }
            br {}
        },
        "NotWanted" => rsx! {
            span { "Skipped {format_name} " }
            {render_list_torrent_link(torrent)}
            span { " {skipped_reason} at {torrent.at}" }
            br {}
        },
        "Existing" => rsx! {
            span { "Found matching {format_name} " }
            {render_list_torrent_link(torrent)}
            span { " in library at {torrent.at}" }
            br {}
        },
        _ => rsx! {},
    }
}

#[server]
pub async fn get_list_data(list_id: String) -> Result<ListPageData, ServerFnError> {
    use mlm_core::ContextExt;
    use mlm_db::{List, ListItem, ListItemKey};

    let context = crate::error::get_context()?;
    let r = context
        .db()
        .r_transaction()
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let list = r
        .get()
        .primary::<List>(list_id.as_str())
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("List not found"))?;

    let mut items = r
        .scan()
        .secondary::<ListItem>(ListItemKey::list_id)
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .range(Some(list.id.clone())..=Some(list.id.clone()))
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let items_dto = items
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
                    id: t.torrent_id,
                    mam_id: t.mam_id,
                    status: torrent_status_to_string(t.status),
                    at: format_timestamp_db(&t.at),
                }),
                ebook_torrent: item.ebook_torrent.map(|t| ListItemTorrentDto {
                    id: t.torrent_id,
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
    })
}

#[server]
pub async fn mark_list_item_done(list_id: String, item_id: String) -> Result<(), ServerFnError> {
    use mlm_core::ContextExt;
    use mlm_db::{DatabaseExt as _, ListItem, Timestamp};

    let context = crate::error::get_context()?;
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

#[component]
pub fn ListPage(id: String) -> Element {
    let list_id = id.clone();
    let mut cached_data = use_signal(|| None::<ListPageData>);

    let mut list_data = match use_server_future(move || {
        let list_id = list_id.clone();
        async move { get_list_data(list_id).await }
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

    use_effect(move || {
        let val = current_value.read();
        if let Some(Ok(data)) = &*val {
            cached_data.set(Some(data.clone()));
        }
    });

    let data_to_show = {
        let val = current_value.read();
        match &*val {
            Some(Ok(data)) => Some(data.clone()),
            Some(Err(_)) | None => cached_data.read().clone(),
        }
    };

    rsx! {
        if let Some(data) = data_to_show {
            ListPageContent {
                list_id: id,
                data,
                on_refresh: move |_| list_data.restart(),
            }
        } else {
            p { "Loading..." }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ListPageContentProps {
    list_id: String,
    data: ListPageData,
    on_refresh: EventHandler<()>,
}

#[component]
fn ListPageContent(props: ListPageContentProps) -> Element {
    let list_id = props.list_id.clone();
    let mut show = use_signal(|| None::<String>);

    let items: Vec<ListItemDto> = props
        .data
        .items
        .iter()
        .filter(|item| matches_show_filter(item, show.read().as_deref()))
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
                ListItemComponent {
                    list_id: list_id.clone(),
                    item: item.clone(),
                    on_refresh: props.on_refresh,
                }
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
    on_refresh: EventHandler<()>,
}

#[component]
fn ListItemComponent(props: ListItemComponentProps) -> Element {
    let list_id = props.list_id.clone();
    let item = props.item.clone();
    let guid = props.item.guid.clone();
    let item_id = guid.1.clone();
    let can_mark_done = props.item.want_audio || props.item.want_ebook;
    let on_refresh = props.on_refresh;

    let mut marking_done = use_signal(|| false);

    let authors_str = props.item.authors.join(", ");

    rsx! {
        div { class: "list_item",
            img { src: "{item.cover_url}" }
            div {
                div { class: "row",
                    h3 { "{item.title}" }
                    div {
                        a { href: "{mam_search_url(&item)}", target: "_blank", rel: "noopener noreferrer", "search on MaM" }
                        if let Some(url) = &item.book_url {
                            a { href: "{url}", target: "_blank", rel: "noopener noreferrer", "goodreads" }
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
                    {render_list_torrent_status(torrent, "audiobook", "as an ebook was found")}
                } else if item.want_audio {
                    span { class: "missing", "Audiobook missing" }
                    br {}
                }

                if let Some(torrent) = &item.ebook_torrent {
                    {render_list_torrent_status(torrent, "ebook", "as an ebook was found")}
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
                                    Ok(_) => on_refresh.call(()),
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
