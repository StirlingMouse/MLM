use std::sync::Arc;

use anyhow::Result;
use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
};
use native_db::Database;
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    data::SelectedTorrent,
    mam::{MaM, Unsats},
    web::{
        AppError, series,
        tables::{Key, SortOn, Sortable, item, items, table_styles},
        time,
    },
};

pub async fn selected_page(
    State((config, db, mam)): State<(
        Arc<Config>,
        Arc<Database<'static>>,
        Arc<Result<Arc<MaM<'static>>>>,
    )>,
    Query(sort): Query<SortOn<SelectedPageSort>>,
    Query(filter): Query<Vec<(SelectedPageFilter, String)>>,
) -> std::result::Result<Html<String>, AppError> {
    let mut torrents = db
        .r_transaction()?
        .scan()
        .primary::<SelectedTorrent>()?
        .all()?
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
                    SelectedPageFilter::Series => {
                        t.meta.series.iter().any(|(name, _)| name == value)
                    }
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

#[derive(Template)]
#[template(path = "pages/selected.html")]
struct SelectedPageTemplate {
    unsats: Option<Unsats>,
    unsat_buffer: u64,
    sort: SortOn<SelectedPageSort>,
    torrents: Vec<SelectedTorrent>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SelectedPageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
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
