mod api;
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
    config::{config_page, config_page_post},
    duplicate::{duplicate_page, duplicate_torrents_page_post},
    errors::{errors_page, errors_page_post},
    events::event_page,
    index::{index_page, index_page_post},
    list::{list_page, list_page_post},
    lists::lists_page,
    replaced::{replaced_torrents_page, replaced_torrents_page_post},
    selected::{selected_page, selected_torrents_page_post},
    torrent::{torrent_file, torrent_page, torrent_page_post},
    torrent_edit::{torrent_edit_page, torrent_edit_page_post},
    torrents::{torrents_page, torrents_page_post},
};
use reqwest::header;
use serde::Serialize;
use tables::{ItemFilter, ItemFilters, Key};
use time::{
    Date, UtcDateTime, UtcOffset,
    format_description::{self, OwnedFormatItem},
};
use tokio::sync::{Mutex, watch::error::SendError};
use tower::ServiceBuilder;
#[allow(unused)]
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    config::{Config, SearchConfig, TorrentFilter},
    data::{AudiobookCategory, EbookCategory, Series, Timestamp, Torrent, TorrentMeta},
    mam::{api::MaM, enums::Flags, meta::MetaError, search::MaMTorrent, serde::DATE_FORMAT},
    stats::{Stats, Triggers},
    web::{
        api::torrent::torrent_api,
        pages::search::{search_page, search_page_post},
    },
};

pub type MaMState = Arc<Result<Arc<MaM<'static>>>>;

pub async fn start_webserver(
    config: Arc<Config>,
    db: Arc<Database<'static>>,
    stats: Arc<Mutex<Stats>>,
    mam: Arc<Result<Arc<MaM<'static>>>>,
    triggers: Triggers,
) -> Result<()> {
    let app = Router::new()
        .route(
            "/",
            get(index_page).with_state((config.clone(), stats, mam.clone())),
        )
        .route("/", post(index_page_post).with_state(triggers.clone()))
        .route(
            "/torrents",
            get(torrents_page).with_state((config.clone(), db.clone())),
        )
        .route(
            "/torrents",
            post(torrents_page_post).with_state((config.clone(), db.clone(), mam.clone())),
        )
        .route(
            "/torrents/{hash}",
            get(torrent_page).with_state((config.clone(), db.clone(), mam.clone())),
        )
        .route(
            "/torrents/{hash}",
            post(torrent_page_post).with_state((
                config.clone(),
                db.clone(),
                mam.clone(),
                triggers.clone(),
            )),
        )
        .route(
            "/torrents/{hash}/edit",
            get(torrent_edit_page).with_state(db.clone()),
        )
        .route(
            "/torrents/{hash}/edit",
            post(torrent_edit_page_post).with_state((config.clone(), db.clone(), mam.clone())),
        )
        .route(
            "/torrents/{hash}/{filename}",
            get(torrent_file).with_state((config.clone(), db.clone())),
        )
        .route("/events", get(event_page).with_state(db.clone()))
        .route(
            "/search",
            get(search_page).with_state((config.clone(), db.clone(), mam.clone())),
        )
        .route(
            "/search",
            post(search_page_post).with_state((config.clone(), db.clone(), mam.clone(), triggers)),
        )
        .route(
            "/lists",
            get(lists_page).with_state((config.clone(), db.clone())),
        )
        .route("/lists/{list_id}", get(list_page).with_state(db.clone()))
        .route(
            "/lists/{list_id}",
            post(list_page_post).with_state(db.clone()),
        )
        .route("/errors", get(errors_page).with_state(db.clone()))
        .route("/errors", post(errors_page_post).with_state(db.clone()))
        .route(
            "/selected",
            get(selected_page).with_state((config.clone(), db.clone(), mam.clone())),
        )
        .route(
            "/selected",
            post(selected_torrents_page_post).with_state(db.clone()),
        )
        .route(
            "/replaced",
            get(replaced_torrents_page).with_state((config.clone(), db.clone())),
        )
        .route(
            "/replaced",
            post(replaced_torrents_page_post).with_state((config.clone(), db.clone(), mam.clone())),
        )
        .route(
            "/duplicate",
            get(duplicate_page).with_state((config.clone(), db.clone())),
        )
        .route(
            "/duplicate",
            post(duplicate_torrents_page_post).with_state((
                config.clone(),
                db.clone(),
                mam.clone(),
            )),
        )
        .route("/config", get(config_page).with_state(config.clone()))
        .route(
            "/config",
            post(config_page_post).with_state((config.clone(), db.clone(), mam.clone())),
        )
        .route(
            "/api/torrents/{hash}",
            get(torrent_api).with_state((config.clone(), db.clone(), mam.clone())),
        )
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

pub trait Page {
    fn build_date(&self) -> &'static str {
        env!("DATE")
    }

    fn item_path(&self) -> &'static str {
        ""
    }

    fn item<'a, T: Key>(&'a self, field: T, label: &'a str) -> ItemFilter<'a, T> {
        ItemFilter {
            field,
            label,
            value: None,
            path: self.item_path(),
        }
    }

    fn item_v<'a, T: Key>(&self, field: T, label: &'a str, value: &'a str) -> ItemFilter<'a, T> {
        ItemFilter {
            field,
            label,
            value: Some(value),
            path: self.item_path(),
        }
    }

    fn items<'a, T: Key>(&self, field: T, labels: &'a [String]) -> ItemFilters<'a, T> {
        ItemFilters {
            field,
            labels,
            path: self.item_path(),
        }
    }

    fn series<'a, T: Key>(&'a self, field: T, series: &'a Vec<Series>) -> SeriesTmpl<'a, T> {
        SeriesTmpl {
            field,
            series,
            path: self.item_path(),
        }
    }
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
/// {% for s in series %}
/// {{ item(*field, s.name) | safe }}{% if !s.entries.0.is_empty() %} #{{ s.entries }}{% endif %}{% if !loop.last %}, {% endif %}
/// {% endfor %}
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
pub struct SeriesTmpl<'a, T: Key> {
    field: T,
    series: &'a Vec<Series>,
    path: &'a str,
}

impl<'a, T: Key> SeriesTmpl<'a, T> {
    fn item(&'a self, field: T, label: &'a str) -> ItemFilter<'a, T> {
        ItemFilter {
            field,
            label,
            value: None,
            path: self.path,
        }
    }
}

impl<'a, T: Key> HtmlSafe for SeriesTmpl<'a, T> {}

#[derive(Template)]
#[template(path = "partials/filter.html")]
struct FilterTemplate<'a> {
    filter: &'a TorrentFilter,
}
impl<'a> HtmlSafe for FilterTemplate<'a> {}

fn filter<'a>(filter: &'a TorrentFilter) -> FilterTemplate<'a> {
    FilterTemplate { filter }
}

#[derive(Template)]
#[template(path = "partials/flag_icons.html")]
pub struct FlagIconsTemplate {
    flags: Flags,
}
impl HtmlSafe for FlagIconsTemplate {}

impl TorrentMeta {
    pub fn flag_icons(&self) -> FlagIconsTemplate {
        FlagIconsTemplate {
            flags: Flags::from_bitfield(self.flags.map_or(0, |f| f.0)),
        }
    }
}

#[derive(Template)]
#[template(path = "partials/cost_icon.html")]
pub struct CostIconTemplate<'a> {
    mam_torrent: &'a MaMTorrent,
}
impl<'a> HtmlSafe for CostIconTemplate<'a> {}

impl MaMTorrent {
    pub fn cost_icon(&self) -> CostIconTemplate<'_> {
        CostIconTemplate { mam_torrent: self }
    }
    pub fn vip_expire(&self) -> Date {
        UtcDateTime::from_unix_timestamp(self.vip_expire as i64)
            .unwrap_or(UtcDateTime::UNIX_EPOCH)
            .date()
    }
}

#[derive(Template)]
#[template(path = "partials/mam_torrents.html")]
struct MaMTorrentsTemplate {
    config: SearchConfig,
    torrents: Vec<(MaMTorrent, TorrentMeta, Option<Torrent>)>,
}
impl HtmlSafe for MaMTorrentsTemplate {}

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

impl<T: Template> HtmlSafe for Conditional<T> {}

/// ```askama
/// <a href="/torrents/{{ hash }}" class=torrent>{{ title }}</a>
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
struct TorrentLink<'a> {
    hash: &'a str,
    title: &'a str,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Could not query db: {0}")]
    Db(#[from] native_db::db_type::Error),
    #[error("Could not render template: {0}")]
    Render(#[from] askama::Error),
    #[error("Qbit Error: {0:?}")]
    QbitError(#[from] qbit::Error),
    #[error("Send Error: {0:?}")]
    SendError(#[from] SendError<()>),
    #[error("Send Error: {0:?}")]
    SendError2(#[from] SendError<isize>),
    #[error("Meta Error: {0:?}")]
    MetaError(#[from] MetaError),
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
