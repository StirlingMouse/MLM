mod download;
mod error;
mod search;
mod torrent;

use std::path::PathBuf;

use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Request},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use mlm_core::Context;
use tower::ServiceBuilder;
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    download::torrent_file,
    search::{search_api, search_api_post},
    torrent::torrent_api,
};

pub fn router(context: Context, dioxus_public_path: PathBuf) -> Router {
    let dioxus_assets_path = dioxus_public_path.join("assets");
    let app = Router::new()
        .route("/api/search", get(search_api).post(search_api_post))
        .route(
            "/api/torrents/{id}",
            get(torrent_api).with_state(context.clone()),
        )
        .route(
            "/torrents/{id}/{filename}",
            get(torrent_file).with_state(context.clone()),
        )
        .with_state(context.clone())
        .nest_service(
            "/assets",
            ServiceBuilder::new()
                .layer(middleware::from_fn(set_static_cache_control))
                .service(ServeDir::new(dioxus_assets_path).fallback(ServeDir::new("server/assets"))),
        );

    #[cfg(debug_assertions)]
    let app = app.nest_service(
        "/assets/favicon.png",
        ServiceBuilder::new()
            .layer(middleware::from_fn(set_static_cache_control))
            .service(ServeFile::new("server/assets/favicon_dev.png")),
    );

    #[cfg(debug_assertions)]
    let app = app.nest_service(
        "/favicon.ico",
        ServiceBuilder::new()
            .layer(middleware::from_fn(set_static_cache_control))
            .service(ServeFile::new("server/assets/favicon_dev.png")),
    );

    #[cfg(not(debug_assertions))]
    let app = app.nest_service(
        "/favicon.ico",
        ServiceBuilder::new()
            .layer(middleware::from_fn(set_static_cache_control))
            .service(ServeFile::new("server/assets/favicon.png")),
    );

    app
}

async fn set_static_cache_control(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("must-revalidate"),
    );
    response
}
