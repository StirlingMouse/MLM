#[cfg(feature = "server")]
use crate::error::{IntoServerFnError, OptionIntoServerFnError};
use crate::sse::STATS_UPDATE_TRIGGER;
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
use crate::components::Pagination;
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
    pub total: usize,
    pub from: usize,
    pub page_size: usize,
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

fn render_list_torrent_link(torrent: &ListItemTorrentDto) -> Element {
    if let Some(id) = &torrent.id {
        rsx! {
            a {
                href: "/torrents/{id}",
                target: "_blank",
                rel: "noopener noreferrer",
                "torrent"
            }
        }
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
pub async fn get_list_data(
    list_id: String,
    from: Option<usize>,
    page_size: Option<usize>,
    show: Option<String>,
) -> Result<ListPageData, ServerFnError> {
    use mlm_core::ContextExt;
    use mlm_db::{List, ListItem, ListItemKey};

    let context = crate::error::get_context()?;
    let r = context
        .db()
        .r_transaction()
        .server_err_ctx("opening read transaction for list page")?;

    let list = r
        .get()
        .primary::<List>(list_id.as_str())
        .server_err_ctx("loading list")?
        .ok_or_server_err("List not found")?;

    let all_items = r
        .scan()
        .secondary::<ListItem>(ListItemKey::list_id)
        .server_err_ctx("scanning list items")?
        .range(list.id.clone()..=list.id.clone())
        .server_err_ctx("scoping list items to list id")?
        .filter_map(|result| match result {
            Ok(item) => Some(item),
            Err(err) => {
                tracing::error!("skipping list item row after scan error: {err}");
                None
            }
        })
        .collect::<Vec<_>>();

    // Sort newest-first
    let mut items = all_items;
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Apply show filter server-side
    let filtered_items: Vec<ListItem> = items
        .into_iter()
        .filter(|item| {
            let want_audio = item_wants_audio(item);
            let want_ebook = item_wants_ebook(item);
            match show.as_deref() {
                Some("any") => want_audio || want_ebook,
                Some("audio") => want_audio,
                Some("ebook") => want_ebook,
                _ => true,
            }
        })
        .collect();

    let total = filtered_items.len();
    let from_val = from.unwrap_or(0);
    let page_size_val = page_size.unwrap_or(500);

    // Clamp from_val to valid range
    let from_val = if page_size_val > 0 && from_val >= total && total > 0 {
        ((total - 1) / page_size_val) * page_size_val
    } else {
        from_val
    };

    let items_dto = filtered_items
        .into_iter()
        .skip(from_val)
        .take(page_size_val)
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
        total,
        from: from_val,
        page_size: page_size_val,
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
        .server_err_ctx("opening write transaction for marking list item done")?;
    let mut item = rw
        .get()
        .primary::<ListItem>((list_id.as_str(), item_id.as_str()))
        .server_err_ctx("loading list item")?
        .ok_or_server_err("Could not find item")?;
    item.marked_done_at = Some(Timestamp::now());
    rw.upsert(item)
        .server_err_ctx("upserting completed list item")?;
    rw.commit()
        .server_err_ctx("committing list item completion")?;

    Ok(())
}

#[component]
pub fn ListPage(id: String) -> Element {
    let list_id = id.clone();
    let mut cached_data = use_signal(|| None::<ListPageData>);

    let mut from = use_signal(|| 0usize);
    let mut show = use_signal(|| None::<String>);

    let list_id_clone = list_id.clone();
    let from_clone = from.clone();
    let show_clone = show.clone();

    let mut list_data = match use_server_future(move || {
        let list_id = list_id_clone.clone();
        let from = *from_clone.read();
        let show = show_clone.read().clone();
        async move { get_list_data(list_id, Some(from), Some(500), show).await }
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

    let data = data_to_show.clone();
    let on_page_change = move |new_from: usize| {
        from.set(new_from);
        list_data.restart();
    };

    let on_filter_change = move |new_show: Option<String>| {
        show.set(new_show);
        from.set(0);
        list_data.restart();
    };

    rsx! {
        if let Some(data) = data {
            ListPageContent {
                list_id: id,
                data,
                on_refresh: move |_| list_data.restart(),
                on_page_change,
                on_filter_change,
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
    on_page_change: Callback<usize>,
    on_filter_change: Callback<Option<String>>,
}

#[component]
fn ListPageContent(props: ListPageContentProps) -> Element {
    let list_id = props.list_id.clone();

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
                            checked: true,
                            onclick: move |_| {
                                props.on_filter_change.call(None);
                            },
                        }
                    }
                    label {
                        "Any Missing"
                        input {
                            r#type: "radio",
                            name: "show",
                            value: "any",
                            onclick: move |_| {
                                props.on_filter_change.call(Some("any".to_string()));
                            },
                        }
                    }
                    label {
                        "Audio Missing"
                        input {
                            r#type: "radio",
                            name: "show",
                            value: "audio",
                            onclick: move |_| {
                                props.on_filter_change.call(Some("audio".to_string()));
                            },
                        }
                    }
                    label {
                        "Ebook Missing"
                        input {
                            r#type: "radio",
                            name: "show",
                            value: "ebook",
                            onclick: move |_| {
                                props.on_filter_change.call(Some("ebook".to_string()));
                            },
                        }
                    }
                }
            }

            for item in &props.data.items {
                ListItemComponent {
                    list_id: list_id.clone(),
                    item: item.clone(),
                    on_refresh: props.on_refresh,
                }
            }

            if props.data.items.is_empty() {
                p {
                    i { "The list is empty" }
                }
            }

            if props.data.total > props.data.page_size {
                Pagination {
                    total: props.data.total,
                    from: props.data.from,
                    page_size: props.data.page_size,
                    on_change: move |new_from| {
                        props.on_page_change.call(new_from);
                    },
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
                        a {
                            href: "{mam_search_url(&item)}",
                            target: "_blank",
                            rel: "noopener noreferrer",
                            "search on MaM"
                        }
                        if let Some(url) = &item.book_url {
                            a {
                                href: "{url}",
                                target: "_blank",
                                rel: "noopener noreferrer",
                                "goodreads"
                            }
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
