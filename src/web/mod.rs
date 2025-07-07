use std::{
    cell::RefCell,
    fmt::{self, Display},
    str::FromStr,
    sync::Arc,
};

use anyhow::Result;
use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use native_db::Database;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use time::{
    UtcOffset,
    format_description::{self, OwnedFormatItem},
};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

use crate::{
    config::Config,
    data::{
        DuplicateTorrent, ErroredTorrent, ErroredTorrentId, Event, EventKey, EventType, Language,
        List, ListItem, ListItemKey, ListKey, SelectedTorrent, Timestamp, Torrent, TorrentKey,
    },
    stats::Stats,
};

pub async fn start_webserver(
    config: Arc<Config>,
    db: Arc<Database<'static>>,
    stats: Arc<Mutex<Stats>>,
) -> Result<()> {
    let app = Router::new()
        .route("/", get(index_page).with_state(stats))
        .route("/torrents", get(torrents_page).with_state(db.clone()))
        .route("/events", get(event_page).with_state(db.clone()))
        .route("/lists", get(lists_page).with_state(db.clone()))
        .route("/lists/{list_id}", get(list_page).with_state(db.clone()))
        .route("/errors", get(errors_page).with_state(db.clone()))
        .route("/selected", get(selected_page).with_state(db.clone()))
        .route("/duplicate", get(duplicate_page).with_state(db.clone()))
        .nest_service("/assets", ServeDir::new("assets"));

    let listener =
        tokio::net::TcpListener::bind((config.web_host.clone(), config.web_port)).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index_page(
    State(stats): State<Arc<Mutex<Stats>>>,
) -> std::result::Result<Html<String>, AppError> {
    let stats = stats.lock().await;
    let template = IndexPageTemplate {
        autograbber_run_at: stats.autograbber_run_at.map(Into::into),
        autograbber_result: stats
            .autograbber_result
            .as_ref()
            .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        linker_run_at: stats.linker_run_at.map(Into::into),
        linker_result: stats
            .linker_result
            .as_ref()
            .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        cleaner_run_at: stats.cleaner_run_at.map(Into::into),
        cleaner_result: stats
            .cleaner_result
            .as_ref()
            .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        goodreads_run_at: stats.goodreads_run_at.map(Into::into),
        goodreads_result: stats
            .goodreads_result
            .as_ref()
            .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

async fn event_page(
    State(db): State<Arc<Database<'static>>>,
    Query(filter): Query<Vec<(EventPageFilter, String)>>,
) -> std::result::Result<Html<String>, AppError> {
    let events = db
        .r_transaction()?
        .scan()
        .secondary::<Event>(EventKey::created_at)?;
    let events = events.all()?.rev();
    let mut events_with_torrent = Vec::with_capacity(events.size_hint().0);
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
    // .filter(|t| {
    //     let Ok(t) = t else {
    //         return true;
    //     };
    //     for (field, value) in filter.iter() {
    //         let ok = match field {
    //             // TODO
    //             EventPageFilter::Kind => true,
    //         };
    //         if !ok {
    //             return false;
    //         }
    //     }
    //     true
    // })
    // .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    let template = EventPageTemplate {
        events: events_with_torrent,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

async fn lists_page(
    State(db): State<Arc<Database<'static>>>,
) -> std::result::Result<Html<String>, AppError> {
    let lists = db
        .r_transaction()?
        .scan()
        .secondary::<List>(ListKey::title)?;
    let lists = lists
        .all()?
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    let template = ListsPageTemplate { lists };
    Ok::<_, AppError>(Html(template.to_string()))
}

async fn list_page(
    State(db): State<Arc<Database<'static>>>,
    Path(list_id): Path<u64>,
) -> std::result::Result<Html<String>, AppError> {
    let Some(list) = db.r_transaction()?.get().primary::<List>(list_id)? else {
        return Err(AppError::NotFound);
    };
    let items = db
        .r_transaction()?
        .scan()
        .secondary::<ListItem>(ListItemKey::created_at)?;
    let items = items
        .all()?
        .rev()
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    let template = ListPageTemplate { list, items };
    Ok::<_, AppError>(Html(template.to_string()))
}

async fn errors_page(
    State(db): State<Arc<Database<'static>>>,
    Query(sort): Query<SortOn<ErrorsPageSort>>,
    Query(filter): Query<Vec<(ErrorsPageFilter, String)>>,
) -> std::result::Result<Html<String>, AppError> {
    let mut errored_torrents = db
        .r_transaction()?
        .scan()
        .primary::<ErroredTorrent>()?
        .all()?
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            for (field, value) in filter.iter() {
                let ok = match field {
                    ErrorsPageFilter::Step => t.id.step() == value,
                    ErrorsPageFilter::Title => &t.title == value,
                    ErrorsPageFilter::SortBy => true,
                    ErrorsPageFilter::Asc => true,
                };
                if !ok {
                    return false;
                }
            }
            true
        })
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
    Query(filter): Query<Vec<(SelectedPageFilter, String)>>,
) -> std::result::Result<Html<String>, AppError> {
    let mut torrents = db
        .r_transaction()?
        .scan()
        .primary::<SelectedTorrent>()?
        .all()?
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            for (field, value) in filter.iter() {
                let ok = match field {
                    SelectedPageFilter::Kind => t.meta.main_cat.as_str() == value,
                    SelectedPageFilter::Title => &t.meta.title == value,
                    SelectedPageFilter::Author => t.meta.authors.contains(value),
                    SelectedPageFilter::Narrator => t.meta.narrators.contains(value),
                    SelectedPageFilter::Series => {
                        t.meta.series.iter().any(|(name, _)| name == value)
                    }
                    SelectedPageFilter::Filetype => t.meta.filetypes.contains(value),
                    SelectedPageFilter::SortBy => true,
                    SelectedPageFilter::Asc => true,
                };
                if !ok {
                    return false;
                }
            }
            true
        })
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
    Query(filter): Query<Vec<(DuplicatePageFilter, String)>>,
) -> std::result::Result<Html<String>, AppError> {
    let mut torrents = db
        .r_transaction()?
        .scan()
        .primary::<DuplicateTorrent>()?
        .all()?
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            for (field, value) in filter.iter() {
                let ok = match field {
                    DuplicatePageFilter::Kind => t.meta.main_cat.as_str() == value,
                    DuplicatePageFilter::Title => &t.meta.title == value,
                    DuplicatePageFilter::Author => t.meta.authors.contains(value),
                    DuplicatePageFilter::Narrator => t.meta.narrators.contains(value),
                    DuplicatePageFilter::Series => {
                        t.meta.series.iter().any(|(name, _)| name == value)
                    }
                    DuplicatePageFilter::Filetype => t.meta.filetypes.contains(value),
                    DuplicatePageFilter::SortBy => true,
                    DuplicatePageFilter::Asc => true,
                };
                if !ok {
                    return false;
                }
            }
            true
        })
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
    Query(filter): Query<Vec<(TorrentsPageFilter, String)>>,
    Query(show): Query<TorrentsPageColumnsQuery>,
) -> std::result::Result<Html<String>, AppError> {
    let mut torrents = db
        .r_transaction()?
        .scan()
        .secondary::<Torrent>(TorrentKey::created_at)?
        .all()?
        .rev()
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            for (field, value) in filter.iter() {
                let ok = match field {
                    TorrentsPageFilter::Kind => t.meta.main_cat.as_str() == value,
                    TorrentsPageFilter::Title => &t.meta.title == value,
                    TorrentsPageFilter::Author => t.meta.authors.contains(value),
                    TorrentsPageFilter::Narrator => t.meta.narrators.contains(value),
                    TorrentsPageFilter::Series => {
                        t.meta.series.iter().any(|(name, _)| name == value)
                    }
                    TorrentsPageFilter::Language => {
                        t.meta.language == Language::from_str(value).ok()
                    }
                    TorrentsPageFilter::Filetype => t.meta.filetypes.contains(value),
                    TorrentsPageFilter::Linked => t.library_path.is_some() == (value == "true"),
                    TorrentsPageFilter::Replaced => t.replaced_with.is_some() == (value == "true"),
                    TorrentsPageFilter::SortBy => true,
                    TorrentsPageFilter::Asc => true,
                    TorrentsPageFilter::Show => true,
                };
                if !ok {
                    return false;
                }
            }
            true
        })
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    if let Some(sort_by) = &sort.sort_by {
        torrents.sort_by(|a, b| {
            let ord = match sort_by {
                TorrentsPageSort::Kind => a.meta.main_cat.cmp(&b.meta.main_cat),
                TorrentsPageSort::Title => a.meta.title.cmp(&b.meta.title),
                TorrentsPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                TorrentsPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                TorrentsPageSort::Series => a.meta.series.cmp(&b.meta.series),
                TorrentsPageSort::Language => a.meta.language.cmp(&b.meta.language),
                TorrentsPageSort::Linked => a.library_path.cmp(&b.library_path),
                TorrentsPageSort::Replaced => a
                    .replaced_with
                    .as_ref()
                    .map(|r| r.1)
                    .cmp(&b.replaced_with.as_ref().map(|r| r.1)),
                TorrentsPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
            };
            if sort.asc { ord.reverse() } else { ord }
        });
    }
    let template = TorrentsPageTemplate {
        sort,
        show: show.show.unwrap_or_default(),
        cols: Default::default(),
        torrents,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

#[derive(Template)]
#[template(path = "pages/index.html")]
struct IndexPageTemplate {
    autograbber_run_at: Option<Timestamp>,
    autograbber_result: Option<Result<(), String>>,
    linker_run_at: Option<Timestamp>,
    linker_result: Option<Result<(), String>>,
    cleaner_run_at: Option<Timestamp>,
    cleaner_result: Option<Result<(), String>>,
    goodreads_run_at: Option<Timestamp>,
    goodreads_result: Option<Result<(), String>>,
}

#[derive(Template)]
#[template(path = "pages/events.html")]
struct EventPageTemplate {
    events: Vec<(Event, Option<Torrent>, Option<Torrent>)>,
}

impl EventPageTemplate {
    fn torrent_title<'a>(&'a self, torrent: &'a Option<Torrent>) -> &'a str {
        torrent
            .as_ref()
            .map(|t| t.meta.title.as_str())
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EventPageFilter {
    Kind,
}

impl Key for EventPageFilter {}

#[derive(Template)]
#[template(path = "pages/lists.html")]
struct ListsPageTemplate {
    lists: Vec<List>,
}

#[derive(Template)]
#[template(path = "pages/list.html")]
struct ListPageTemplate {
    list: List,
    items: Vec<ListItem>,
}

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

impl Key for ErrorsPageSort {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ErrorsPageFilter {
    Step,
    Title,
    // Workaround sort decode failure
    SortBy,
    Asc,
}

impl Key for ErrorsPageFilter {}

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

impl Key for SelectedPageSort {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SelectedPageFilter {
    Kind,
    Title,
    Author,
    Narrator,
    Series,
    Filetype,
    // Workaround sort decode failure
    SortBy,
    Asc,
}

impl Key for SelectedPageFilter {}

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

impl Key for DuplicatePageSort {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DuplicatePageFilter {
    Kind,
    Title,
    Author,
    Narrator,
    Series,
    Filetype,
    // Workaround sort decode failure
    SortBy,
    Asc,
}

impl Key for DuplicatePageFilter {}

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
    show: TorrentsPageColumns,
    cols: RefCell<Vec<String>>,
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
    Language,
    Linked,
    Replaced,
    CreatedAt,
}

impl Key for TorrentsPageSort {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TorrentsPageFilter {
    Kind,
    Title,
    Author,
    Narrator,
    Series,
    Language,
    Filetype,
    Linked,
    Replaced,
    // Workaround sort decode failure
    SortBy,
    Asc,
    Show,
}

impl Key for TorrentsPageFilter {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
struct TorrentsPageColumns {
    authors: bool,
    narrators: bool,
    series: bool,
    language: bool,
    filetypes: bool,
    path: bool,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
struct TorrentsPageColumnsQuery {
    show: Option<TorrentsPageColumns>,
}

impl Default for TorrentsPageColumns {
    fn default() -> Self {
        TorrentsPageColumns {
            authors: true,
            narrators: true,
            series: true,
            language: false,
            filetypes: true,
            path: false,
        }
    }
}

impl TryFrom<String> for TorrentsPageColumns {
    type Error = String;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let mut columns = TorrentsPageColumns {
            authors: false,
            narrators: false,
            series: false,
            language: false,
            filetypes: false,
            path: false,
        };
        for column in value.split(",") {
            match column {
                "author" => columns.authors = true,
                "narrator" => columns.narrators = true,
                "series" => columns.series = true,
                "language" => columns.language = true,
                "filetype" => columns.filetypes = true,
                "path" => columns.path = true,
                "" => {}
                _ => {
                    return Err(format!("Unknown column {column}"));
                }
            }
        }
        Ok(columns)
    }
}

impl Sortable for TorrentsPageTemplate {
    type SortKey = TorrentsPageSort;

    fn get_current_sort(&self) -> SortOn<Self::SortKey> {
        self.sort
    }
}
impl HidableColumns for TorrentsPageTemplate {
    fn add_column(&self, size: &str) {
        self.cols.borrow_mut().push(size.to_owned());
    }
}

#[derive(Clone, Copy, Deserialize)]
struct SortOn<T: Key> {
    sort_by: Option<T>,
    #[serde(default)]
    asc: bool,
}

trait Key: Clone + Copy + PartialEq + Serialize {}

impl<T: Key> Display for SortOn<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.sort_by.unwrap().serialize(f)
    }
}

trait Sortable {
    type SortKey: Key;

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
            show: true,
        }
    }
}

trait HidableColumns: Sortable {
    fn add_column(&self, size: &str);

    fn table_header_if(
        &self,
        show: &bool,
        sort_key: Option<Self::SortKey>,
        label: &str,
        size: &str,
    ) -> TableHeader<Self::SortKey> {
        let sort = self.get_current_sort();
        if *show {
            self.add_column(size);
        }
        TableHeader {
            current_key: sort.sort_by,
            asc: sort.asc,
            key: sort_key,
            label: label.to_owned(),
            show: *show,
        }
    }
    fn table_header_s(
        &self,
        sort_key: Option<Self::SortKey>,
        label: &str,
        size: &str,
    ) -> TableHeader<Self::SortKey> {
        let sort = self.get_current_sort();
        self.add_column(size);
        TableHeader {
            current_key: sort.sort_by,
            asc: sort.asc,
            key: sort_key,
            label: label.to_owned(),
            show: true,
        }
    }
}

/// ```askama
/// {% if show %}
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
/// {% endif %}
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
struct TableHeader<T: Key> {
    current_key: Option<T>,
    asc: bool,
    key: Option<T>,
    label: String,
    show: bool,
}

impl<T: Key> TableHeader<T> {
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

/// ```askama
/// <a href="{{link()}}">{{label}}</a>
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
struct ItemFilter<'a, T: Key> {
    field: T,
    label: &'a str,
}

impl<'a, T: Key> ItemFilter<'a, T> {
    fn link(&self) -> String {
        let key = SortOn {
            sort_by: Some(self.field),
            asc: false,
        };
        format!("?{}={}", key, &urlencoding::encode(self.label))
    }
}

fn item<T: Key>(field: T, label: &str) -> ItemFilter<T> {
    ItemFilter { field, label }
}

/// ```askama
/// {% for label in labels %}
/// {{ self::item(*field, label) | safe }}{% if !loop.last %}, {% endif %}
/// {% endfor %}
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
struct ItemFilters<'a, T: Key> {
    field: T,
    labels: &'a [String],
}

fn items<'a, T: Key>(field: T, labels: &'a [String]) -> ItemFilters<'a, T> {
    ItemFilters { field, labels }
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
