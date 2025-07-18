mod pages;
mod tables;

use std::sync::Arc;

use anyhow::Result;
use askama::{Template, filters::HtmlSafe};
use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use native_db::Database;
use once_cell::sync::Lazy;
use pages::{
    config::config_page,
    duplicate::duplicate_page,
    errors::errors_page,
    events::event_page,
    index::{index_page, index_page_post},
    list::list_page,
    lists::lists_page,
    selected::selected_page,
    torrents::{torrents_page, torrents_page_post},
};
use reqwest::header;
use serde::Serialize;
use tables::{Key, item};
use time::{
    Date, UtcOffset,
    format_description::{self, OwnedFormatItem},
};
use tokio::sync::{Mutex, watch::error::SendError};
use tower::ServiceBuilder;
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    config::{Config, Filter},
    data::Timestamp,
    mam::{DATE_FORMAT, MaM},
    mam_enums::{AudiobookCategory, EbookCategory},
    qbittorrent::QbitError,
    stats::{Stats, Triggers},
};

pub async fn start_webserver(
    config: Arc<Config>,
    db: Arc<Database<'static>>,
    stats: Arc<Mutex<Stats>>,
    mam: Arc<MaM<'static>>,
    triggers: Triggers,
) -> Result<()> {
    let app = Router::new()
        .route("/", get(index_page).with_state((stats, mam.clone())))
        .route("/", post(index_page_post).with_state(triggers))
        .route("/torrents", get(torrents_page).with_state(db.clone()))
        .route(
            "/torrents",
            post(torrents_page_post).with_state((config.clone(), db.clone(), mam.clone())),
        )
        .route("/events", get(event_page).with_state(db.clone()))
        .route("/lists", get(lists_page).with_state(db.clone()))
        .route("/lists/{list_id}", get(list_page).with_state(db.clone()))
        .route("/errors", get(errors_page).with_state(db.clone()))
        .route(
            "/selected",
            get(selected_page).with_state((config.clone(), db.clone(), mam.clone())),
        )
        .route("/duplicate", get(duplicate_page).with_state(db.clone()))
        .route("/config", get(config_page).with_state(config.clone()))
        .nest_service(
            "/assets",
            ServiceBuilder::new()
                .layer(middleware::from_fn(set_static_cache_control))
                .service(ServeDir::new("assets")),
        );

    #[cfg(debug_assertions)]
    let app = app.nest_service(
        "/assets/favicon.png",
        ServiceBuilder::new()
            .layer(middleware::from_fn(set_static_cache_control))
            .service(ServeFile::new("assets/favicon_dev.png")),
    );

    let listener =
        tokio::net::TcpListener::bind((config.web_host.clone(), config.web_port)).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn set_static_cache_control(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("must-revalidate"),
    );
    response
}

/// ```askama
/// {% if values.len() >= 5 %}
/// [<br>
/// {% for v in values %}
///   <span class=string>{{ v | json }}</span>,<br>
/// {% endfor %}
/// ]
/// {% else %}
/// [ {% for v in values %}<span class=string>{{ v | json }}</span>{% if !loop.last %}, {% endif %}{% endfor %} ]
/// {% endif %}
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
struct YamlItems<'a, V: Serialize> {
    values: &'a [V],
}

impl<'a, V: Serialize> HtmlSafe for YamlItems<'a, V> {}

fn yaml_items<'a, V: Serialize>(values: &'a [V]) -> YamlItems<'a, V> {
    YamlItems { values }
}

fn date(date: &Date) -> String {
    date.format(&DATE_FORMAT).unwrap_or_default()
}

pub static TIME_FORMAT: Lazy<OwnedFormatItem> = Lazy::new(|| {
    format_description::parse_owned::<2>("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap()
});

fn time(time: &Timestamp) -> String {
    time.0
        .to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC))
        .replace_nanosecond(0)
        .unwrap()
        .format(&TIME_FORMAT)
        .unwrap_or_default()
}

/// ```askama
/// {% for (name, num) in series %}
/// {{ self::item(*field, name) | safe }}{% if !num.is_empty() %} #{{ num }}{% endif %}{% if !loop.last %}, {% endif %}
/// {% endfor %}
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
struct Series<'a, T: Key> {
    field: T,
    series: &'a Vec<(String, String)>,
}

fn series<T: Key>(field: T, series: &Vec<(String, String)>) -> Series<'_, T> {
    Series { field, series }
}

#[derive(Template)]
#[template(path = "partials/filter.html")]
struct FilterTemplate<'a> {
    filter: &'a Filter,
}
impl<'a> HtmlSafe for FilterTemplate<'a> {}

fn filter<'a>(filter: &'a Filter) -> FilterTemplate<'a> {
    FilterTemplate { filter }
}

/// ```askama
/// {% match template %}
/// {% when Some(template) %}{{ template | safe }}
/// {% when None %}{% endmatch %}
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
struct Conditional<T: Template> {
    template: Option<T>,
}

/// ```askama
/// <a href="https://www.myanonamouse.net/t/{{ id }}" class=torrent target=_blank>{{ title }}</a>
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
struct TorrentLink<'a> {
    id: u64,
    title: &'a str,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Could not query db: {0}")]
    Db(#[from] native_db::db_type::Error),
    #[error("Could not render template: {0}")]
    Render(#[from] askama::Error),
    #[error("Qbit Error: {0:?}")]
    QbitError(#[from] QbitError),
    #[error("Send Error: {0:?}")]
    SendError(#[from] SendError<()>),
    #[error("Error: {0:?}")]
    Generic(#[from] anyhow::Error),
    #[error("Page Not Found")]
    NotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        #[derive(Debug, Template)]
        #[template(source = "<p>{{error}}</p>", ext = "html")]
        struct Tmpl {
            #[allow(dead_code)]
            error: AppError,
        }
        match self {
            AppError::Db(ref error) => eprintln!("{:?}", error),
            AppError::Render(ref error) => eprintln!("{:?}", error),
            _ => {}
        }

        let status = StatusCode::INTERNAL_SERVER_ERROR;
        let tmpl = Tmpl { error: self };
        if let Ok(body) = tmpl.render() {
            (status, Html(body)).into_response()
        } else {
            (status, "Something went wrong").into_response()
        }
    }
}
