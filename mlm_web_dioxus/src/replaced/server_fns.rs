use dioxus::prelude::*;
#[cfg(feature = "server")]
use std::str::FromStr;

use super::types::*;

#[cfg(feature = "server")]
use crate::error::IntoServerFnError;
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
#[cfg(feature = "server")]
use mlm_core::{
    ContextExt, Torrent,
    linker::{refresh_mam_metadata, refresh_metadata_relink},
};
#[cfg(feature = "server")]
use mlm_db::{DatabaseExt as _, Language, TorrentKey, ids};

#[cfg(feature = "server")]
fn matches_filter(t: &Torrent, field: ReplacedPageFilter, value: &str) -> bool {
    match field {
        ReplacedPageFilter::Kind => t.meta.media_type.as_str() == value,
        ReplacedPageFilter::Title => t.meta.title == value,
        ReplacedPageFilter::Author => t.meta.authors.contains(&value.to_string()),
        ReplacedPageFilter::Narrator => t.meta.narrators.contains(&value.to_string()),
        ReplacedPageFilter::Series => t.meta.series.iter().any(|s| s.name == value),
        ReplacedPageFilter::Language => {
            if value.is_empty() {
                t.meta.language.is_none()
            } else {
                t.meta.language == Language::from_str(value).ok()
            }
        }
        ReplacedPageFilter::Filetype => t.meta.filetypes.iter().any(|f| f == value),
        ReplacedPageFilter::Linked => t.library_path.is_some() == (value == "true"),
    }
}

#[cfg(feature = "server")]
fn convert_row(t: &Torrent) -> ReplacedRow {
    ReplacedRow {
        id: t.id.clone(),
        mam_id: t.mam_id,
        meta: ReplacedMeta {
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
            language: t.meta.language.map(|l| l.to_str().to_string()),
            size: t.meta.size.to_string(),
            filetypes: t.meta.filetypes.clone(),
        },
        linked: t.library_path.is_some(),
        created_at: format_timestamp_db(&t.created_at),
        replaced_at: t
            .replaced_with
            .as_ref()
            .map(|(_, ts)| format_timestamp_db(ts)),
        abs_id: t.meta.ids.get(ids::ABS).cloned(),
    }
}

#[server]
pub async fn get_replaced_data(
    sort: Option<ReplacedPageSort>,
    asc: bool,
    filters: Vec<(ReplacedPageFilter, String)>,
    from: Option<usize>,
    page_size: Option<usize>,
    _show: ReplacedPageColumns,
) -> Result<ReplacedData, ServerFnError> {
    let context = crate::error::get_context()?;

    let mut from_val = from.unwrap_or(0);
    let page_size_val = page_size.unwrap_or(500);

    let r = context.db().r_transaction().server_err()?;

    let mut replaced = r
        .scan()
        .secondary::<Torrent>(TorrentKey::created_at)
        .server_err()?
        .all()
        .server_err()?
        .rev()
        .filter_map(Result::ok)
        .filter(|t| t.replaced_with.is_some())
        .filter(|t| {
            filters
                .iter()
                .all(|(field, value)| matches_filter(t, *field, value))
        })
        .collect::<Vec<_>>();

    if let Some(sort_by) = sort {
        replaced.sort_by(|a, b| {
            let ord = match sort_by {
                ReplacedPageSort::Kind => a.meta.media_type.cmp(&b.meta.media_type),
                ReplacedPageSort::Title => a.meta.title.cmp(&b.meta.title),
                ReplacedPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                ReplacedPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                ReplacedPageSort::Series => a.meta.series.cmp(&b.meta.series),
                ReplacedPageSort::Language => a.meta.language.cmp(&b.meta.language),
                ReplacedPageSort::Size => a.meta.size.cmp(&b.meta.size),
                ReplacedPageSort::Replaced => a
                    .replaced_with
                    .as_ref()
                    .map(|r| r.1)
                    .cmp(&b.replaced_with.as_ref().map(|r| r.1)),
                ReplacedPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if asc { ord.reverse() } else { ord }
        });
    }

    let total = replaced.len();
    if page_size_val > 0 && from_val >= total && total > 0 {
        from_val = ((total - 1) / page_size_val) * page_size_val;
    }

    let limit = if page_size_val == 0 {
        usize::MAX
    } else {
        page_size_val
    };

    let mut rows = Vec::new();
    for torrent in replaced.into_iter().skip(from_val).take(limit) {
        let Some((replacement_id, _)) = &torrent.replaced_with else {
            continue;
        };
        let Some(replacement) = r
            .get()
            .primary::<Torrent>(replacement_id.clone())
            .server_err()?
        else {
            continue;
        };
        rows.push(ReplacedPairRow {
            torrent: convert_row(&torrent),
            replacement: convert_row(&replacement),
        });
    }

    let abs_url = context
        .config()
        .await
        .audiobookshelf
        .as_ref()
        .map(|abs| abs.url.clone());

    Ok(ReplacedData {
        torrents: rows,
        total,
        from: from_val,
        page_size: page_size_val,
        abs_url,
    })
}

#[server]
pub async fn apply_replaced_action(
    action: ReplacedBulkAction,
    torrent_ids: Vec<String>,
) -> Result<(), ServerFnError> {
    if torrent_ids.is_empty() {
        return Err(ServerFnError::new("No torrents selected"));
    }

    let context = crate::error::get_context()?;

    match action {
        ReplacedBulkAction::Refresh => {
            let config = context.config().await;
            let mam = context.mam().server_err()?;
            for id in torrent_ids {
                refresh_mam_metadata(&config, context.db(), &mam, id, &context.events)
                    .await
                    .server_err()?;
            }
        }
        ReplacedBulkAction::RefreshRelink => {
            let config = context.config().await;
            let mam = context.mam().server_err()?;
            for id in torrent_ids {
                refresh_metadata_relink(&config, context.db(), &mam, id, &context.events)
                    .await
                    .server_err()?;
            }
        }
        ReplacedBulkAction::Remove => {
            let (_guard, rw) = context.db().rw_async().await.server_err()?;
            for id in torrent_ids {
                let Some(torrent) = rw.get().primary::<Torrent>(id).server_err()? else {
                    continue;
                };
                rw.remove(torrent).server_err()?;
            }
            rw.commit().server_err()?;
        }
    }

    Ok(())
}
