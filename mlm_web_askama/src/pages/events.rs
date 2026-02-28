use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{OriginalUri, Query, State},
    response::{Html, IntoResponse, Response},
};
use mlm_db::{Event, EventKey, EventType, Torrent, TorrentCost, TorrentKey};
use native_db::Database;
use serde::{Deserialize, Serialize};

use crate::{
    AppError, Conditional, Page, TorrentLink,
    tables::{Key, Pagination, PaginationParams, table_styles},
    time,
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
                    EventType::RemovedFromTracker { .. } => value == "removed",
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
                    _ => true,
                },
                EventPageFilter::Category => true,
                EventPageFilter::HasUpdates => match t.event {
                    EventType::Updated { ref fields, .. } => {
                        fields.iter().any(|f| !f.from.is_empty())
                    }
                    _ => false,
                },
                EventPageFilter::Field => match t.event {
                    EventType::Updated { ref fields, .. } => {
                        fields.iter().any(|f| &f.field.to_string() == value)
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
    let linker_filter = filter.iter().find(|f| f.0 == EventPageFilter::Linker);
    let category_filter = filter.iter().find(|f| f.0 == EventPageFilter::Category);
    let r = db.r_transaction()?;
    let events = events
        .map(|event| {
            let event = event?;
            let torrent: Option<Torrent> = if let Some(id) = &event.torrent_id {
                r.get().primary(id.clone())?
            } else if let Some(mam_id) = &event.mam_id {
                r.get().secondary(TorrentKey::mam_id, *mam_id)?
            } else {
                None
            };

            if let Some(torrent) = torrent {
                if let Some((_, linker)) = linker_filter
                    && torrent.linker.as_ref() != Some(linker)
                {
                    return Ok(None);
                }
                if let Some((_, category)) = category_filter {
                    let matches = if category.is_empty() {
                        torrent.category.is_none()
                    } else {
                        torrent.category.as_ref() == Some(category)
                    };
                    if !matches {
                        return Ok(None);
                    }
                }

                let replaced_with = torrent
                    .replaced_with
                    .clone()
                    .and_then(|(id, _)| r.get().primary(id).ok()?);
                Ok(Some((event, Some(torrent), replaced_with)))
            } else {
                if linker_filter.is_some() || category_filter.is_some() {
                    return Ok(None);
                }
                Ok(Some((event, None, None)))
            }
        })
        .filter_map(|e| match e {
            Ok(Some((event, torrent, replaced_with))) => Some(Ok((event, torrent, replaced_with))),
            Err(err) => Some(Err(err)),
            _ => None,
        });
    let mut paging = match paging.default_page_size(uri, 500, event_count as usize) {
        Ok(paging) => paging,
        Err(redirect) => return Ok(redirect.into_response()),
    };
    let events: Result<Vec<EventWithTorrent>, native_db::db_type::Error> =
        if let Some(paging) = &mut paging {
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
    let template = EventPageTemplate {
        paging: paging.unwrap_or_default(),
        show: filter.iter().find_map(|f| {
            if f.0 == EventPageFilter::Show {
                Some(f.1.as_str())
            } else {
                None
            }
        }),
        events: events?,
    };
    Ok::<_, AppError>(Html(template.to_string()).into_response())
}

type EventWithTorrent = (Event, Option<Torrent>, Option<Torrent>);
#[derive(Template)]
#[template(path = "pages/events.html")]
struct EventPageTemplate<'a> {
    paging: Pagination,
    show: Option<&'a str>,
    events: Vec<EventWithTorrent>,
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
    Field,
    // Workaround sort decode failure
    From,
    PageSize,
}

impl Key for EventPageFilter {}
