use anyhow::{Error, Result};
use askama::Template;
use axum::{
    extract::{OriginalUri, Query, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use mlm_db::{
    DatabaseExt as _, DuplicateTorrent, SelectedTorrent, Timestamp, Torrent, TorrentCost,
};
use mlm_parse::normalize_title;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    cleaner::clean_torrent,
    stats::Context,
    web::{
        AppError, Page,
        tables::{Key, SortOn, Sortable, table_styles_rows},
        time,
    },
};

pub async fn duplicate_page(
    State(context): State<Context>,
    Query(sort): Query<SortOn<DuplicatePageSort>>,
    Query(filter): Query<Vec<(DuplicatePageFilter, String)>>,
) -> std::result::Result<Html<String>, AppError> {
    let config = context.config().await;
    let mut duplicate_torrents = context
        .db
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
                    DuplicatePageFilter::Kind => t.meta.media_type.as_str() == value,
                    DuplicatePageFilter::Title => &t.meta.title == value,
                    DuplicatePageFilter::Author => t.meta.authors.contains(value),
                    DuplicatePageFilter::Narrator => t.meta.narrators.contains(value),
                    DuplicatePageFilter::Series => t.meta.series.iter().any(|s| &s.name == value),
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
                DuplicatePageSort::Kind => a.meta.media_type.cmp(&b.meta.media_type),
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
        let Some(duplicate) = context.db.r_transaction()?.get().primary(with.clone())? else {
            continue;
        };
        torrents.push((torrent, duplicate));
    }
    let template = DuplicatePageTemplate {
        abs_url: config.audiobookshelf.as_ref().map(|abs| abs.url.clone()),
        sort,
        torrents,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn duplicate_torrents_page_post(
    State(context): State<Context>,
    uri: OriginalUri,
    Form(form): Form<TorrentsPageForm>,
) -> Result<Redirect, AppError> {
    let config = context.config().await;
    match form.action.as_str() {
        "replace" => {
            let mam = context.mam()?;
            for torrent in form.torrents {
                let r = context.db.r_transaction()?;
                let Some(duplicate_torrent) = r.get().primary::<DuplicateTorrent>(torrent)? else {
                    return Err(anyhow::Error::msg("Could not find torrent").into());
                };
                let Some(hash) = duplicate_torrent.duplicate_of.clone() else {
                    return Err(anyhow::Error::msg("No duplicate_of set").into());
                };
                let Some(duplicate_of) = r.get().primary::<Torrent>(hash)? else {
                    return Err(anyhow::Error::msg("Could not find original torrent").into());
                };

                let Some(mam_torrent) = mam
                    .get_torrent_info_by_id(duplicate_torrent.mam_id)
                    .await
                    .ok()
                    .flatten()
                else {
                    return Err(
                        anyhow::Error::msg("Could not find duplicate torrent on MaM").into(),
                    );
                };

                let meta = mam_torrent.as_meta()?;
                let title_search = normalize_title(&meta.title);
                let tags: Vec<_> = config
                    .tags
                    .iter()
                    .filter(|t| t.filter.matches(&mam_torrent))
                    .collect();
                let category = tags.iter().find_map(|t| t.category.clone());
                let tags = tags.iter().flat_map(|t| t.tags.clone()).collect();
                let cost = if mam_torrent.vip {
                    TorrentCost::Vip
                } else if mam_torrent.personal_freeleech {
                    TorrentCost::PersonalFreeleech
                } else if mam_torrent.free {
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

                {
                    let (_guard, rw) = context.db.rw_async().await?;
                    rw.insert(SelectedTorrent {
                        mam_id: mam_torrent.id,
                        goodreads_id: None,
                        hash: None,
                        dl_link: mam_torrent
                            .dl
                            .clone()
                            .or_else(|| duplicate_torrent.dl_link.clone())
                            .ok_or_else(|| {
                                Error::msg(format!("no dl field for torrent {}", mam_torrent.id))
                            })?,
                        unsat_buffer: None,
                        wedge_buffer: None,
                        cost,
                        category,
                        tags,
                        title_search,
                        meta,
                        grabber: None,
                        created_at: Timestamp::now(),
                        started_at: None,
                        removed_at: None,
                    })?;
                    rw.remove(duplicate_torrent)?;
                    rw.commit()?;
                }
                clean_torrent(&config, &context.db, duplicate_of, false).await?;
            }
        }
        "remove" => {
            for torrent in form.torrents {
                let (_guard, rw) = context.db.rw_async().await?;
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
    abs_url: Option<String>,
    sort: SortOn<DuplicatePageSort>,
    torrents: Vec<(DuplicateTorrent, Torrent)>,
}

impl Page for DuplicatePageTemplate {}

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
