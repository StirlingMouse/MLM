use std::cell::Ref;
use std::mem;
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

use crate::data::{Category, ClientStatus, MainCat, MetadataSource, Series, SeriesEntry};
use crate::mam_enums::Flags;
use crate::web::{MaMState, Page, tables};
use crate::{
    cleaner::clean_torrent,
    config::Config,
    data::{Language, LibraryMismatch, Torrent, TorrentKey},
    linker::{refresh_metadata, refresh_metadata_relink},
    web::{
        AppError,
        tables::{Flex, HidableColumns, Key, Pagination, PaginationParams, SortOn, Sortable},
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
    let metadata = filter
        .iter()
        .find(|(field, _)| field == &TorrentsPageFilter::Metadata)
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
                        } else if let Some(cat) = &t.meta.cat {
                            let cats = value
                                .split(",")
                                .filter_map(|id| id.parse().ok())
                                .filter_map(Category::from_one_id)
                                .collect::<Vec<_>>();
                            cats.contains(cat) || cat.as_str() == value
                        } else {
                            false
                        }
                    }
                    TorrentsPageFilter::Flags => {
                        if value.is_empty() {
                            t.meta.flags.is_none_or(|f| f.0 == 0)
                        } else if let Some(flags) = &t.meta.flags {
                            let flags = Flags::from_bitfield(flags.0);
                            match value.as_str() {
                                "violence" => flags.violence == Some(true),
                                "explicit" => flags.explicit == Some(true),
                                "some_explicit" => flags.some_explicit == Some(true),
                                "language" => flags.crude_language == Some(true),
                                "abridged" => flags.abridged == Some(true),
                                "lgbt" => flags.lgbt == Some(true),
                                _ => false,
                            }
                        } else {
                            false
                        }
                    }
                    TorrentsPageFilter::Title => &t.meta.title == value,
                    TorrentsPageFilter::Author => {
                        if value.is_empty() {
                            t.meta.authors.is_empty()
                        } else {
                            t.meta.authors.contains(value)
                        }
                    }
                    TorrentsPageFilter::Narrator => {
                        if value.is_empty() {
                            t.meta.narrators.is_empty()
                        } else {
                            t.meta.narrators.contains(value)
                        }
                    }
                    TorrentsPageFilter::Series => {
                        if value.is_empty() {
                            t.meta.series.is_empty()
                        } else {
                            t.meta.series.iter().any(|s| &s.name == value)
                        }
                    }
                    TorrentsPageFilter::Language => {
                        if value.is_empty() {
                            t.meta.language.is_none()
                        } else {
                            t.meta.language == Language::from_str(value).ok()
                        }
                    }
                    TorrentsPageFilter::Filetype => t.meta.filetypes.contains(value),
                    TorrentsPageFilter::Linker => {
                        if value.is_empty() {
                            t.linker.is_none()
                        } else {
                            t.linker.as_ref() == Some(value)
                        }
                    }
                    TorrentsPageFilter::QbitCategory => {
                        if value.is_empty() {
                            t.category.is_none()
                        } else {
                            t.category.as_ref() == Some(value)
                        }
                    }
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
                    TorrentsPageFilter::Abs => t.abs_id.is_some() == (value == "true"),
                    TorrentsPageFilter::Source => match value.as_str() {
                        "mam" => t.meta.source == MetadataSource::Mam,
                        "manual" => t.meta.source == MetadataSource::Manual,
                        _ => false,
                    },
                    TorrentsPageFilter::Query => true,
                    TorrentsPageFilter::Metadata => true,
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

    if sort.sort_by.is_none() && query.is_some() {
        torrents.sort_by_key(|(_, score)| -*score);
    }
    let mut torrents = torrents.into_iter().map(|(t, _)| t).collect::<Vec<_>>();
    if let Some(metadata) = metadata {
        match metadata.as_str() {
            "title" => {
                torrents.sort_by(|a, b| a.title_search.cmp(&b.title_search));
                let mut batch: Vec<Torrent> = vec![];
                let mut new_torrents: Vec<Torrent> = vec![];
                for torrent in torrents {
                    if let Some(current) = batch.first() {
                        if current.title_search != torrent.title_search {
                            if batch.len() > 1
                                && !batch.iter().all(|t| t.meta.title == current.meta.title)
                            {
                                new_torrents.extend(mem::take(&mut batch));
                            } else {
                                batch.clear();
                            }
                        }
                        batch.push(torrent);
                    } else {
                        batch.push(torrent);
                    }
                }
                torrents = new_torrents;
            }
            "authors" => {
                torrents.sort_by(|a, b| a.title_search.cmp(&b.title_search));
                let mut batch: Vec<Torrent> = vec![];
                let mut new_torrents: Vec<Torrent> = vec![];
                for torrent in torrents {
                    if let Some(current) = batch.first() {
                        if current.title_search != torrent.title_search {
                            if batch.len() > 1
                                && !batch.iter().all(|t| t.meta.authors == current.meta.authors)
                            {
                                new_torrents.extend(mem::take(&mut batch));
                            } else {
                                batch.clear();
                            }
                        }
                        batch.push(torrent);
                    } else {
                        batch.push(torrent);
                    }
                }
                torrents = new_torrents;
            }
            "series" => {
                torrents.sort_by(|a, b| a.title_search.cmp(&b.title_search));
                let mut batch: Vec<Torrent> = vec![];
                let mut new_torrents: Vec<Torrent> = vec![];
                for torrent in torrents {
                    if let Some(current) = batch.first() {
                        if current.title_search != torrent.title_search {
                            if batch.len() > 1
                                && !batch.iter().all(|t| t.meta.series == current.meta.series)
                            {
                                new_torrents.extend(mem::take(&mut batch));
                            } else {
                                batch.clear();
                            }
                        }
                        batch.push(torrent);
                    } else {
                        batch.push(torrent);
                    }
                }
                torrents = new_torrents;
            }
            "leading" => {
                fn remove_leading(title: &str) -> &str {
                    title
                        .strip_prefix("the ")
                        .or_else(|| title.strip_prefix("a "))
                        .unwrap_or(title)
                }
                torrents.sort_by(|a, b| {
                    remove_leading(&a.title_search).cmp(remove_leading(&b.title_search))
                });
                let mut batch: Vec<Torrent> = vec![];
                let mut new_torrents: Vec<Torrent> = vec![];
                for torrent in torrents {
                    if let Some(current) = batch.first() {
                        if remove_leading(&current.title_search)
                            != remove_leading(&torrent.title_search)
                        {
                            if batch.len() > 1
                                && !batch.iter().all(|t| t.meta.title == current.meta.title)
                            {
                                new_torrents.extend(mem::take(&mut batch));
                            } else {
                                batch.clear();
                            }
                        }
                        batch.push(torrent);
                    } else {
                        batch.push(torrent);
                    }
                }
                torrents = new_torrents;
            }
            "subtitle" => {
                fn subtitle(title: &str) -> &str {
                    let Some((title, _subtitle)) = title.split_once(':') else {
                        return title;
                    };
                    title
                }
                torrents.sort_by(|a, b| subtitle(&a.title_search).cmp(subtitle(&b.title_search)));
                let mut batch: Vec<Torrent> = vec![];
                let mut new_torrents: Vec<Torrent> = vec![];
                for torrent in torrents {
                    if let Some(current) = batch.first() {
                        if subtitle(&current.title_search) != subtitle(&torrent.title_search) {
                            if batch.len() > 1
                                && !batch.iter().all(|t| t.meta.title == current.meta.title)
                            {
                                new_torrents.extend(mem::take(&mut batch));
                            } else {
                                batch.clear();
                            }
                        }
                        batch.push(torrent);
                    } else {
                        batch.push(torrent);
                    }
                }
                torrents = new_torrents;
            }
            "missing_ebook" => {
                torrents.sort_by(|a, b| a.title_search.cmp(&b.title_search));
                let mut batch: Vec<Torrent> = vec![];
                let mut new_torrents: Vec<Torrent> = vec![];
                for torrent in torrents {
                    if let Some(current) = batch.first() {
                        if current.title_search != torrent.title_search {
                            if batch.iter().any(|t| t.meta.main_cat == MainCat::Audio)
                                && !batch.iter().any(|t| t.meta.main_cat == MainCat::Ebook)
                            {
                                new_torrents.extend(mem::take(&mut batch));
                            } else {
                                batch.clear();
                            }
                        }
                        batch.push(torrent);
                    } else {
                        batch.push(torrent);
                    }
                }
                torrents = new_torrents;
            }
            "initials" => {
                fn bunched_initials(name: &str) -> bool {
                    let mut capital = false;
                    for char in name.chars() {
                        if capital && char.is_uppercase() {
                            return true;
                        }
                        capital = char.is_uppercase()
                    }
                    false
                }
                torrents.retain(|t| {
                    t.meta.authors.iter().any(|name| bunched_initials(name))
                        || t.meta.narrators.iter().any(|name| bunched_initials(name))
                });
            }
            "series_with_holes" => {
                fn first_series(series: &[Series]) -> Option<&Series> {
                    series.iter().find(|s| !s.entries.0.is_empty())
                }
                fn series_name(series: &[Series]) -> &str {
                    first_series(series)
                        .map(|s| s.name.as_str())
                        .unwrap_or_default()
                }
                torrents
                    .sort_by(|a, b| series_name(&a.meta.series).cmp(series_name(&b.meta.series)));
                let mut batch: Vec<(Torrent, Series)> = vec![];
                let mut new_torrents: Vec<Torrent> = vec![];
                for torrent in torrents {
                    let Some(series) = first_series(&torrent.meta.series) else {
                        continue;
                    };
                    if let Some(current) = batch.first() {
                        if current.1.name != series.name {
                            if batch.iter().any(|t| t.0.library_path.is_some()) {
                                batch.sort_by(|a, b| a.1.entries.cmp(&b.1.entries));
                                let last = batch
                                    .iter()
                                    .flat_map(|s| &s.1.entries.0)
                                    .map(|s| match s {
                                        SeriesEntry::Num(n) => *n,
                                        SeriesEntry::Range(_start, end) => *end,
                                        SeriesEntry::Part(n, _) => *n,
                                    } as i32)
                                    .max()
                                    .unwrap_or_default();
                                for i in 1..=last {
                                    if !batch
                                        .iter()
                                        .any(|(_, series)| series.entries.contains(i as f32))
                                    {
                                        new_torrents.extend(
                                            mem::take(&mut batch).into_iter().map(|(t, _)| t),
                                        );
                                        break;
                                    }
                                }
                            }
                            batch.clear();
                        }
                        let series = series.clone();
                        batch.push((torrent, series));
                    } else {
                        let series = series.clone();
                        batch.push((torrent, series));
                    }
                }
                torrents = new_torrents;
            }
            "series_authors" => {
                fn first_series(series: &[Series]) -> Option<&Series> {
                    series.iter().find(|s| !s.entries.0.is_empty())
                }
                fn series_name(series: &[Series]) -> &str {
                    first_series(series)
                        .map(|s| s.name.as_str())
                        .unwrap_or_default()
                }
                torrents
                    .sort_by(|a, b| series_name(&a.meta.series).cmp(series_name(&b.meta.series)));
                let mut batch: Vec<(Torrent, Series)> = vec![];
                let mut new_torrents: Vec<Torrent> = vec![];
                for torrent in torrents {
                    let Some(series) = first_series(&torrent.meta.series) else {
                        continue;
                    };
                    if let Some(current) = batch.first() {
                        if current.1.name != series.name {
                            if batch.len() > 1
                                && !batch
                                    .iter()
                                    .all(|t| t.0.meta.authors == current.0.meta.authors)
                            {
                                new_torrents
                                    .extend(mem::take(&mut batch).into_iter().map(|(t, _)| t));
                            }
                            batch.clear();
                        }
                        let series = series.clone();
                        batch.push((torrent, series));
                    } else {
                        let series = series.clone();
                        batch.push((torrent, series));
                    }
                }
                torrents = new_torrents;
            }
            _ => return Err(anyhow::Error::msg("Unknown metadata filter").into()),
        }
    }

    let paging = match paging.default_page_size(uri, 500, torrents.len()) {
        Ok(paging) => paging,
        Err(redirect) => return Ok(redirect.into_response()),
    };

    if let Some(sort_by) = &sort.sort_by {
        torrents.sort_by(|a, b| {
            let ord = match sort_by {
                TorrentsPageSort::Kind => a.meta.main_cat.cmp(&b.meta.main_cat),
                TorrentsPageSort::Category => a
                    .meta
                    .cat
                    .partial_cmp(&b.meta.cat)
                    .unwrap_or(std::cmp::Ordering::Less),
                TorrentsPageSort::Title => a.meta.title.cmp(&b.meta.title),
                TorrentsPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                TorrentsPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                TorrentsPageSort::Series => a
                    .meta
                    .series
                    .cmp(&b.meta.series)
                    .then(a.meta.main_cat.cmp(&b.meta.main_cat)),
                TorrentsPageSort::Language => a.meta.language.cmp(&b.meta.language),
                TorrentsPageSort::Size => a.meta.size.cmp(&b.meta.size),
                TorrentsPageSort::Linker => a.linker.cmp(&b.linker),
                TorrentsPageSort::QbitCategory => a.category.cmp(&b.category),
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
                clean_torrent(&config, &db, torrent, true).await?;
            }
        }
        "refresh" => {
            let Ok(mam) = mam.as_ref() else {
                return Err(anyhow::Error::msg("mam_id error").into());
            };
            for torrent in form.torrents {
                refresh_metadata(&config, &db, mam, torrent).await?;
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
    cols: RefCell<Vec<Box<dyn tables::Size>>>,
    query: String,
    torrents: Vec<Torrent>,
}

impl Page for TorrentsPageTemplate {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TorrentsPageSort {
    Kind,
    Category,
    Title,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Linker,
    QbitCategory,
    Linked,
    CreatedAt,
}

impl Key for TorrentsPageSort {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TorrentsPageFilter {
    Kind,
    Category,
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
    category: bool,
    flags: bool,
    authors: bool,
    narrators: bool,
    series: bool,
    language: bool,
    size: bool,
    filetypes: bool,
    linker: bool,
    qbit_category: bool,
    path: bool,
    created_at: bool,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TorrentsPageColumnsQuery {
    show: Option<TorrentsPageColumns>,
}

impl Default for TorrentsPageColumns {
    fn default() -> Self {
        TorrentsPageColumns {
            category: false,
            flags: false,
            authors: true,
            narrators: true,
            series: true,
            language: false,
            size: true,
            filetypes: true,
            linker: false,
            qbit_category: false,
            path: false,
            created_at: true,
        }
    }
}

impl TryFrom<String> for TorrentsPageColumns {
    type Error = String;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let mut columns = TorrentsPageColumns {
            category: false,
            flags: false,
            authors: false,
            narrators: false,
            series: false,
            language: false,
            size: false,
            filetypes: false,
            linker: false,
            qbit_category: false,
            path: false,
            created_at: false,
        };
        for column in value.split(",") {
            match column {
                "category" => columns.category = true,
                "flags" => columns.flags = true,
                "author" => columns.authors = true,
                "narrator" => columns.narrators = true,
                "series" => columns.series = true,
                "language" => columns.language = true,
                "size" => columns.size = true,
                "filetype" => columns.filetypes = true,
                "linker" => columns.linker = true,
                "qbit_category" => columns.qbit_category = true,
                "path" => columns.path = true,
                "created_at" => columns.created_at = true,
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
    fn add_column(&self, size: Box<dyn tables::Size>) {
        self.cols.borrow_mut().push(size);
    }

    fn get_columns(&self) -> Ref<'_, Vec<Box<dyn tables::Size>>> {
        self.cols.borrow()
    }
}
