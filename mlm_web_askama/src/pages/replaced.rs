use std::cell::Ref;
use std::cell::RefCell;
use std::str::FromStr;

use anyhow::Result;
use askama::Template;
use axum::response::{IntoResponse, Response};
use axum::{
    extract::{OriginalUri, Query, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use mlm_db::{Language, Torrent, TorrentKey, ids};
use serde::{Deserialize, Serialize};

use crate::{
    AppError, Page,
    tables::{self, Flex, HidableColumns, Key, Pagination, PaginationParams, SortOn, Sortable},
    time,
};
use mlm_core::{
    Context, ContextExt,
    linker::{refresh_mam_metadata, refresh_metadata_relink},
};
use mlm_db::DatabaseExt as _;

pub async fn replaced_torrents_page(
    State(context): State<Context>,
    uri: OriginalUri,
    Query(sort): Query<SortOn<TorrentsPageSort>>,
    Query(filter): Query<Vec<(TorrentsPageFilter, String)>>,
    Query(show): Query<TorrentsPageColumnsQuery>,
    Query(paging): Query<PaginationParams>,
) -> std::result::Result<Response, AppError> {
    let config = context.config().await;
    let torrents = context
        .db()
        .r_transaction()?
        .scan()
        .secondary::<Torrent>(TorrentKey::created_at)?;
    let mut replaced_torrents = torrents
        .all()?
        .rev()
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            if t.replaced_with.is_none() {
                return false;
            }
            for (field, value) in filter.iter() {
                let ok = match field {
                    TorrentsPageFilter::Kind => t.meta.media_type.as_str() == value,
                    TorrentsPageFilter::Title => &t.meta.title == value,
                    TorrentsPageFilter::Author => t.meta.authors.contains(value),
                    TorrentsPageFilter::Narrator => t.meta.narrators.contains(value),
                    TorrentsPageFilter::Series => t.meta.series.iter().any(|s| &s.name == value),
                    TorrentsPageFilter::Language => {
                        t.meta.language == Language::from_str(value).ok()
                    }
                    TorrentsPageFilter::Filetype => t.meta.filetypes.contains(value),
                    TorrentsPageFilter::Linked => t.library_path.is_some() == (value == "true"),
                    TorrentsPageFilter::SortBy => true,
                    TorrentsPageFilter::Asc => true,
                    TorrentsPageFilter::Show => true,
                    TorrentsPageFilter::From => true,
                    TorrentsPageFilter::PageSize => true,
                };
                if !ok {
                    return false;
                }
            }
            true
        })
        .collect::<Result<Vec<_>, _>>()?;

    let paging = match paging.default_page_size(uri, 500, replaced_torrents.len()) {
        Ok(paging) => paging,
        Err(redirect) => return Ok(redirect.into_response()),
    };

    if let Some(sort_by) = &sort.sort_by {
        replaced_torrents.sort_by(|a, b| {
            let ord = match sort_by {
                TorrentsPageSort::Kind => a.meta.media_type.cmp(&b.meta.media_type),
                TorrentsPageSort::Title => a.meta.title.cmp(&b.meta.title),
                TorrentsPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                TorrentsPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                TorrentsPageSort::Series => a.meta.series.cmp(&b.meta.series),
                TorrentsPageSort::Language => a.meta.language.cmp(&b.meta.language),
                TorrentsPageSort::Size => a.meta.size.cmp(&b.meta.size),
                TorrentsPageSort::Linked => a.library_path.cmp(&b.library_path),
                TorrentsPageSort::Replaced => a
                    .replaced_with
                    .as_ref()
                    .map(|r| r.1)
                    .cmp(&b.replaced_with.as_ref().map(|r| r.1)),
                TorrentsPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if sort.asc { ord.reverse() } else { ord }
        });
    }
    if let Some(paging) = &paging {
        replaced_torrents = replaced_torrents
            .into_iter()
            .skip(paging.from)
            .take(paging.page_size)
            .collect();
    }

    let mut torrents = vec![];
    for torrent in replaced_torrents {
        let Some((with, _)) = &torrent.replaced_with else {
            continue;
        };
        let Some(replacement) = context.db().r_transaction()?.get().primary(with.clone())? else {
            continue;
        };
        torrents.push((torrent, replacement));
    }

    let template = ReplacedTorrentsPageTemplate {
        abs_url: config.audiobookshelf.as_ref().map(|abs| abs.url.clone()),
        paging: paging.unwrap_or_default(),
        sort,
        show: show.show.unwrap_or_default(),
        cols: Default::default(),
        torrents,
    };
    Ok::<_, AppError>(Html(template.to_string()).into_response())
}

pub async fn replaced_torrents_page_post(
    State(context): State<Context>,
    uri: OriginalUri,
    Form(form): Form<TorrentsPageForm>,
) -> Result<Redirect, AppError> {
    let config = context.config().await;
    match form.action.as_str() {
        "refresh" => {
            let mam = context.mam()?;
            for torrent in form.torrents {
                refresh_mam_metadata(&config, context.db(), &mam, torrent, &context.events).await?;
            }
        }
        "refresh-relink" => {
            let mam = context.mam()?;
            for torrent in form.torrents {
                refresh_metadata_relink(&config, context.db(), &mam, torrent, &context.events)
                    .await?;
            }
        }
        "remove" => {
            for torrent in form.torrents {
                let (_guard, rw) = context.db().rw_async().await?;
                let Some(torrent) = rw.get().primary::<Torrent>(torrent)? else {
                    return Err(anyhow::Error::msg("Could not find torrent").into());
                };
                rw.remove(torrent)?;
                rw.commit()?;
            }
        }
        action => {
            eprintln!("unknown action: {action}");
        }
    }

    Ok(Redirect::to(&uri.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct TorrentsPageForm {
    action: String,
    #[serde(default, rename = "torrent")]
    torrents: Vec<String>,
}

#[derive(Template)]
#[template(path = "pages/replaced.html")]
struct ReplacedTorrentsPageTemplate {
    abs_url: Option<String>,
    paging: Pagination,
    sort: SortOn<TorrentsPageSort>,
    show: TorrentsPageColumns,
    cols: RefCell<Vec<Box<dyn tables::Size>>>,
    torrents: Vec<(Torrent, Torrent)>,
}

impl Page for ReplacedTorrentsPageTemplate {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TorrentsPageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Linked,
    Replaced,
    CreatedAt,
}

impl Key for TorrentsPageSort {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TorrentsPageFilter {
    Kind,
    Title,
    Author,
    Narrator,
    Series,
    Language,
    Filetype,
    Linked,
    // Workaround sort decode failure
    SortBy,
    Asc,
    Show,
    From,
    PageSize,
}

impl Key for TorrentsPageFilter {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
struct TorrentsPageColumns {
    authors: bool,
    narrators: bool,
    series: bool,
    language: bool,
    size: bool,
    filetypes: bool,
    path: bool,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TorrentsPageColumnsQuery {
    show: Option<TorrentsPageColumns>,
}

impl Default for TorrentsPageColumns {
    fn default() -> Self {
        TorrentsPageColumns {
            authors: true,
            narrators: true,
            series: true,
            language: false,
            size: true,
            filetypes: true,
            path: false,
        }
    }
}

impl TryFrom<String> for TorrentsPageColumns {
    type Error = String;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let mut columns = TorrentsPageColumns {
            authors: false,
            narrators: false,
            series: false,
            language: false,
            size: false,
            filetypes: false,
            path: false,
        };
        for column in value.split(",") {
            match column {
                "author" => columns.authors = true,
                "narrator" => columns.narrators = true,
                "series" => columns.series = true,
                "language" => columns.language = true,
                "size" => columns.size = true,
                "filetype" => columns.filetypes = true,
                "path" => columns.path = true,
                "" => {}
                _ => {
                    return Err(format!("Unknown column {column}"));
                }
            }
        }
        Ok(columns)
    }
}

impl Sortable for ReplacedTorrentsPageTemplate {
    type SortKey = TorrentsPageSort;

    fn get_current_sort(&self) -> SortOn<Self::SortKey> {
        self.sort
    }
}
impl HidableColumns for ReplacedTorrentsPageTemplate {
    fn add_column(&self, size: Box<dyn tables::Size>) {
        self.cols.borrow_mut().push(size);
    }

    fn get_columns(&self) -> Ref<'_, Vec<Box<dyn tables::Size>>> {
        self.cols.borrow()
    }
}
