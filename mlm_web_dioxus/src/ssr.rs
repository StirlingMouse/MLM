use crate::app::root;
use axum::Extension;
use axum::response::sse::KeepAlive;
use axum::{
    Router,
    response::sse::{Event, Sse},
    routing::get,
};
use dioxus::prelude::*;
use dioxus::server::{DioxusRouterExt, ServeConfig};
use mlm_core::{Context, ContextExt as _};
use mlm_db::SelectedTorrent;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::{IntervalStream, WatchStream};

async fn dioxus_stats_updates(
    Extension(context): Extension<Context>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = WatchStream::new(context.stats.updates())
        .map(|_time| Ok(Event::default().data("update")));
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}

async fn dioxus_events_updates(
    Extension(context): Extension<Context>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = WatchStream::new(context.events.event.1.clone())
        .map(|_event| Ok(Event::default().data("update")));
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}

async fn dioxus_qbit_progress(
    Extension(context): Extension<Context>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream =
        IntervalStream::new(tokio::time::interval(Duration::from_secs(10))).then(move |_| {
            let context = context.clone();
            async move { fetch_qbit_progress(&context).await }
        });
    // Always send an event (empty Vec if no downloading torrents) so client can clear stale progress.
    let stream =
        stream.map(|data| Ok(Event::default().data(data.unwrap_or_else(|| "[]".to_string()))));
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}

/// Polls qBittorrent for download progress of actively-seeding MLM torrents.
/// Returns a JSON-serialized `Vec<(mam_id, progress_pct)>` or `None` if nothing is downloading.
async fn fetch_qbit_progress(context: &Context) -> Option<String> {
    let config = context.config().await;

    let downloading: Vec<(u64, String)> = context
        .db()
        .r_transaction()
        .ok()?
        .scan()
        .primary::<SelectedTorrent>()
        .ok()?
        .all()
        .ok()?
        .filter_map(Result::ok)
        .filter(|t| t.started_at.is_some() && t.removed_at.is_none())
        .filter_map(|t| t.hash.map(|h| (t.mam_id, h)))
        .collect();

    if downloading.is_empty() {
        return None;
    }

    let hash_to_mam: std::collections::HashMap<String, u64> =
        downloading.iter().map(|(id, h)| (h.clone(), *id)).collect();
    let hashes: Vec<String> = downloading.into_iter().map(|(_, h)| h).collect();

    let mut progress: Vec<(u64, u32)> = Vec::new();
    for qbit_conf in config.qbittorrent.iter() {
        let Ok(qbit) = qbit::Api::new_login_username_password(
            &qbit_conf.url,
            &qbit_conf.username,
            &qbit_conf.password,
        )
        .await
        else {
            continue;
        };
        let params = qbit::parameters::TorrentListParams {
            hashes: Some(hashes.clone()),
            ..Default::default()
        };
        let Ok(torrents) = qbit.torrents(Some(params)).await else {
            continue;
        };
        for torrent in torrents {
            if let Some(&mam_id) = hash_to_mam.get(&torrent.hash) {
                progress.push((mam_id, (torrent.progress * 100.0) as u32));
            }
        }
    }

    serde_json::to_string(&progress).ok()
}

pub fn router(ctx: Context) -> Router<()> {
    Router::new()
        .route("/dioxus-stats-updates", get(dioxus_stats_updates))
        .route("/dioxus-events-updates", get(dioxus_events_updates))
        .route("/dioxus-selected-updates", get(dioxus_stats_updates))
        .route("/dioxus-errors-updates", get(dioxus_stats_updates))
        .route("/dioxus-qbit-progress", get(dioxus_qbit_progress))
        .serve_api_application(ServeConfig::builder(), root)
        .layer(Extension(ctx))
}
