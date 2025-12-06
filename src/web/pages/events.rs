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
    let event_count = r.len().secondary::<Event>(EventKey::created_at)?;
    let events = events.all()?.rev();
    let mut events_with_torrent = Vec::with_capacity(events.size_hint().0);
    let events = events.filter(|t| {
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
                        .torrent_id
                        .as_ref()
                        .and_then(|id| r.get().primary::<Torrent>(id.clone()).ok()?)
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
    });
    let mut paging = match paging.default_page_size(uri, 500, event_count as usize) {
        Ok(paging) => paging,
        Err(redirect) => return Ok(redirect.into_response()),
    };
    let events: Result<Vec<Event>, native_db::db_type::Error> = if let Some(paging) = &mut paging {
        let mut count = 0;
        let events = events
            .inspect(|_| {
                count += 1;
            })
            .skip(paging.from)
            .take(paging.page_size)
            .collect();
        if count < paging.page_size + paging.from {
            paging.total = count;
        }
        events
    } else {
        events.collect()
    };
    for event in events? {
        if let Some(id) = &event.torrent_id {
            let r = db.r_transaction()?;
            let torrent: Option<Torrent> = r.get().primary(id.clone())?;
            let replaced_with = torrent
                .as_ref()
                .and_then(|t| t.replaced_with.clone())
                .and_then(|(id, _)| r.get().primary(id).ok()?);

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
                id: &t.id,
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
