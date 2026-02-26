use dioxus::prelude::*;

use super::types::*;

#[cfg(feature = "server")]
use crate::error::{IntoServerFnError, OptionIntoServerFnError};
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
#[cfg(feature = "server")]
use mlm_core::{ContextExt, Torrent, cleaner::clean_torrent};
#[cfg(feature = "server")]
use mlm_db::{DatabaseExt as _, DuplicateTorrent, SelectedTorrent, Timestamp, TorrentCost, ids};
#[cfg(feature = "server")]
use mlm_parse::normalize_title;

#[cfg(feature = "server")]
fn matches_filter(t: &DuplicateTorrent, field: DuplicatePageFilter, value: &str) -> bool {
    match field {
        DuplicatePageFilter::Kind => t.meta.media_type.as_str() == value,
        DuplicatePageFilter::Title => t.meta.title == value,
        DuplicatePageFilter::Author => t.meta.authors.contains(&value.to_string()),
        DuplicatePageFilter::Narrator => t.meta.narrators.contains(&value.to_string()),
        DuplicatePageFilter::Series => t.meta.series.iter().any(|s| s.name == value),
        DuplicatePageFilter::Filetype => t.meta.filetypes.iter().any(|f| f == value),
    }
}

#[cfg(feature = "server")]
fn convert_candidate_row(t: &DuplicateTorrent) -> DuplicateCandidateRow {
    DuplicateCandidateRow {
        mam_id: t.mam_id,
        meta: DuplicateMeta {
            title: t.meta.title.clone(),
            media_type: t.meta.media_type.as_str().to_string(),
            authors: t.meta.authors.clone(),
            narrators: t.meta.narrators.clone(),
            series: t
                .meta
                .series
                .iter()
                .map(|series| crate::dto::Series {
                    name: series.name.clone(),
                    entries: series.entries.to_string(),
                })
                .collect(),
            size: t.meta.size.to_string(),
            filetypes: t.meta.filetypes.clone(),
        },
        created_at: format_timestamp_db(&t.created_at),
    }
}

#[cfg(feature = "server")]
fn convert_original_row(t: &Torrent) -> DuplicateOriginalRow {
    DuplicateOriginalRow {
        id: t.id.clone(),
        mam_id: t.mam_id,
        meta: DuplicateMeta {
            title: t.meta.title.clone(),
            media_type: t.meta.media_type.as_str().to_string(),
            authors: t.meta.authors.clone(),
            narrators: t.meta.narrators.clone(),
            series: t
                .meta
                .series
                .iter()
                .map(|series| crate::dto::Series {
                    name: series.name.clone(),
                    entries: series.entries.to_string(),
                })
                .collect(),
            size: t.meta.size.to_string(),
            filetypes: t.meta.filetypes.clone(),
        },
        linked: t.library_path.is_some(),
        linked_path: t
            .library_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        created_at: format_timestamp_db(&t.created_at),
        abs_id: t.meta.ids.get(ids::ABS).cloned(),
    }
}

#[server]
pub async fn get_duplicate_data(
    sort: Option<DuplicatePageSort>,
    asc: bool,
    filters: Vec<(DuplicatePageFilter, String)>,
    from: Option<usize>,
    page_size: Option<usize>,
) -> Result<DuplicateData, ServerFnError> {
    let context = crate::error::get_context()?;

    let mut from_val = from.unwrap_or(0);
    let page_size_val = page_size.unwrap_or(500);

    let r = context.db().r_transaction().server_err()?;

    let mut duplicates = r
        .scan()
        .primary::<DuplicateTorrent>()
        .server_err()?
        .all()
        .server_err()?
        .filter_map(Result::ok)
        .filter(|t| {
            filters
                .iter()
                .all(|(field, value)| matches_filter(t, *field, value))
        })
        .collect::<Vec<_>>();

    if let Some(sort_by) = sort {
        duplicates.sort_by(|a, b| {
            let ord = match sort_by {
                DuplicatePageSort::Kind => a.meta.media_type.cmp(&b.meta.media_type),
                DuplicatePageSort::Title => a.meta.title.cmp(&b.meta.title),
                DuplicatePageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                DuplicatePageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                DuplicatePageSort::Series => a.meta.series.cmp(&b.meta.series),
                DuplicatePageSort::Size => a.meta.size.cmp(&b.meta.size),
                DuplicatePageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if asc { ord.reverse() } else { ord }
        });
    }

    let total = duplicates.len();
    if page_size_val > 0 && from_val >= total && total > 0 {
        from_val = ((total - 1) / page_size_val) * page_size_val;
    }

    let limit = if page_size_val == 0 {
        usize::MAX
    } else {
        page_size_val
    };

    let mut rows = Vec::new();
    for duplicate in duplicates.into_iter().skip(from_val).take(limit) {
        let Some(duplicate_of_id) = &duplicate.duplicate_of else {
            continue;
        };
        let Some(duplicate_of) = r
            .get()
            .primary::<Torrent>(duplicate_of_id.clone())
            .server_err()?
        else {
            continue;
        };
        rows.push(DuplicatePairRow {
            torrent: convert_candidate_row(&duplicate),
            duplicate_of: convert_original_row(&duplicate_of),
        });
    }

    let abs_url = context
        .config()
        .await
        .audiobookshelf
        .as_ref()
        .map(|abs| abs.url.clone());

    Ok(DuplicateData {
        torrents: rows,
        total,
        from: from_val,
        page_size: page_size_val,
        abs_url,
    })
}

#[server]
pub async fn apply_duplicate_action(
    action: DuplicateBulkAction,
    torrent_ids: Vec<u64>,
) -> Result<(), ServerFnError> {
    if torrent_ids.is_empty() {
        return Err(ServerFnError::new("No torrents selected"));
    }

    let context = crate::error::get_context()?;
    let config = context.config().await;

    match action {
        DuplicateBulkAction::Replace => {
            let mam = context.mam().server_err()?;
            for mam_id in torrent_ids {
                let r = context.db().r_transaction().server_err()?;
                let Some(duplicate_torrent) =
                    r.get().primary::<DuplicateTorrent>(mam_id).server_err()?
                else {
                    continue;
                };
                let Some(hash) = duplicate_torrent.duplicate_of.clone() else {
                    return Err(ServerFnError::new("No duplicate_of set"));
                };
                let Some(duplicate_of) = r.get().primary::<Torrent>(hash).server_err()? else {
                    return Err(ServerFnError::new("Could not find original torrent"));
                };

                let Some(mam_torrent) = mam
                    .get_torrent_info_by_id(duplicate_torrent.mam_id)
                    .await
                    .server_err()?
                else {
                    return Err(ServerFnError::new(
                        "Could not find duplicate torrent on MaM",
                    ));
                };

                let meta = mam_torrent.as_meta().server_err()?;
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
                } else {
                    TorrentCost::TryWedge
                };

                let (_guard, rw) = context.db().rw_async().await.server_err()?;
                rw.insert(SelectedTorrent {
                    mam_id: mam_torrent.id,
                    hash: None,
                    dl_link: mam_torrent
                        .dl
                        .clone()
                        .or_else(|| duplicate_torrent.dl_link.clone())
                        .ok_or_server_err("No download link for duplicate torrent")?,
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
                })
                .server_err()?;
                rw.remove(duplicate_torrent).server_err()?;
                rw.commit().server_err()?;

                clean_torrent(&config, context.db(), duplicate_of, false, &context.events)
                    .await
                    .server_err()?;
            }
        }
        DuplicateBulkAction::Remove => {
            let (_guard, rw) = context.db().rw_async().await.server_err()?;
            for mam_id in torrent_ids {
                let Some(torrent) = rw.get().primary::<DuplicateTorrent>(mam_id).server_err()?
                else {
                    continue;
                };
                rw.remove(torrent).server_err()?;
            }
            rw.commit().server_err()?;
        }
    }

    Ok(())
}
