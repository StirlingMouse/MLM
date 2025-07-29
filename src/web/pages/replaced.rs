use std::str::FromStr;
use std::{cell::RefCell, sync::Arc};

use anyhow::Result;
use askama::Template;
use axum::{
    extract::{OriginalUri, Query, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use native_db::Database;
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    data::{Language, Torrent, TorrentKey},
    linker::{refresh_metadata, refresh_metadata_relink},
    mam::MaM,
    web::{
        AppError, series,
        tables::{
            HidableColumns, Key, Pagination, PaginationParams, SortOn, Sortable, item, items,
            table_styles_rows,
        },
        time,
    },
};

pub async fn replaced_torrents_page(
    State(db): State<Arc<Database<'static>>>,
    Query(sort): Query<SortOn<TorrentsPageSort>>,
    Query(filter): Query<Vec<(TorrentsPageFilter, String)>>,
    Query(show): Query<TorrentsPageColumnsQuery>,
    Query(paging): Query<PaginationParams>,
) -> std::result::Result<Html<String>, AppError> {
    let torrents = db
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
                    TorrentsPageFilter::Kind => t.meta.main_cat.as_str() == value,
                    TorrentsPageFilter::Title => &t.meta.title == value,
                    TorrentsPageFilter::Author => t.meta.authors.contains(value),
                    TorrentsPageFilter::Narrator => t.meta.narrators.contains(value),
                    TorrentsPageFilter::Series => {
                        t.meta.series.iter().any(|(name, _)| name == value)
                    }
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

    let paging = paging.default_page_size(500, replaced_torrents.len());

    if let Some(sort_by) = &sort.sort_by {
        replaced_torrents.sort_by(|a, b| {
            let ord = match sort_by {
                TorrentsPageSort::Kind => a.meta.main_cat.cmp(&b.meta.main_cat),
                TorrentsPageSort::Title => a.meta.title.cmp(&b.meta.title),
                TorrentsPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                TorrentsPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                TorrentsPageSort::Series => a.meta.series.cmp(&b.meta.series),
                TorrentsPageSort::Language => a.meta.language.cmp(&b.meta.language),
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
        let Some(replacement) = db.r_transaction()?.get().primary(with.clone())? else {
            continue;
        };
        torrents.push((torrent, replacement));
    }

    let template = ReplacedTorrentsPageTemplate {
        paging: paging.unwrap_or_default(),
        sort,
        show: show.show.unwrap_or_default(),
        cols: Default::default(),
        torrents,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn replaced_torrents_page_post(
    State((config, db, mam)): State<(
        Arc<Config>,
        Arc<Database<'static>>,
        Arc<Result<Arc<MaM<'static>>>>,
    )>,
    uri: OriginalUri,
    Form(form): Form<TorrentsPageForm>,
) -> Result<Redirect, AppError> {
    match form.action.as_str() {
        "refresh" => {
            let Ok(mam) = mam.as_ref() else {
                return Err(anyhow::Error::msg("mam_id error").into());
            };
            for torrent in form.torrents {
                refresh_metadata(&db, &mam, torrent).await?;
            }
        }
        "refresh-relink" => {
            let Ok(mam) = mam.as_ref() else {
                return Err(anyhow::Error::msg("mam_id error").into());
            };
            for torrent in form.torrents {
                refresh_metadata_relink(&config, &db, &mam, torrent).await?;
            }
        }
        "remove" => {
            for torrent in form.torrents {
                let rw = db.rw_transaction()?;
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
    paging: Pagination,
    sort: SortOn<TorrentsPageSort>,
    show: TorrentsPageColumns,
    cols: RefCell<Vec<String>>,
    torrents: Vec<(Torrent, Torrent)>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TorrentsPageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Language,
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
            filetypes: false,
            path: false,
        };
        for column in value.split(",") {
            match column {
                "author" => columns.authors = true,
                "narrator" => columns.narrators = true,
                "series" => columns.series = true,
                "language" => columns.language = true,
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
    fn add_column(&self, size: &str) {
        self.cols.borrow_mut().push(size.to_owned());
    }
}
