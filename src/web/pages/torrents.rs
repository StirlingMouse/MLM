use std::str::FromStr;
use std::{cell::RefCell, sync::Arc};

use anyhow::Result;
use askama::Template;
use axum::response::{IntoResponse, Response};
use axum::{
    extract::{OriginalUri, Query, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use native_db::Database;
use serde::{Deserialize, Serialize};
use sublime_fuzzy::FuzzySearch;

use crate::data::{Category, ClientStatus};
use crate::web::{MaMState, Page};
use crate::{
    cleaner::clean_torrent,
    config::Config,
    data::{Language, LibraryMismatch, Torrent, TorrentKey},
    linker::{refresh_metadata, refresh_metadata_relink},
    web::{
        AppError, series,
        tables::{
            HidableColumns, Key, Pagination, PaginationParams, SortOn, Sortable, item, item_v,
            items, table_styles,
        },
        time,
    },
};

pub async fn torrents_page(
    State((config, db)): State<(Arc<Config>, Arc<Database<'static>>)>,
    uri: OriginalUri,
    Query(sort): Query<SortOn<TorrentsPageSort>>,
    Query(filter): Query<Vec<(TorrentsPageFilter, String)>>,
    Query(show): Query<TorrentsPageColumnsQuery>,
    Query(paging): Query<PaginationParams>,
) -> std::result::Result<Response, AppError> {
    let torrents = db
        .r_transaction()?
        .scan()
        .secondary::<Torrent>(TorrentKey::created_at)?;
    let query = filter
        .iter()
        .find(|(field, _)| field == &TorrentsPageFilter::Query)
        .and_then(|(_, value)| if value.is_empty() { None } else { Some(value) });
    let show = show.show.unwrap_or_default();

    let mut torrents = torrents
        .all()?
        .rev()
        .filter_map(|t| {
            let Ok(t) = t else {
                return Some(t.map(|t| (t, 0)));
            };
            let mut torrent_score = 0;
            for (field, value) in filter.iter() {
                let ok = match field {
                    TorrentsPageFilter::Kind => t.meta.main_cat.as_str() == value,
                    TorrentsPageFilter::Category => {
                        if value.is_empty() {
                            t.meta.cat.is_none()
                        } else {
                            false
                        }
                    }
                    TorrentsPageFilter::Title => &t.meta.title == value,
                    TorrentsPageFilter::Author => t.meta.authors.contains(value),
                    TorrentsPageFilter::Narrator => t.meta.narrators.contains(value),
                    TorrentsPageFilter::Series => t.meta.series.iter().any(|s| &s.name == value),
                    TorrentsPageFilter::Language => {
                        t.meta.language == Language::from_str(value).ok()
                    }
                    TorrentsPageFilter::Filetype => t.meta.filetypes.contains(value),
                    TorrentsPageFilter::Linked => t.library_path.is_some() == (value == "true"),
                    TorrentsPageFilter::LibraryMismatch => {
                        if value.is_empty() {
                            t.library_mismatch.is_some()
                        } else {
                            match t.library_mismatch {
                                Some(LibraryMismatch::NewLibraryDir(ref path)) => {
                                    value == "new_library"
                                        || value.as_str() == path.to_string_lossy()
                                }
                                Some(LibraryMismatch::NewPath(ref path)) => {
                                    value == "new_path" || value.as_str() == path.to_string_lossy()
                                }
                                Some(LibraryMismatch::NoLibrary) => value == "no_library",
                                None => false,
                            }
                        }
                    }
                    TorrentsPageFilter::ClientStatus => match t.client_status {
                        Some(ClientStatus::NotInClient) => value == "not_in_client",
                        Some(ClientStatus::RemovedFromMam) => value == "removed_from_mam",
                        None => false,
                    },
                    TorrentsPageFilter::Query => true,
                    TorrentsPageFilter::SortBy => true,
                    TorrentsPageFilter::Asc => true,
                    TorrentsPageFilter::Show => true,
                    TorrentsPageFilter::From => true,
                    TorrentsPageFilter::PageSize => true,
                };
                if !ok {
                    return None;
                }
                if field == &TorrentsPageFilter::Query && !value.is_empty() {
                    torrent_score += score(value, &t.meta.title);
                    if show.authors {
                        for author in &t.meta.authors {
                            torrent_score += score(value, author);
                        }
                    }
                    if show.narrators {
                        for narrator in &t.meta.narrators {
                            torrent_score += score(value, narrator);
                        }
                    }
                    if show.series {
                        for s in &t.meta.series {
                            torrent_score += score(value, &s.name);
                        }
                    }
                }
            }
            if query.is_some() && torrent_score < 10 {
                return None;
            }
            Some(Ok((t, torrent_score)))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let paging = match paging.default_page_size(uri, 500, torrents.len()) {
        Ok(paging) => paging,
        Err(redirect) => return Ok(redirect.into_response()),
    };

    if sort.sort_by.is_none() && query.is_some() {
        torrents.sort_by_key(|(_, score)| -*score);
    }
    let mut torrents = torrents.into_iter().map(|(t, _)| t).collect::<Vec<_>>();
    if let Some(sort_by) = &sort.sort_by {
        torrents.sort_by(|a, b| {
            let ord = match sort_by {
                TorrentsPageSort::Kind => a.meta.main_cat.cmp(&b.meta.main_cat),
                TorrentsPageSort::Title => a.meta.title.cmp(&b.meta.title),
                TorrentsPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                TorrentsPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                TorrentsPageSort::Series => a.meta.series.cmp(&b.meta.series),
                TorrentsPageSort::Language => a.meta.language.cmp(&b.meta.language),
                TorrentsPageSort::Size => a.meta.size.cmp(&b.meta.size),
                TorrentsPageSort::Linked => a.library_path.cmp(&b.library_path),
                TorrentsPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if sort.asc { ord.reverse() } else { ord }
        });
    }
    if let Some(paging) = &paging {
        torrents = torrents
            .into_iter()
            .skip(paging.from)
            .take(paging.page_size)
            .collect();
    }

    let template = TorrentsPageTemplate {
        abs_url: config.audiobookshelf.as_ref().map(|abs| abs.url.clone()),
        paging: paging.unwrap_or_default(),
        sort,
        show,
        cols: Default::default(),
        query: query.map(|q| q.as_str()).unwrap_or("").to_owned(),
        torrents,
    };
    Ok::<_, AppError>(Html(template.to_string()).into_response())
}

fn score(query: &str, target: &str) -> isize {
    FuzzySearch::new(query, target)
        .case_insensitive()
        .best_match()
        .map_or(0, |m| m.score())
}

pub async fn torrents_page_post(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    uri: OriginalUri,
    Form(form): Form<TorrentsPageForm>,
) -> Result<Redirect, AppError> {
    match form.action.as_str() {
        "clean" => {
            for torrent in form.torrents {
                let Some(torrent) = db.r_transaction()?.get().primary(torrent)? else {
                    return Err(anyhow::Error::msg("Could not find torrent").into());
                };
                clean_torrent(&config, &db, torrent).await?;
            }
        }
        "refresh" => {
            let Ok(mam) = mam.as_ref() else {
                return Err(anyhow::Error::msg("mam_id error").into());
            };
            for torrent in form.torrents {
                refresh_metadata(&db, mam, torrent).await?;
            }
        }
        "refresh-relink" => {
            let Ok(mam) = mam.as_ref() else {
                return Err(anyhow::Error::msg("mam_id error").into());
            };
            for torrent in form.torrents {
                refresh_metadata_relink(&config, &db, mam, torrent).await?;
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
#[template(path = "pages/torrents.html")]
struct TorrentsPageTemplate {
    abs_url: Option<String>,
    paging: Pagination,
    sort: SortOn<TorrentsPageSort>,
    show: TorrentsPageColumns,
    cols: RefCell<Vec<String>>,
    query: String,
    torrents: Vec<Torrent>,
}

impl Page for TorrentsPageTemplate {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
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
    CreatedAt,
}

impl Key for TorrentsPageSort {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TorrentsPageFilter {
    Kind,
    Category,
    Title,
    Author,
    Narrator,
    Series,
    Language,
    Filetype,
    Linked,
    LibraryMismatch,
    ClientStatus,
    Query,
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

impl Sortable for TorrentsPageTemplate {
    type SortKey = TorrentsPageSort;

    fn get_current_sort(&self) -> SortOn<Self::SortKey> {
        self.sort
    }
}
impl HidableColumns for TorrentsPageTemplate {
    fn add_column(&self, size: &str) {
        self.cols.borrow_mut().push(size.to_owned());
    }
}

impl Torrent {
    pub fn cat_name(&self) -> &str {
        match self.meta.cat {
            Some(Category::Audio(cat)) => cat.to_str(),
            Some(Category::Ebook(cat)) => cat.to_str(),
            None => "N/A",
        }
    }
}
