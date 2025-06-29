use std::{
    fmt::{self, Display},
    sync::Arc,
};

use anyhow::Result;
use askama::Template;
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use itertools::Itertools;
use native_db::Database;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use time::{
    OffsetDateTime, UtcOffset,
    format_description::{self, OwnedFormatItem},
};
use tower_http::services::ServeDir;

use crate::{
    config::Config,
    data::{DuplicateTorrent, ErroredTorrent, ErroredTorrentId, SelectedTorrent, Torrent},
};

pub async fn start_webserver(config: Arc<Config>, db: Arc<Database<'static>>) -> Result<()> {
    let app = Router::new()
        .route("/", get(index_page))
        .route("/errors", get(errors_page).with_state(db.clone()))
        .route("/selected", get(selected_page).with_state(db.clone()))
        .route("/duplicate", get(duplicate_page).with_state(db.clone()))
        .route("/torrents", get(torrents_page).with_state(db.clone()))
        .nest_service("/assets", ServeDir::new("assets"));

    let listener =
        tokio::net::TcpListener::bind((config.web_host.clone(), config.web_port)).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index_page() -> std::result::Result<Html<String>, AppError> {
    let template = IndexPageTemplate {};
    Ok::<_, AppError>(Html(template.to_string()))
}

async fn errors_page(
    State(db): State<Arc<Database<'static>>>,
    Query(sort): Query<SortOn<ErrorsPageSort>>,
) -> std::result::Result<Html<String>, AppError> {
    let mut errored_torrents = db
        .r_transaction()?
        .scan()
        .primary::<ErroredTorrent>()?
        .all()?
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    if let Some(sort_by) = &sort.sort_by {
        errored_torrents.sort_by(|a, b| {
            let ord = match sort_by {
                ErrorsPageSort::Step => a.id.step().cmp(b.id.step()),
                ErrorsPageSort::Title => a.title.cmp(&b.title),
                ErrorsPageSort::Error => a.error.cmp(&b.error),
                ErrorsPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if sort.asc { ord.reverse() } else { ord }
        });
    }
    let template = ErrorsPageTemplate {
        sort,
        errors: errored_torrents,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

async fn selected_page(
    State(db): State<Arc<Database<'static>>>,
    Query(sort): Query<SortOn<SelectedPageSort>>,
) -> std::result::Result<Html<String>, AppError> {
    let mut torrents = db
        .r_transaction()?
        .scan()
        .primary::<SelectedTorrent>()?
        .all()?
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    if let Some(sort_by) = &sort.sort_by {
        torrents.sort_by(|a, b| {
            let ord = match sort_by {
                SelectedPageSort::Kind => a.meta.main_cat.cmp(&b.meta.main_cat),
                SelectedPageSort::Title => a.meta.title.cmp(&b.meta.title),
                SelectedPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                SelectedPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                SelectedPageSort::Series => a.meta.series.cmp(&b.meta.series),
                SelectedPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if sort.asc { ord.reverse() } else { ord }
        });
    }
    let template = SelectedPageTemplate { sort, torrents };
    Ok::<_, AppError>(Html(template.to_string()))
}

async fn duplicate_page(
    State(db): State<Arc<Database<'static>>>,
    Query(sort): Query<SortOn<DuplicatePageSort>>,
) -> std::result::Result<Html<String>, AppError> {
    let mut torrents = db
        .r_transaction()?
        .scan()
        .primary::<DuplicateTorrent>()?
        .all()?
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    if let Some(sort_by) = &sort.sort_by {
        torrents.sort_by(|a, b| {
            let ord = match sort_by {
                DuplicatePageSort::Kind => a.meta.main_cat.cmp(&b.meta.main_cat),
                DuplicatePageSort::Title => a.meta.title.cmp(&b.meta.title),
                DuplicatePageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                DuplicatePageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                DuplicatePageSort::Series => a.meta.series.cmp(&b.meta.series),
                DuplicatePageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if sort.asc { ord.reverse() } else { ord }
        });
    }
    let template = DuplicatePageTemplate { sort, torrents };
    Ok::<_, AppError>(Html(template.to_string()))
}

async fn torrents_page(
    State(db): State<Arc<Database<'static>>>,
    Query(sort): Query<SortOn<TorrentsPageSort>>,
) -> std::result::Result<Html<String>, AppError> {
    let mut torrents = db
        .r_transaction()?
        .scan()
        .primary::<Torrent>()?
        .all()?
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    if let Some(sort_by) = &sort.sort_by {
        torrents.sort_by(|a, b| {
            let ord = match sort_by {
                TorrentsPageSort::Kind => a.meta.main_cat.cmp(&b.meta.main_cat),
                TorrentsPageSort::Title => a.meta.title.cmp(&b.meta.title),
                TorrentsPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                TorrentsPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                TorrentsPageSort::Series => a.meta.series.cmp(&b.meta.series),
                TorrentsPageSort::Linked => a.library_path.is_some().cmp(&b.library_path.is_some()),
                TorrentsPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if sort.asc { ord.reverse() } else { ord }
        });
    }
    let template = TorrentsPageTemplate { sort, torrents };
    Ok::<_, AppError>(Html(template.to_string()))
}

#[derive(Template)]
#[template(path = "pages/index.html")]
struct IndexPageTemplate {}

#[derive(Template)]
#[template(path = "pages/errors.html")]
struct ErrorsPageTemplate {
    sort: SortOn<ErrorsPageSort>,
    errors: Vec<ErroredTorrent>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ErrorsPageSort {
    Step,
    Title,
    Error,
    CreatedAt,
}

impl SortKey for ErrorsPageSort {}

impl Sortable for ErrorsPageTemplate {
    type SortKey = ErrorsPageSort;

    fn get_current_sort(&self) -> SortOn<Self::SortKey> {
        self.sort
    }
}

#[derive(Template)]
#[template(path = "pages/selected.html")]
struct SelectedPageTemplate {
    sort: SortOn<SelectedPageSort>,
    torrents: Vec<SelectedTorrent>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SelectedPageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    CreatedAt,
}

impl SortKey for SelectedPageSort {}

impl Sortable for SelectedPageTemplate {
    type SortKey = SelectedPageSort;

    fn get_current_sort(&self) -> SortOn<Self::SortKey> {
        self.sort
    }
}

#[derive(Template)]
#[template(path = "pages/duplicate.html")]
struct DuplicatePageTemplate {
    sort: SortOn<DuplicatePageSort>,
    torrents: Vec<DuplicateTorrent>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DuplicatePageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    CreatedAt,
}

impl SortKey for DuplicatePageSort {}

impl Sortable for DuplicatePageTemplate {
    type SortKey = DuplicatePageSort;

    fn get_current_sort(&self) -> SortOn<Self::SortKey> {
        self.sort
    }
}

#[derive(Template)]
#[template(path = "pages/torrents.html")]
struct TorrentsPageTemplate {
    sort: SortOn<TorrentsPageSort>,
    torrents: Vec<Torrent>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TorrentsPageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Linked,
    CreatedAt,
}

impl SortKey for TorrentsPageSort {}

impl Sortable for TorrentsPageTemplate {
    type SortKey = TorrentsPageSort;

    fn get_current_sort(&self) -> SortOn<Self::SortKey> {
        self.sort
    }
}

#[derive(Clone, Copy, Deserialize)]
struct SortOn<T: SortKey> {
    sort_by: Option<T>,
    #[serde(default)]
    asc: bool,
}

trait SortKey: Clone + Copy + PartialEq + Serialize {}

impl<T: SortKey> Display for SortOn<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.sort_by.unwrap().serialize(f)
    }
}

trait Sortable {
    type SortKey: SortKey;

    fn get_current_sort(&self) -> SortOn<Self::SortKey>;

    fn table_header(
        &self,
        sort_key: Option<Self::SortKey>,
        label: &str,
    ) -> TableHeader<Self::SortKey> {
        let sort = self.get_current_sort();
        TableHeader {
            current_key: sort.sort_by,
            asc: sort.asc,
            key: sort_key,
            label: label.to_owned(),
        }
    }
}

/// ```askama
/// {% match key %}
/// {% when Some(key) %}
/// <a
///   href="{{link()}}"
///   class="header {% if Some(**key) == current_key %}sorting{% endif %}"
/// >
/// {{ label }}
/// {% if Some(**key) == current_key %}
///   {% if asc %}↑{% else %}↓{% endif %}
/// {% endif %}
/// </a>
/// {% when None %}
/// <div class="header">
/// {{ label }}
/// </div>
/// {% endmatch %}
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
struct TableHeader<T: SortKey> {
    current_key: Option<T>,
    asc: bool,
    key: Option<T>,
    label: String,
}

impl<T: SortKey> TableHeader<T> {
    fn link(&self) -> String {
        let key = SortOn {
            sort_by: self.key,
            asc: false,
        };
        if self.key == self.current_key {
            format!("?sort_by={}&asc={}", key, !self.asc)
        } else {
            format!("?sort_by={}", key)
        }
    }
}

fn table_styles(cols: u64) -> String {
    let mut styles = format!("grid-template-columns: repeat({cols}, auto);");

    for i in 1..=cols {
        styles.push_str(&format!("& > div:nth-child({}n+{})", cols * 2, cols + i));
        if i < cols {
            styles.push(',');
        }
    }
    styles.push_str("{ background: var(--alternate); }");

    styles
}

pub static TIME_FORMAT: Lazy<OwnedFormatItem> = Lazy::new(|| {
    format_description::parse_owned::<2>("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap()
});

fn time(time: &OffsetDateTime) -> String {
    time.to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC))
        .replace_nanosecond(0)
        .unwrap()
        .format(&TIME_FORMAT)
        .unwrap_or_default()
}

fn series((name, num): &(String, String)) -> String {
    if num.is_empty() {
        name.to_string()
    } else {
        format!("{} #{}", name, num)
    }
}

impl ErroredTorrentId {
    pub fn step(&self) -> &str {
        match self {
            crate::data::v1::ErroredTorrentId::Grabber(_) => "auto grabber",
            crate::data::v1::ErroredTorrentId::Linker(_) => "library linker",
            crate::data::v1::ErroredTorrentId::Cleaner(_) => "library cleaner",
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Could not query db: {0}")]
    Db(#[from] native_db::db_type::Error),
    #[error("Could not render template: {0}")]
    Render(#[from] askama::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        #[derive(Debug, Template)]
        #[template(source = "<p>{error}</p>", ext = "html")]
        struct Tmpl {
            #[allow(dead_code)]
            error: AppError,
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
