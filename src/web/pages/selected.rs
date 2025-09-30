use std::sync::Arc;

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
    data::{SelectedTorrent, Timestamp},
    mam::Unsats,
    web::{
        AppError, MaMState, Page, series,
        tables::{Key, SortOn, Sortable, item, items, table_styles},
        time,
    },
};

pub async fn selected_page(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    Query(sort): Query<SortOn<SelectedPageSort>>,
    Query(filter): Query<Vec<(SelectedPageFilter, String)>>,
) -> std::result::Result<Html<String>, AppError> {
    let mut torrents = db
        .r_transaction()?
        .scan()
        .primary::<SelectedTorrent>()?
        .all()?
        .filter(|t| t.as_ref().is_ok_and(|t| t.removed_at.is_none()))
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            for (field, value) in filter.iter() {
                let ok = match field {
                    SelectedPageFilter::Kind => t.meta.main_cat.as_str() == value,
                    SelectedPageFilter::Title => &t.meta.title == value,
                    SelectedPageFilter::Author => t.meta.authors.contains(value),
                    SelectedPageFilter::Narrator => t.meta.narrators.contains(value),
                    SelectedPageFilter::Series => t.meta.series.iter().any(|s| &s.name == value),
                    SelectedPageFilter::Filetype => t.meta.filetypes.contains(value),
                    SelectedPageFilter::Cost => t.cost.as_str() == value,
                    SelectedPageFilter::SortBy => true,
                    SelectedPageFilter::Asc => true,
                };
                if !ok {
                    return false;
                }
            }
            true
        })
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    if let Some(sort_by) = &sort.sort_by {
        torrents.sort_by(|a, b| {
            let ord = match sort_by {
                SelectedPageSort::Kind => a.meta.main_cat.cmp(&b.meta.main_cat),
                SelectedPageSort::Title => a.meta.title.cmp(&b.meta.title),
                SelectedPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                SelectedPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                SelectedPageSort::Series => a.meta.series.cmp(&b.meta.series),
                SelectedPageSort::Size => a.meta.size.cmp(&b.meta.size),
                SelectedPageSort::Cost => a.cost.cmp(&b.cost),
                SelectedPageSort::Buffer => a
                    .unsat_buffer
                    .unwrap_or(config.unsat_buffer)
                    .cmp(&b.unsat_buffer.unwrap_or(config.unsat_buffer)),
                SelectedPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if sort.asc { ord.reverse() } else { ord }
        });
    }
    let unsats = match mam.as_ref() {
        Ok(mam) => mam.user_info().await.map(|u| u.unsat).ok(),
        _ => None,
    };
    let template = SelectedPageTemplate {
        unsats,
        unsat_buffer: config.unsat_buffer,
        sort,
        torrents,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn selected_torrents_page_post(
    State(db): State<Arc<Database<'static>>>,
    uri: OriginalUri,
    Form(form): Form<TorrentsPageForm>,
) -> Result<Redirect, AppError> {
    match form.action.as_str() {
        "remove" => {
            for torrent in form.torrents {
                let rw = db.rw_transaction()?;
                let Some(mut torrent) = rw.get().primary::<SelectedTorrent>(torrent)? else {
                    return Err(anyhow::Error::msg("Could not find torrent").into());
                };
                torrent.removed_at = Some(Timestamp::now());
                rw.upsert(torrent)?;
                rw.commit()?;
            }
        }
        "update" => {
            for torrent in form.torrents {
                let rw = db.rw_transaction()?;
                let Some(mut torrent) = rw.get().primary::<SelectedTorrent>(torrent)? else {
                    return Err(anyhow::Error::msg("Could not find torrent").into());
                };
                torrent.unsat_buffer = Some(form.unsats.unwrap_or_default());
                rw.upsert(torrent)?;
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
    unsats: Option<u64>,
    #[serde(default, rename = "torrent")]
    torrents: Vec<u64>,
}

#[derive(Template)]
#[template(path = "pages/selected.html")]
struct SelectedPageTemplate {
    unsats: Option<Unsats>,
    unsat_buffer: u64,
    sort: SortOn<SelectedPageSort>,
    torrents: Vec<SelectedTorrent>,
}

impl Page for SelectedPageTemplate {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SelectedPageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Size,
    Cost,
    Buffer,
    CreatedAt,
}

impl Key for SelectedPageSort {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectedPageFilter {
    Kind,
    Title,
    Author,
    Narrator,
    Series,
    Filetype,
    Cost,
    // Workaround sort decode failure
    SortBy,
    Asc,
}

impl Key for SelectedPageFilter {}

impl Sortable for SelectedPageTemplate {
    type SortKey = SelectedPageSort;

    fn get_current_sort(&self) -> SortOn<Self::SortKey> {
        self.sort
    }
}
