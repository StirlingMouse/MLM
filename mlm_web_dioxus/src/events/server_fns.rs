#![allow(clippy::too_many_arguments)]

use super::types::EventData;
#[cfg(feature = "server")]
use crate::dto::{Event, EventType, MetadataSource, TorrentMetaDiff, convert_torrent};
#[cfg(feature = "server")]
use crate::error::IntoServerFnError;
#[cfg(feature = "server")]
use crate::utils::format_timestamp;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use mlm_core::ContextExt;
#[cfg(feature = "server")]
use mlm_core::{Event as DbEvent, EventKey, EventType as DbEventType, TorrentKey};

#[cfg(feature = "server")]
use super::types::EventWithTorrentData;

#[server]
pub async fn get_events_data(
    show: Option<String>,
    grabber: Option<String>,
    linker: Option<String>,
    category: Option<String>,
    has_updates: Option<String>,
    field: Option<String>,
    from: Option<usize>,
    page_size: Option<usize>,
) -> Result<EventData, ServerFnError> {
    let context = crate::error::get_context()?;
    let db = context.db();

    let from_val = from.unwrap_or(0);
    let page_size_val = page_size.unwrap_or(500);

    let r = db.r_transaction().server_err_ctx("r_transaction")?;

    let convert_event = |db_event: &DbEvent| -> Event {
        Event {
            id: db_event.id.0.to_string(),
            created_at: format_timestamp(&db_event.created_at),
            event: match &db_event.event {
                DbEventType::Grabbed {
                    grabber,
                    cost,
                    wedged,
                } => EventType::Grabbed {
                    grabber: grabber.clone(),
                    cost: cost.as_ref().map(|c| c.into()),
                    wedged: *wedged,
                },
                DbEventType::Linked {
                    linker,
                    library_path,
                } => EventType::Linked {
                    linker: linker.clone(),
                    library_path: library_path.clone(),
                },
                DbEventType::Cleaned {
                    library_path,
                    files,
                } => EventType::Cleaned {
                    library_path: library_path.clone(),
                    files: files.clone(),
                },
                DbEventType::Updated { fields, source } => EventType::Updated {
                    fields: fields
                        .iter()
                        .map(|f| TorrentMetaDiff {
                            field: f.field.to_string(),
                            from: f.from.clone(),
                            to: f.to.clone(),
                        })
                        .collect(),
                    source: (MetadataSource::from(&source.0), source.1.clone()),
                },
                DbEventType::RemovedFromTracker => EventType::RemovedFromTracker,
            },
        }
    };

    let no_filters = show.is_none()
        && grabber.is_none()
        && linker.is_none()
        && category.is_none()
        && has_updates.is_none()
        && field.is_none();

    if no_filters {
        let total = r
            .len()
            .secondary::<DbEvent>(EventKey::created_at)
            .server_err()?;
        let events_iter = r
            .scan()
            .secondary::<DbEvent>(EventKey::created_at)
            .server_err()?;
        let events = events_iter
            .all()
            .server_err()?
            .rev()
            .skip(from_val)
            .take(page_size_val);

        let mut result_events = Vec::new();
        for event_res in events {
            let db_event = event_res.server_err()?;
            let db_torrent: Option<mlm_core::Torrent> = if let Some(id) = &db_event.torrent_id {
                r.get().primary(id.clone()).ok().flatten()
            } else if let Some(mam_id) = &db_event.mam_id {
                r.get()
                    .secondary(TorrentKey::mam_id, *mam_id)
                    .ok()
                    .flatten()
            } else {
                None
            };

            let mut db_replacement = None;
            if let Some(ref t) = db_torrent {
                db_replacement = t
                    .replaced_with
                    .clone()
                    .and_then(|(id, _)| r.get().primary(id).ok().flatten());
            }

            result_events.push(EventWithTorrentData {
                event: convert_event(&db_event),
                torrent: db_torrent.as_ref().map(convert_torrent),
                replacement: db_replacement.as_ref().map(convert_torrent),
            });
        }

        return Ok(EventData {
            events: result_events,
            total: total as usize,
            from: from_val,
            page_size: page_size_val,
        });
    }

    let events_iter = r
        .scan()
        .secondary::<DbEvent>(EventKey::created_at)
        .server_err_ctx("scan")?;

    let events = events_iter.all().server_err_ctx("all")?.rev();

    let mut result_events = Vec::new();
    let mut total_matching = 0;

    let needs_torrent_for_filter = linker.is_some() || category.is_some();

    for event_res in events {
        let db_event = event_res.server_err()?;

        let mut event_matches = true;

        if let Some(ref val) = show {
            match &db_event.event {
                DbEventType::Grabbed { .. } => {
                    if val != "grabber" {
                        event_matches = false;
                    }
                }
                DbEventType::Linked { .. } => {
                    if val != "linker" {
                        event_matches = false;
                    }
                }
                DbEventType::Cleaned { .. } => {
                    if val != "cleaner" {
                        event_matches = false;
                    }
                }
                DbEventType::Updated { .. } => {
                    if val != "updated" {
                        event_matches = false;
                    }
                }
                DbEventType::RemovedFromTracker => {
                    if val != "removed" {
                        event_matches = false;
                    }
                }
            }
        }

        if event_matches && let Some(ref val) = grabber {
            match &db_event.event {
                DbEventType::Grabbed { grabber, .. } => {
                    if val.is_empty() {
                        if grabber.is_some() {
                            event_matches = false;
                        }
                    } else if grabber.as_ref() != Some(val) {
                        event_matches = false;
                    }
                }
                _ => {
                    event_matches = false;
                }
            }
        }

        if event_matches && has_updates.is_some() {
            match &db_event.event {
                DbEventType::Updated { fields, .. } => {
                    if !fields.iter().any(|f| !f.from.is_empty()) {
                        event_matches = false;
                    }
                }
                _ => {
                    event_matches = false;
                }
            }
        }

        if event_matches && let Some(ref val) = field {
            match &db_event.event {
                DbEventType::Updated { fields, .. } => {
                    if !fields.iter().any(|f| &f.field.to_string() == val) {
                        event_matches = false;
                    }
                }
                _ => {
                    event_matches = false;
                }
            }
        }

        if !event_matches {
            continue;
        }

        let mut torrent_matches = true;
        let mut db_torrent: Option<mlm_core::Torrent> = None;
        let mut db_replacement = None;

        let in_page = total_matching >= from_val && total_matching < from_val + page_size_val;

        if needs_torrent_for_filter || in_page {
            db_torrent = if let Some(id) = &db_event.torrent_id {
                r.get().primary(id.clone()).ok().flatten()
            } else if let Some(mam_id) = &db_event.mam_id {
                r.get()
                    .secondary(TorrentKey::mam_id, *mam_id)
                    .ok()
                    .flatten()
            } else {
                None
            };

            if let Some(ref t) = db_torrent {
                if let Some(ref val) = linker
                    && t.linker.as_ref() != Some(val)
                {
                    torrent_matches = false;
                }
                if let Some(ref val) = category {
                    let cat_matches = if val.is_empty() {
                        t.category.is_none()
                    } else {
                        t.category.as_ref() == Some(val)
                    };
                    if !cat_matches {
                        torrent_matches = false;
                    }
                }

                if torrent_matches && in_page {
                    db_replacement = t
                        .replaced_with
                        .clone()
                        .and_then(|(id, _)| r.get().primary(id).ok().flatten());
                }
            } else if needs_torrent_for_filter {
                torrent_matches = false;
            }
        }

        if torrent_matches {
            if in_page {
                result_events.push(EventWithTorrentData {
                    event: convert_event(&db_event),
                    torrent: db_torrent.as_ref().map(convert_torrent),
                    replacement: db_replacement.as_ref().map(convert_torrent),
                });
            }
            total_matching += 1;
        }
    }

    Ok(EventData {
        events: result_events,
        total: total_matching,
        from: from_val,
        page_size: page_size_val,
    })
}
