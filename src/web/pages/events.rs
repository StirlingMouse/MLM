use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
};
use native_db::Database;
use serde::{Deserialize, Serialize};

use crate::{
    data::{Event, EventKey, EventType, Torrent, TorrentCost},
    web::{
        AppError, Conditional, TorrentLink,
        tables::{Key, table_styles},
        time,
    },
};

pub async fn event_page(
    State(db): State<Arc<Database<'static>>>,
    Query(filter): Query<Vec<(EventPageFilter, String)>>,
) -> std::result::Result<Html<String>, AppError> {
    let events = db
        .r_transaction()?
        .scan()
        .secondary::<Event>(EventKey::created_at)?;
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
                },
            };
            if !ok {
                return false;
            }
        }
        true
    });
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
        show: filter.iter().find_map(|f| {
            if f.0 == EventPageFilter::Show {
                Some(f.1.as_str())
            } else {
                None
            }
        }),
        events: events_with_torrent,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

#[derive(Template)]
#[template(path = "pages/events.html")]
struct EventPageTemplate<'a> {
    show: Option<&'a str>,
    events: Vec<(Event, Option<Torrent>, Option<Torrent>)>,
}

impl<'a> EventPageTemplate<'a> {
    fn torrent_title(&'a self, torrent: &'a Option<Torrent>) -> Conditional<TorrentLink<'a>> {
        Conditional {
            template: torrent.as_ref().map(|t| TorrentLink {
                id: t.meta.mam_id,
                title: &t.meta.title,
            }),
        }
    }

    fn torrent_main_cat(&'a self, torrent: &'a Option<Torrent>) -> &'a str {
        torrent
            .as_ref()
            .map(|t| t.meta.main_cat.as_str())
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPageFilter {
    Show,
}

impl Key for EventPageFilter {}
