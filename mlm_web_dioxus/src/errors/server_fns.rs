use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::error::IntoServerFnError;
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
#[cfg(feature = "server")]
use mlm_core::ContextExt;
#[cfg(feature = "server")]
use mlm_db::{DatabaseExt as _, ErroredTorrent, ErroredTorrentId, ErroredTorrentKey, ids};

use super::types::*;

#[server]
pub async fn get_errors_data(
    sort: Option<ErrorsPageSort>,
    asc: bool,
    filters: Vec<(ErrorsPageFilter, String)>,
) -> Result<ErrorsData, ServerFnError> {
    let context = crate::error::get_context()?;

    let mut errors = context
        .db()
        .r_transaction()
        .server_err()?
        .scan()
        .secondary::<ErroredTorrent>(ErroredTorrentKey::created_at)
        .server_err()?
        .all()
        .server_err()?
        .rev()
        .filter_map(Result::ok)
        .filter(|t| {
            filters.iter().all(|(field, value)| match field {
                ErrorsPageFilter::Step => error_step(&t.id) == value,
                ErrorsPageFilter::Title => t.title == *value,
            })
        })
        .collect::<Vec<_>>();

    if let Some(sort_by) = sort {
        errors.sort_by(|a, b| {
            let ord = match sort_by {
                ErrorsPageSort::Step => error_step(&a.id).cmp(error_step(&b.id)),
                ErrorsPageSort::Title => a.title.cmp(&b.title),
                ErrorsPageSort::Error => a.error.cmp(&b.error),
                ErrorsPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if asc { ord.reverse() } else { ord }
        });
    }

    Ok(ErrorsData {
        errors: errors.into_iter().map(convert_error_row).collect(),
    })
}

#[server]
pub async fn remove_errors_action(error_ids: Vec<String>) -> Result<(), ServerFnError> {
    if error_ids.is_empty() {
        return Err(ServerFnError::new("No errors selected"));
    }

    let context = crate::error::get_context()?;

    let (_guard, rw) = context.db().rw_async().await.server_err()?;

    for error_id in error_ids {
        let id = serde_json::from_str::<ErroredTorrentId>(&error_id).server_err()?;
        let Some(error) = rw.get().primary::<ErroredTorrent>(id).server_err()? else {
            continue;
        };
        rw.remove(error).server_err()?;
    }

    rw.commit().server_err()?;
    Ok(())
}

#[cfg(feature = "server")]
fn error_step(id: &ErroredTorrentId) -> &'static str {
    match id {
        ErroredTorrentId::Grabber(_) => "auto grabber",
        ErroredTorrentId::Linker(_) => "library linker",
        ErroredTorrentId::Cleaner(_) => "library cleaner",
    }
}

#[cfg(feature = "server")]
fn convert_error_row(error: ErroredTorrent) -> ErrorsRow {
    ErrorsRow {
        id_json: serde_json::to_string(&error.id).unwrap_or_default(),
        step: error_step(&error.id).to_string(),
        title: error.title,
        error: error.error,
        created_at: format_timestamp_db(&error.created_at),
        mam_id: error
            .meta
            .and_then(|meta| meta.ids.get(ids::MAM).cloned())
            .and_then(|id| id.parse::<u64>().ok()),
    }
}
