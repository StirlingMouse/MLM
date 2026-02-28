use dioxus::prelude::*;
#[cfg(feature = "server")]
use std::str::FromStr;

#[cfg(feature = "server")]
use crate::error::IntoServerFnError;
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
#[cfg(feature = "server")]
use mlm_core::ContextExt;
#[cfg(feature = "server")]
use mlm_db::{DatabaseExt as _, Flags, Language, OldCategory, SelectedTorrent, Timestamp};

use super::types::{
    SelectedBulkAction, SelectedData, SelectedMeta, SelectedPageColumns, SelectedPageFilter,
    SelectedPageSort, SelectedRow, SelectedUserInfo,
};

#[server]
pub async fn get_selected_data(
    sort: Option<SelectedPageSort>,
    asc: bool,
    filters: Vec<(SelectedPageFilter, String)>,
    show: SelectedPageColumns,
) -> Result<SelectedData, ServerFnError> {
    let context = crate::error::get_context()?;
    let config = context.config().await;

    let mut torrents = context
        .db()
        .r_transaction()
        .server_err()?
        .scan()
        .primary::<SelectedTorrent>()
        .server_err()?
        .all()
        .server_err()?
        .filter_map(Result::ok)
        .filter(|t| show.removed_at || t.removed_at.is_none())
        .filter(|t| {
            filters.iter().all(|(field, value)| match field {
                SelectedPageFilter::Kind => t.meta.media_type.as_str() == value,
                SelectedPageFilter::Category => {
                    if value.is_empty() {
                        t.meta.cat.is_none()
                    } else if let Some(cat) = &t.meta.cat {
                        let cats = value
                            .split(',')
                            .filter_map(|id| id.parse().ok())
                            .filter_map(OldCategory::from_one_id)
                            .collect::<Vec<_>>();
                        cats.contains(cat) || cat.as_str() == value
                    } else {
                        false
                    }
                }
                SelectedPageFilter::Flags => {
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
                SelectedPageFilter::Title => t.meta.title == *value,
                SelectedPageFilter::Author => t.meta.authors.contains(value),
                SelectedPageFilter::Narrator => t.meta.narrators.contains(value),
                SelectedPageFilter::Series => t.meta.series.iter().any(|s| &s.name == value),
                SelectedPageFilter::Language => {
                    if value.is_empty() {
                        t.meta.language.is_none()
                    } else {
                        t.meta.language == Language::from_str(value).ok()
                    }
                }
                SelectedPageFilter::Filetype => t.meta.filetypes.contains(value),
                SelectedPageFilter::Cost => t.cost.as_str() == value,
                SelectedPageFilter::Grabber => {
                    if value.is_empty() {
                        t.grabber.is_none()
                    } else {
                        t.grabber.as_deref() == Some(value)
                    }
                }
            })
        })
        .collect::<Vec<_>>();

    if let Some(sort_by) = sort {
        torrents.sort_by(|a, b| {
            let ord = match sort_by {
                SelectedPageSort::Kind => a.meta.media_type.cmp(&b.meta.media_type),
                SelectedPageSort::Title => a.meta.title.cmp(&b.meta.title),
                SelectedPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                SelectedPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                SelectedPageSort::Series => a.meta.series.cmp(&b.meta.series),
                SelectedPageSort::Language => a.meta.language.cmp(&b.meta.language),
                SelectedPageSort::Size => a.meta.size.cmp(&b.meta.size),
                SelectedPageSort::Cost => a.cost.cmp(&b.cost),
                SelectedPageSort::Buffer => a
                    .unsat_buffer
                    .unwrap_or(config.unsat_buffer)
                    .cmp(&b.unsat_buffer.unwrap_or(config.unsat_buffer)),
                SelectedPageSort::Grabber => a.grabber.cmp(&b.grabber),
                SelectedPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
                SelectedPageSort::StartedAt => a.started_at.cmp(&b.started_at),
            };
            if asc { ord.reverse() } else { ord }
        });
    }

    let queued = torrents.iter().filter(|t| t.started_at.is_none()).count();
    let downloading = torrents.iter().filter(|t| t.started_at.is_some()).count();

    Ok(SelectedData {
        torrents: torrents
            .into_iter()
            .map(|t| convert_selected_row(&t, config.unsat_buffer))
            .collect(),
        queued,
        downloading,
    })
}

#[server]
pub async fn get_selected_user_info() -> Result<Option<SelectedUserInfo>, ServerFnError> {
    let context = crate::error::get_context()?;
    let config = context.config().await;

    let downloading_size: f64 = context
        .db()
        .r_transaction()
        .server_err()?
        .scan()
        .primary::<SelectedTorrent>()
        .server_err()?
        .all()
        .server_err()?
        .filter_map(Result::ok)
        .filter(|t| t.removed_at.is_none() && t.started_at.is_some())
        .map(|t| t.meta.size.bytes() as f64)
        .sum();

    let user_info = match context.mam() {
        Ok(mam) => mam.user_info().await.ok().map(|user_info| {
            let remaining_buffer = mlm_db::Size::from_bytes(
                ((user_info.uploaded_bytes - user_info.downloaded_bytes - downloading_size)
                    / config.min_ratio) as u64,
            )
            .to_string();
            SelectedUserInfo {
                unsat_count: user_info.unsat.count,
                unsat_limit: user_info.unsat.limit,
                wedges: user_info.wedges,
                bonus: user_info.seedbonus,
                remaining_buffer: Some(remaining_buffer),
            }
        }),
        Err(_) => None,
    };

    Ok(user_info)
}

#[server]
pub async fn apply_selected_action(
    action: SelectedBulkAction,
    mam_ids: Vec<u64>,
    unsats: Option<u64>,
) -> Result<(), ServerFnError> {
    if mam_ids.is_empty() {
        return Err(ServerFnError::new("No torrents selected"));
    }

    let context = crate::error::get_context()?;

    match action {
        SelectedBulkAction::Remove => {
            let (_guard, rw) = context.db().rw_async().await.server_err()?;
            for mam_id in mam_ids {
                let Some(mut torrent) = rw.get().primary::<SelectedTorrent>(mam_id).server_err()?
                else {
                    continue;
                };
                if torrent.removed_at.is_none() {
                    torrent.removed_at = Some(Timestamp::now());
                    rw.upsert(torrent).server_err()?;
                } else {
                    rw.remove(torrent).server_err()?;
                }
            }
            rw.commit().server_err()?;
        }
        SelectedBulkAction::Update => {
            let (_guard, rw) = context.db().rw_async().await.server_err()?;
            for mam_id in mam_ids {
                let Some(mut torrent) = rw.get().primary::<SelectedTorrent>(mam_id).server_err()?
                else {
                    continue;
                };
                torrent.unsat_buffer = Some(unsats.unwrap_or_default());
                torrent.removed_at = None;
                rw.upsert(torrent).server_err()?;
            }
            rw.commit().server_err()?;
        }
    }

    Ok(())
}

#[cfg(feature = "server")]
fn convert_selected_row(t: &SelectedTorrent, default_unsat: u64) -> SelectedRow {
    let flags = Flags::from_bitfield(t.meta.flags.map_or(0, |f| f.0));
    let mut flag_values = Vec::new();
    if flags.crude_language == Some(true) {
        flag_values.push("language".to_string());
    }
    if flags.violence == Some(true) {
        flag_values.push("violence".to_string());
    }
    if flags.some_explicit == Some(true) {
        flag_values.push("some_explicit".to_string());
    }
    if flags.explicit == Some(true) {
        flag_values.push("explicit".to_string());
    }
    if flags.abridged == Some(true) {
        flag_values.push("abridged".to_string());
    }
    if flags.lgbt == Some(true) {
        flag_values.push("lgbt".to_string());
    }

    let (cat_name, cat_id) = if let Some(cat) = &t.meta.cat {
        (cat.as_str().to_string(), Some(cat.as_id().to_string()))
    } else {
        ("N/A".to_string(), None)
    };

    SelectedRow {
        mam_id: t.mam_id,
        meta: SelectedMeta {
            title: t.meta.title.clone(),
            media_type: t.meta.media_type.as_str().to_string(),
            cat_name,
            cat_id,
            flags: flag_values,
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
        cost: t.cost.as_str().to_string(),
        required_unsats: t.unsat_buffer.unwrap_or(default_unsat),
        grabber: t.grabber.clone(),
        created_at: format_timestamp_db(&t.created_at),
        started_at: t.started_at.as_ref().map(format_timestamp_db),
        removed_at: t.removed_at.as_ref().map(format_timestamp_db),
    }
}
