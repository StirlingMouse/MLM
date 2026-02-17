#[cfg(feature = "server")]
use crate::dto::convert_torrent;
use crate::dto::Torrent;
use crate::utils::format_size;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use mlm_core::{Context, ContextExt, Torrent as DbTorrent, TorrentKey};
#[cfg(feature = "server")]
use sublime_fuzzy::FuzzySearch;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TorrentsPageSort {
    Kind,
    Category,
    Title,
    Edition,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Linker,
    QbitCategory,
    Linked,
    CreatedAt,
    UploadedAt,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TorrentsPageFilter {
    Kind,
    Category,
    Categories,
    Flags,
    Title,
    Author,
    Narrator,
    Series,
    Language,
    Filetype,
    Linker,
    QbitCategory,
    Linked,
    LibraryMismatch,
    ClientStatus,
    Abs,
    Query,
    Source,
    Metadata,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TorrentsData {
    pub torrents: Vec<Torrent>,
    pub total: usize,
    pub from: usize,
    pub page_size: usize,
}

#[server]
pub async fn get_torrents_data(
    sort: Option<TorrentsPageSort>,
    asc: bool,
    filters: Vec<(TorrentsPageFilter, String)>,
    from: Option<usize>,
    page_size: Option<usize>,
) -> Result<TorrentsData, ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let db = context.db();

    let from_val = from.unwrap_or(0);
    let page_size_val = page_size.unwrap_or(500);

    let _sort_val = sort.unwrap_or(TorrentsPageSort::CreatedAt);

    let r = db
        .r_transaction()
        .context("r_transaction")
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let torrents_iter = r
        .scan()
        .secondary::<DbTorrent>(TorrentKey::created_at)
        .context("scan")
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let torrents = torrents_iter
        .all()
        .context("all")
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .rev();

    let query = filters
        .iter()
        .find(|f| f.0 == TorrentsPageFilter::Query)
        .map(|f| f.1.clone());

    let mut filtered_torrents = Vec::new();

    for t_res in torrents {
        let t = t_res
            .context("torrent")
            .map_err(|e: anyhow::Error| ServerFnError::new(e.to_string()))?;

        let mut matches = true;
        for (field, value) in &filters {
            let ok = match field {
                TorrentsPageFilter::Query => true,
                TorrentsPageFilter::Kind => t.meta.media_type.as_str().eq_ignore_ascii_case(value),
                TorrentsPageFilter::Category | TorrentsPageFilter::Categories => {
                    t.meta.categories.iter().any(|c| c.eq_ignore_ascii_case(value))
                }
                TorrentsPageFilter::Flags => t
                    .meta
                    .flags
                    .as_ref()
                    .map(|f| format!("{:?}", f).eq_ignore_ascii_case(value))
                    .unwrap_or(false),
                TorrentsPageFilter::Title => t.meta.title.to_lowercase().contains(&value.to_lowercase()),
                TorrentsPageFilter::Author => t
                    .meta
                    .authors
                    .iter()
                    .any(|a| a.to_lowercase().contains(&value.to_lowercase())),
                TorrentsPageFilter::Narrator => t
                    .meta
                    .narrators
                    .iter()
                    .any(|n| n.to_lowercase().contains(&value.to_lowercase())),
                TorrentsPageFilter::Series => t
                    .meta
                    .series
                    .iter()
                    .any(|s| s.name.to_lowercase().contains(&value.to_lowercase())),
                TorrentsPageFilter::Language => t
                    .meta
                    .language
                    .as_ref()
                    .map(|l| l.to_string().eq_ignore_ascii_case(value))
                    .unwrap_or(false),
                TorrentsPageFilter::Filetype => t
                    .meta
                    .filetypes
                    .iter()
                    .any(|f| f.eq_ignore_ascii_case(value)),
                TorrentsPageFilter::Linker => t.linker.as_deref() == Some(value),
                TorrentsPageFilter::QbitCategory => t.category.as_deref() == Some(value),
                TorrentsPageFilter::Linked => {
                    let wants_linked = value.eq_ignore_ascii_case("true");
                    let is_linked = t.library_path.is_some();
                    wants_linked == is_linked
                }
                TorrentsPageFilter::LibraryMismatch => {
                    let wants_mismatch = value.eq_ignore_ascii_case("true");
                    let has_mismatch = t.library_path.is_some()
                        && t.library_files.is_empty();
                    wants_mismatch == has_mismatch
                }
                TorrentsPageFilter::ClientStatus => t
                    .client_status
                    .as_ref()
                    .map(|s| format!("{:?}", s).eq_ignore_ascii_case(value))
                    .unwrap_or(false),
                TorrentsPageFilter::Abs => {
                    let wants_abs = value.eq_ignore_ascii_case("true");
                    t.library_path.is_some() == wants_abs
                }
                TorrentsPageFilter::Source => {
                    format!("{:?}", t.meta.source).eq_ignore_ascii_case(value)
                }
                TorrentsPageFilter::Metadata => t.meta.ids.iter().any(|(k, _)| {
                    k.to_string().eq_ignore_ascii_case(value)
                }),
            };
            if !ok {
                matches = false;
                break;
            }
        }

        if matches {
            let mut score = 0;
            if let Some(ref q) = query {
                score = fuzzy_score(q, &t.meta.title);
                for author in &t.meta.authors {
                    score = score.max(fuzzy_score(q, author));
                }
                for narrator in &t.meta.narrators {
                    score = score.max(fuzzy_score(q, narrator));
                }
                for s in &t.meta.series {
                    score = score.max(fuzzy_score(q, &s.name));
                }

                if score < 10 {
                    continue;
                }
            }
            filtered_torrents.push((t, score));
        }
    }

    if let Some(sort_by) = sort {
        filtered_torrents.sort_by(|(a, _), (b, _)| {
            let ord = match sort_by {
                TorrentsPageSort::Kind => a.meta.media_type.cmp(&b.meta.media_type),
                TorrentsPageSort::Category => a
                    .meta
                    .cat
                    .partial_cmp(&b.meta.cat)
                    .unwrap_or(std::cmp::Ordering::Less),
                TorrentsPageSort::Title => a.meta.title.cmp(&b.meta.title),
                TorrentsPageSort::Edition => a
                    .meta
                    .edition
                    .as_ref()
                    .map(|e| e.1)
                    .cmp(&b.meta.edition.as_ref().map(|e| e.1))
                    .then(a.meta.edition.cmp(&b.meta.edition)),
                TorrentsPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                TorrentsPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                TorrentsPageSort::Series => a
                    .meta
                    .series
                    .cmp(&b.meta.series)
                    .then(a.meta.media_type.cmp(&b.meta.media_type)),
                TorrentsPageSort::Language => a.meta.language.cmp(&b.meta.language),
                TorrentsPageSort::Size => a.meta.size.cmp(&b.meta.size),
                TorrentsPageSort::Linker => a.linker.cmp(&b.linker),
                TorrentsPageSort::QbitCategory => a.category.cmp(&b.category),
                TorrentsPageSort::Linked => a.library_path.cmp(&b.library_path),
                TorrentsPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
                TorrentsPageSort::UploadedAt => a.meta.uploaded_at.cmp(&b.meta.uploaded_at),
            };
            if asc { ord } else { ord.reverse() }
        });
    } else if query.is_some() {
        filtered_torrents.sort_by_key(|(_, score)| -*score);
    }

    let total = filtered_torrents.len();
    let torrents = filtered_torrents
        .into_iter()
        .map(|(t, _)| convert_torrent(&t))
        .skip(from_val)
        .take(page_size_val)
        .collect();

    Ok(TorrentsData {
        torrents,
        total,
        from: from_val,
        page_size: page_size_val,
    })
}

#[cfg(feature = "server")]
fn fuzzy_score(query: &str, target: &str) -> isize {
    FuzzySearch::new(query, target)
        .case_insensitive()
        .best_match()
        .map_or(0, |m: sublime_fuzzy::Match| m.score())
}

#[component]
pub fn TorrentsPage() -> Element {
    let mut query_text = use_signal(String::new);

    let mut torrents_data = use_server_future(move || async move {
        let query = query_text.read().clone();
        let filters = if query.is_empty() {
            Vec::new()
        } else {
            vec![(TorrentsPageFilter::Query, query)]
        };
        get_torrents_data(None, false, filters, None, None).await
    })?;

    let data = torrents_data.suspend()?;
    let data = data.read();

    rsx! {
        div { class: "torrents-page",
            div { class: "row",
                h1 { "Torrents" }
                form {
                    class: "option_group query",
                    onsubmit: move |ev: Event<FormData>| {
                        ev.prevent_default();
                        torrents_data.restart();
                    },
                    input {
                        r#type: "text",
                        name: "query",
                        value: "{query_text}",
                        oninput: move |ev| query_text.set(ev.value()),
                        placeholder: "Search...",
                    }
                    input { r#type: "submit", value: "Search" }
                }
            }

            match &*data {
                Ok(data) => rsx! {
                    div { class: "torrents-table table",
                        div { class: "header", "Title" }
                        div { class: "header", "Size" }
                        for t in data.torrents.clone() {
                            div { class: "torrent-row",
                                a { href: "/dioxus/torrents/{t.id}", "{t.meta.title}" }
                            }
                            div { class: "torrent-row",
                                "{format_size(t.meta.size)}"
                            }
                        }
                    }
                    div { class: "pagination",
                        "Showing {data.from} to {data.from + data.torrents.len()} of {data.total}"
                    }
                },
                Err(e) => rsx! { p { class: "error", "Error: {e}" } },
            }
        }
    }
}
