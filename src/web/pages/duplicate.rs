use std::sync::Arc;

use anyhow::{Context as _, Error, Result};
use askama::Template;
use axum::{
    extract::{OriginalUri, Query, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use native_db::Database;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{
    cleaner::clean_torrent,
    config::Config,
    data::{DuplicateTorrent, SelectedTorrent, Timestamp, Torrent, TorrentCost},
    mam::{MaM, SearchQuery, Tor, normalize_title},
    mam_enums::SearchIn,
    web::{
        AppError, series,
        tables::{Key, SortOn, Sortable, item, items, table_styles_rows},
        time,
    },
};

pub async fn duplicate_page(
    State(db): State<Arc<Database<'static>>>,
    Query(sort): Query<SortOn<DuplicatePageSort>>,
    Query(filter): Query<Vec<(DuplicatePageFilter, String)>>,
) -> std::result::Result<Html<String>, AppError> {
    let mut duplicate_torrents = db
        .r_transaction()?
        .scan()
        .primary::<DuplicateTorrent>()?
        .all()?
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            for (field, value) in filter.iter() {
                let ok = match field {
                    DuplicatePageFilter::Kind => t.meta.main_cat.as_str() == value,
                    DuplicatePageFilter::Title => &t.meta.title == value,
                    DuplicatePageFilter::Author => t.meta.authors.contains(value),
                    DuplicatePageFilter::Narrator => t.meta.narrators.contains(value),
                    DuplicatePageFilter::Series => {
                        t.meta.series.iter().any(|(name, _)| name == value)
                    }
                    DuplicatePageFilter::Filetype => t.meta.filetypes.contains(value),
                    DuplicatePageFilter::SortBy => true,
                    DuplicatePageFilter::Asc => true,
                };
                if !ok {
                    return false;
                }
            }
            true
        })
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    if let Some(sort_by) = &sort.sort_by {
        duplicate_torrents.sort_by(|a, b| {
            let ord = match sort_by {
                DuplicatePageSort::Kind => a.meta.main_cat.cmp(&b.meta.main_cat),
                DuplicatePageSort::Title => a.meta.title.cmp(&b.meta.title),
                DuplicatePageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                DuplicatePageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                DuplicatePageSort::Series => a.meta.series.cmp(&b.meta.series),
                DuplicatePageSort::Size => a.meta.size.cmp(&b.meta.size),
                DuplicatePageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if sort.asc { ord.reverse() } else { ord }
        });
    }
    let mut torrents = vec![];
    for torrent in duplicate_torrents {
        let Some(with) = &torrent.duplicate_of else {
            continue;
        };
        let Some(duplicate) = db.r_transaction()?.get().primary(with.clone())? else {
            continue;
        };
        torrents.push((torrent, duplicate));
    }
    let template = DuplicatePageTemplate { sort, torrents };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn duplicate_torrents_page_post(
    State((config, db, mam)): State<(
        Arc<Config>,
        Arc<Database<'static>>,
        Arc<Result<Arc<MaM<'static>>>>,
    )>,
    uri: OriginalUri,
    Form(form): Form<TorrentsPageForm>,
) -> Result<Redirect, AppError> {
    match form.action.as_str() {
        "replace" => {
            let Ok(mam) = mam.as_ref() else {
                return Err(anyhow::Error::msg("mam_id error").into());
            };
            for torrent in form.torrents {
                let r = db.r_transaction()?;
                let Some(duplicate_torrent) = r.get().primary::<DuplicateTorrent>(torrent)? else {
                    return Err(anyhow::Error::msg("Could not find torrent").into());
                };
                let Some(hash) = duplicate_torrent.duplicate_of.clone() else {
                    return Err(anyhow::Error::msg("No duplicate_of set").into());
                };
                let Some(duplicate_of) = r.get().primary::<Torrent>(hash)? else {
                    return Err(anyhow::Error::msg("Could not find original torrent").into());
                };

                let page_results = mam
                    .search(&SearchQuery {
                        description: false,
                        dl_link: true,
                        isbn: false,
                        perpage: 100,
                        tor: Tor {
                            start_number: 0,
                            text: &duplicate_torrent.meta.authors.join("|"),
                            srch_in: vec![SearchIn::Author],
                            browse_lang: duplicate_torrent
                                .meta
                                .language
                                .map(|l| l.to_id())
                                .into_iter()
                                .collect(),
                            main_cat: vec![duplicate_torrent.meta.main_cat.as_id()],
                            min_size: duplicate_torrent.meta.size.bytes()
                                - (duplicate_torrent.meta.size.bytes() as f64 * 0.1) as u64,
                            max_size: duplicate_torrent.meta.size.bytes()
                                + (duplicate_torrent.meta.size.bytes() as f64 * 0.1) as u64,
                            unit: 1,
                            ..Default::default()
                        },
                    })
                    .await
                    .context("search")?;
                debug!(
                    "result: perpage: {}, start: {}, data: {}, total: {}, found: {}",
                    page_results.perpage,
                    page_results.start,
                    page_results.data.len(),
                    page_results.total,
                    page_results.found
                );

                let Some(mam_torrent) = page_results
                    .data
                    .into_iter()
                    .find(|t| t.id == duplicate_torrent.mam_id)
                else {
                    return Err(
                        anyhow::Error::msg("Could not find duplicate torrent on MaM").into(),
                    );
                };

                let title_search = normalize_title(&mam_torrent.title);
                let tags: Vec<_> = config
                    .tags
                    .iter()
                    .filter(|t| t.filter.matches(&mam_torrent))
                    .collect();
                let category = tags.iter().find_map(|t| t.category.clone());
                let tags = tags.iter().flat_map(|t| t.tags.clone()).collect();
                let cost = if mam_torrent.vip > 0 {
                    TorrentCost::Vip
                } else if mam_torrent.personal_freeleech > 0 {
                    TorrentCost::PersonalFreeleech
                } else if mam_torrent.free > 0 {
                    TorrentCost::GlobalFreeleech
                // TODO: Allow select
                // } else if cost == Cost::Wedge {
                //     TorrentCost::UseWedge
                // } else if cost == Cost::TryWedge {
                //     TorrentCost::TryWedge
                // } else {
                //     TorrentCost::Ratio
                } else {
                    TorrentCost::TryWedge
                };
                info!(
                    "Selecting torrent \"{}\" in format {}, cost: {:?}, with category {:?} and tags {:?}",
                    mam_torrent.title, mam_torrent.filetype, cost, category, tags
                );

                let rw = db.rw_transaction()?;
                rw.remove(duplicate_torrent)?;
                rw.insert(SelectedTorrent {
                    mam_id: mam_torrent.id,
                    dl_link: mam_torrent.dl.clone().ok_or_else(|| {
                        Error::msg(format!("no dl field for torrent {}", mam_torrent.id))
                    })?,
                    unsat_buffer: None,
                    cost,
                    category,
                    tags,
                    title_search,
                    meta: mam_torrent.as_meta()?,
                    created_at: Timestamp::now(),
                    removed_at: None,
                })?;
                rw.commit()?;
                clean_torrent(&config, &db, duplicate_of).await?;
            }
        }
        "remove" => {
            for torrent in form.torrents {
                let rw = db.rw_transaction()?;
                let Some(torrent) = rw.get().primary::<DuplicateTorrent>(torrent)? else {
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
    torrents: Vec<u64>,
}

#[derive(Template)]
#[template(path = "pages/duplicate.html")]
struct DuplicatePageTemplate {
    sort: SortOn<DuplicatePageSort>,
    torrents: Vec<(DuplicateTorrent, Torrent)>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DuplicatePageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Size,
    CreatedAt,
}

impl Key for DuplicatePageSort {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicatePageFilter {
    Kind,
    Title,
    Author,
    Narrator,
    Series,
    Filetype,
    // Workaround sort decode failure
    SortBy,
    Asc,
}

impl Key for DuplicatePageFilter {}

impl Sortable for DuplicatePageTemplate {
    type SortKey = DuplicatePageSort;

    fn get_current_sort(&self) -> SortOn<Self::SortKey> {
        self.sort
    }
}
