use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
};
use native_db::Database;
use serde::{Deserialize, Serialize};

use crate::{
    data::{ErroredTorrent, ErroredTorrentId, ErroredTorrentKey},
    web::{
        AppError,
        tables::{Key, SortOn, Sortable, item, table_styles},
        time,
    },
};

pub async fn errors_page(
    State(db): State<Arc<Database<'static>>>,
    Query(sort): Query<SortOn<ErrorsPageSort>>,
    Query(filter): Query<Vec<(ErrorsPageFilter, String)>>,
) -> std::result::Result<Html<String>, AppError> {
    let mut errored_torrents = db
        .r_transaction()?
        .scan()
        .secondary::<ErroredTorrent>(ErroredTorrentKey::created_at)?
        .all()?
        .rev()
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            for (field, value) in filter.iter() {
                let ok = match field {
                    ErrorsPageFilter::Step => t.id.step() == value,
                    ErrorsPageFilter::Title => &t.title == value,
                    ErrorsPageFilter::SortBy => true,
                    ErrorsPageFilter::Asc => true,
                };
                if !ok {
                    return false;
                }
            }
            true
        })
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    if let Some(sort_by) = &sort.sort_by {
        errored_torrents.sort_by(|a, b| {
            let ord = match sort_by {
                ErrorsPageSort::Step => a.id.step().cmp(b.id.step()),
                ErrorsPageSort::Title => a.title.cmp(&b.title),
                ErrorsPageSort::Error => a.error.cmp(&b.error),
                ErrorsPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if sort.asc { ord.reverse() } else { ord }
        });
    }
    let template = ErrorsPageTemplate {
        sort,
        errors: errored_torrents,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

#[derive(Template)]
#[template(path = "pages/errors.html")]
struct ErrorsPageTemplate {
    sort: SortOn<ErrorsPageSort>,
    errors: Vec<ErroredTorrent>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorsPageSort {
    Step,
    Title,
    Error,
    CreatedAt,
}

impl Key for ErrorsPageSort {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorsPageFilter {
    Step,
    Title,
    // Workaround sort decode failure
    SortBy,
    Asc,
}

impl Key for ErrorsPageFilter {}

impl Sortable for ErrorsPageTemplate {
    type SortKey = ErrorsPageSort;

    fn get_current_sort(&self) -> SortOn<Self::SortKey> {
        self.sort
    }
}

impl ErroredTorrentId {
    pub fn step(&self) -> &str {
        match self {
            ErroredTorrentId::Grabber(_) => "auto grabber",
            ErroredTorrentId::Linker(_) => "library linker",
            ErroredTorrentId::Cleaner(_) => "library cleaner",
        }
    }
}
