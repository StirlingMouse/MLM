pub mod app;
pub mod components;
pub mod dto;
pub mod duplicate;
pub mod error;
pub mod errors;
pub mod events;
pub mod home;
pub mod replaced;
pub mod search;
pub mod selected;
pub mod sse;
pub mod stats;
pub mod torrent_detail;
pub mod torrents;
pub mod utils;

#[cfg(feature = "server")]
pub mod ssr {
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
    use mlm_core::Context;
    use std::convert::Infallible;
    use std::time::Duration;
    use tokio_stream::StreamExt;
    use tokio_stream::wrappers::WatchStream;

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

    pub fn router(ctx: Context) -> Router<()> {
        Router::new()
            .route("/dioxus-stats-updates", get(dioxus_stats_updates))
            .route("/dioxus-events-updates", get(dioxus_events_updates))
            .serve_api_application(ServeConfig::builder(), root)
            .layer(Extension(ctx))
    }
}

#[cfg(feature = "web")]
pub mod web {
    use crate::app::root;

    pub fn launch() {
        dioxus::launch(root);
    }
}
