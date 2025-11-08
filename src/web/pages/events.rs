use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{OriginalUri, Query, State},
    response::{Html, IntoResponse, Response},
};
use native_db::Database;
use serde::{Deserialize, Serialize};

use crate::{
    data::{Event, EventKey, EventType, Torrent, TorrentCost},
    web::{
        AppError, Conditional, Page, TorrentLink,
        tables::{Key, Pagination, PaginationParams, table_styles},
        time,
    },
};

pub async fn event_page(
    State(db): State<Arc<Database<'static>>>,
    uri: OriginalUri,
    Query(filter): Query<Vec<(EventPageFilter, String)>>,
    Query(paging): Query<PaginationParams>,
) -> std::result::Result<Response, AppError> {
    let r = db.r_transaction()?;
    let events = r.scan().secondary::<Event>(EventKey::created_at)?;
    let events = events.all()?.rev();
    let mut events_with_torrent = Vec::with_capacity(events.size_hint().0);
    let mut events = events
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            for (field, value) in filter.iter() {
                let ok = match field {
                    EventPageFilter::Show => match t.event {
                        EventType::Grabbed { .. } => value == "grabber",
                        EventType::Linked { .. } => value == "linker",
                        EventType::Cleaned { .. } => value == "cleaner",
                        EventType::Updated { .. } => value == "updated",
                        EventType::RemovedFromMam { .. } => value == "removed",
                    },
                    EventPageFilter::Grabber => match t.event {
                        EventType::Grabbed { ref grabber, .. } => {
                            if value.is_empty() {
                                grabber.is_none()
                            } else {
                                grabber.as_ref() == Some(value)
                            }
                        }
                        _ => false,
                    },
                    EventPageFilter::Linker => match t.event {
                        EventType::Linked { ref linker, .. } => {
                            if value.is_empty() {
                                linker.is_none()
                            } else {
                                linker.as_ref() == Some(value)
                            }
                        }
                        _ => false,
                    },
                    EventPageFilter::Category => {
                        match t
                            .hash
                            .as_ref()
                            .and_then(|hash| r.get().primary::<Torrent>(hash.clone()).ok()?)
                        {
                            Some(torrent) => {
                                if value.is_empty() {
                                    torrent.category.is_none()
                                } else {
                                    torrent.category.as_ref() == Some(value)
                                }
                            }
                            None => false,
                        }
                    }
                    EventPageFilter::HasUpdates => match t.event {
                        EventType::Updated { ref fields, .. } => {
                            fields.iter().any(|f| !f.from.is_empty())
                        }
                        _ => false,
                    },
                    EventPageFilter::From => true,
                    EventPageFilter::PageSize => true,
                };
                if !ok {
                    return false;
                }
            }
            true
        })
        .collect::<Vec<_>>();
    let paging = match paging.default_page_size(uri, 500, events.len()) {
        Ok(paging) => paging,
        Err(redirect) => return Ok(redirect.into_response()),
    };
    if let Some(paging) = &paging {
        events = events
            .into_iter()
            .skip(paging.from)
            .take(paging.page_size)
            .collect();
    }
    for event in events {
        let event = event?;
        if let Some(hash) = &event.hash {
            let r = db.r_transaction()?;
            let torrent: Option<Torrent> = r.get().primary(hash.clone())?;
            let replaced_with = torrent
                .as_ref()
                .and_then(|t| t.replaced_with.clone())
                .and_then(|(hash, _)| r.get().primary(hash).ok()?);

            events_with_torrent.push((event, torrent, replaced_with));
        } else {
            events_with_torrent.push((event, None, None));
        }
    }
    let template = EventPageTemplate {
        paging: paging.unwrap_or_default(),
        show: filter.iter().find_map(|f| {
            if f.0 == EventPageFilter::Show {
                Some(f.1.as_str())
            } else {
                None
            }
        }),
        events: events_with_torrent,
    };
    Ok::<_, AppError>(Html(template.to_string()).into_response())
}

#[derive(Template)]
#[template(path = "pages/events.html")]
struct EventPageTemplate<'a> {
    paging: Pagination,
    show: Option<&'a str>,
    events: Vec<(Event, Option<Torrent>, Option<Torrent>)>,
}

impl<'a> Page for EventPageTemplate<'a> {}

impl<'a> EventPageTemplate<'a> {
    fn torrent_title(&'a self, torrent: &'a Option<Torrent>) -> Conditional<TorrentLink<'a>> {
        Conditional {
            template: torrent.as_ref().map(|t| TorrentLink {
                hash: &t.hash,
                title: &t.meta.title,
            }),
        }
    }

    fn torrent_media_type(&'a self, torrent: &'a Option<Torrent>) -> &'a str {
        torrent
            .as_ref()
            .map(|t| t.meta.media_type.as_str())
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPageFilter {
    Show,
    Grabber,
    Linker,
    Category,
    HasUpdates,
    // Workaround sort decode failure
    From,
    PageSize,
}

impl Key for EventPageFilter {}
