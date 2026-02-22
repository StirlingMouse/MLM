use std::collections::BTreeSet;

use crate::components::Pagination;
use dioxus::prelude::*;
#[cfg(feature = "web")]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use mlm_core::{
    Context, ContextExt, Torrent as DbTorrent, TorrentKey,
    cleaner::clean_torrent,
    linker::{refresh_mam_metadata, refresh_metadata_relink},
};
#[cfg(feature = "server")]
use mlm_db::{
    ClientStatus, DatabaseExt as _, Flags, Language, LibraryMismatch, MetadataSource, OldCategory,
    ids,
};
#[cfg(feature = "server")]
use std::str::FromStr;
#[cfg(feature = "server")]
use sublime_fuzzy::FuzzySearch;

#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TorrentsPageSort {
    Kind,
    Category,
    Title,
    Edition,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Linker,
    QbitCategory,
    Linked,
    CreatedAt,
    UploadedAt,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TorrentsPageFilter {
    Kind,
    Category,
    Categories,
    Flags,
    Title,
    Author,
    Narrator,
    Series,
    Language,
    Filetype,
    Linker,
    QbitCategory,
    Linked,
    LibraryMismatch,
    ClientStatus,
    Abs,
    Query,
    Source,
    Metadata,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TorrentsBulkAction {
    Refresh,
    RefreshRelink,
    Clean,
    Remove,
}

impl TorrentsBulkAction {
    fn label(self) -> &'static str {
        match self {
            Self::Refresh => "refresh metadata",
            Self::RefreshRelink => "refresh metadata and relink",
            Self::Clean => "clean torrent",
            Self::Remove => "remove torrent from MLM",
        }
    }

    fn success_label(self) -> &'static str {
        match self {
            Self::Refresh => "Refreshed metadata",
            Self::RefreshRelink => "Refreshed metadata and relinked",
            Self::Clean => "Cleaned torrents",
            Self::Remove => "Removed torrents",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentsPageColumns {
    pub category: bool,
    pub categories: bool,
    pub flags: bool,
    pub edition: bool,
    pub authors: bool,
    pub narrators: bool,
    pub series: bool,
    pub language: bool,
    pub size: bool,
    pub filetypes: bool,
    pub linker: bool,
    pub qbit_category: bool,
    pub path: bool,
    pub created_at: bool,
    pub uploaded_at: bool,
}

impl Default for TorrentsPageColumns {
    fn default() -> Self {
        Self {
            category: false,
            categories: false,
            flags: false,
            edition: false,
            authors: true,
            narrators: true,
            series: true,
            language: false,
            size: true,
            filetypes: true,
            linker: false,
            qbit_category: false,
            path: false,
            created_at: true,
            uploaded_at: false,
        }
    }
}

impl TorrentsPageColumns {
    fn table_grid_template(self) -> String {
        let mut cols = vec!["30px", if self.category { "130px" } else { "89px" }];
        if self.categories {
            cols.push("1fr");
        }
        if self.flags {
            cols.push("60px");
        }
        cols.push("2fr");
        if self.edition {
            cols.push("80px");
        }
        if self.authors {
            cols.push("1fr");
        }
        if self.narrators {
            cols.push("1fr");
        }
        if self.series {
            cols.push("1fr");
        }
        if self.language {
            cols.push("100px");
        }
        if self.size {
            cols.push("81px");
        }
        if self.filetypes {
            cols.push("100px");
        }
        if self.linker {
            cols.push("130px");
        }
        if self.qbit_category {
            cols.push("100px");
        }
        cols.push(if self.path { "2fr" } else { "72px" });
        if self.created_at {
            cols.push("157px");
        }
        if self.uploaded_at {
            cols.push("157px");
        }
        cols.push("132px");
        cols.join(" ")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentsSeries {
    pub name: String,
    pub entries: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TorrentLibraryMismatch {
    NewLibraryDir(String),
    NewPath(String),
    NoLibrary,
}

impl TorrentLibraryMismatch {
    fn filter_value(&self) -> &'static str {
        match self {
            Self::NewLibraryDir(_) => "new_library",
            Self::NewPath(_) => "new_path",
            Self::NoLibrary => "no_library",
        }
    }

    fn title(&self) -> String {
        match self {
            Self::NewLibraryDir(path) => format!("Wanted library dir: {path}"),
            Self::NewPath(path) => format!("Wanted library path: {path}"),
            Self::NoLibrary => "No longer wanted in library".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentsMeta {
    pub title: String,
    pub media_type: String,
    pub cat_name: String,
    pub cat_id: Option<String>,
    pub categories: Vec<String>,
    pub flags: Vec<String>,
    pub edition: Option<String>,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<TorrentsSeries>,
    pub language: Option<String>,
    pub size: String,
    pub filetypes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentsRow {
    pub id: String,
    pub mam_id: Option<u64>,
    pub meta: TorrentsMeta,
    pub linker: Option<String>,
    pub category: Option<String>,
    pub library_path: Option<String>,
    pub library_mismatch: Option<TorrentLibraryMismatch>,
    pub client_status: Option<String>,
    pub linked: bool,
    pub created_at: String,
    pub uploaded_at: String,
    pub abs_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct TorrentsData {
    pub torrents: Vec<TorrentsRow>,
    pub total: usize,
    pub from: usize,
    pub page_size: usize,
    pub abs_url: Option<String>,
}

#[server]
pub async fn get_torrents_data(
    sort: Option<TorrentsPageSort>,
    asc: bool,
    filters: Vec<(TorrentsPageFilter, String)>,
    from: Option<usize>,
    page_size: Option<usize>,
    show: TorrentsPageColumns,
) -> Result<TorrentsData, ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let db = context.db();

    let mut from_val = from.unwrap_or(0);
    let page_size_val = page_size.unwrap_or(500);

    let r = db
        .r_transaction()
        .context("r_transaction")
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let torrents_iter = r
        .scan()
        .secondary::<DbTorrent>(TorrentKey::created_at)
        .context("scan")
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let torrents = torrents_iter
        .all()
        .context("all")
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .rev();

    let query = filters
        .iter()
        .find(|(field, value)| *field == TorrentsPageFilter::Query && !value.is_empty())
        .map(|(_, value)| value.clone());

    if sort.is_none() && query.is_none() && filters.is_empty() {
        let total = r
            .len()
            .secondary::<DbTorrent>(TorrentKey::created_at)
            .map_err(|e| ServerFnError::new(e.to_string()))? as usize;
        if page_size_val > 0 && from_val >= total && total > 0 {
            from_val = ((total - 1) / page_size_val) * page_size_val;
        }

        let mut rows = Vec::new();
        let limit = if page_size_val == 0 {
            usize::MAX
        } else {
            page_size_val
        };
        for torrent in torrents.skip(from_val).take(limit) {
            let t = torrent
                .context("torrent")
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            rows.push(convert_torrent_row(&t));
        }

        let abs_url = context
            .config()
            .await
            .audiobookshelf
            .as_ref()
            .map(|abs| abs.url.clone());

        return Ok(TorrentsData {
            torrents: rows,
            total,
            from: from_val,
            page_size: page_size_val,
            abs_url,
        });
    }

    if sort.is_none() && query.is_none() {
        let mut rows = Vec::new();
        let mut total = 0usize;
        let limit = if page_size_val == 0 {
            usize::MAX
        } else {
            page_size_val
        };
        for torrent in torrents {
            let t = torrent
                .context("torrent")
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            if filters
                .iter()
                .all(|(field, value)| matches_filter(&t, *field, value))
            {
                if total >= from_val && rows.len() < limit {
                    rows.push(convert_torrent_row(&t));
                }
                total += 1;
            }
        }

        let abs_url = context
            .config()
            .await
            .audiobookshelf
            .as_ref()
            .map(|abs| abs.url.clone());

        return Ok(TorrentsData {
            torrents: rows,
            total,
            from: from_val,
            page_size: page_size_val,
            abs_url,
        });
    }

    let mut filtered_torrents = Vec::new();

    for torrent in torrents {
        let t = torrent
            .context("torrent")
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut matches = true;
        for (field, value) in &filters {
            if !matches_filter(&t, *field, value) {
                matches = false;
                break;
            }
        }
        if !matches {
            continue;
        }

        let mut score = 0;
        if let Some(value) = query.as_deref() {
            score += fuzzy_score(value, &t.meta.title);
            if show.authors {
                for author in &t.meta.authors {
                    score += fuzzy_score(value, author);
                }
            }
            if show.narrators {
                for narrator in &t.meta.narrators {
                    score += fuzzy_score(value, narrator);
                }
            }
            if show.series {
                for s in &t.meta.series {
                    score += fuzzy_score(value, &s.name);
                }
            }
            if score < 10 {
                continue;
            }
        }

        filtered_torrents.push((t, score));
    }

    if let Some(sort_by) = sort {
        filtered_torrents.sort_by(|(a, _), (b, _)| {
            let ord = match sort_by {
                TorrentsPageSort::Kind => a.meta.media_type.cmp(&b.meta.media_type),
                TorrentsPageSort::Category => a
                    .meta
                    .cat
                    .partial_cmp(&b.meta.cat)
                    .unwrap_or(std::cmp::Ordering::Less),
                TorrentsPageSort::Title => a.meta.title.cmp(&b.meta.title),
                TorrentsPageSort::Edition => a
                    .meta
                    .edition
                    .as_ref()
                    .map(|e| e.1)
                    .cmp(&b.meta.edition.as_ref().map(|e| e.1))
                    .then(a.meta.edition.cmp(&b.meta.edition)),
                TorrentsPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                TorrentsPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                TorrentsPageSort::Series => a
                    .meta
                    .series
                    .cmp(&b.meta.series)
                    .then(a.meta.media_type.cmp(&b.meta.media_type)),
                TorrentsPageSort::Language => a.meta.language.cmp(&b.meta.language),
                TorrentsPageSort::Size => a.meta.size.cmp(&b.meta.size),
                TorrentsPageSort::Linker => a.linker.cmp(&b.linker),
                TorrentsPageSort::QbitCategory => a.category.cmp(&b.category),
                TorrentsPageSort::Linked => a.library_path.cmp(&b.library_path),
                TorrentsPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
                TorrentsPageSort::UploadedAt => a.meta.uploaded_at.cmp(&b.meta.uploaded_at),
            };
            if asc { ord.reverse() } else { ord }
        });
    } else if query.is_some() {
        filtered_torrents.sort_by_key(|(_, score)| -*score);
    }

    let total = filtered_torrents.len();
    if page_size_val > 0 && from_val >= total && total > 0 {
        from_val = ((total - 1) / page_size_val) * page_size_val;
    }

    let mut rows: Vec<TorrentsRow> = filtered_torrents
        .into_iter()
        .map(|(t, _)| convert_torrent_row(&t))
        .collect();

    if page_size_val > 0 {
        rows = rows
            .into_iter()
            .skip(from_val)
            .take(page_size_val)
            .collect();
    }

    let abs_url = context
        .config()
        .await
        .audiobookshelf
        .as_ref()
        .map(|abs| abs.url.clone());

    Ok(TorrentsData {
        torrents: rows,
        total,
        from: from_val,
        page_size: page_size_val,
        abs_url,
    })
}

#[server]
pub async fn apply_torrents_action(
    action: TorrentsBulkAction,
    torrent_ids: Vec<String>,
) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    if torrent_ids.is_empty() {
        return Err(ServerFnError::new("No torrents selected"));
    }

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;

    match action {
        TorrentsBulkAction::Clean => {
            let config = context.config().await;
            for id in torrent_ids {
                let Some(torrent) = context
                    .db()
                    .r_transaction()
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .get()
                    .primary::<DbTorrent>(id)
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                else {
                    return Err(ServerFnError::new("Could not find torrent"));
                };
                clean_torrent(&config, context.db(), torrent, true, &context.events)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            }
        }
        TorrentsBulkAction::Refresh => {
            let config = context.config().await;
            let mam = context
                .mam()
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            for id in torrent_ids {
                refresh_mam_metadata(&config, context.db(), &mam, id, &context.events)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            }
        }
        TorrentsBulkAction::RefreshRelink => {
            let config = context.config().await;
            let mam = context
                .mam()
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            for id in torrent_ids {
                refresh_metadata_relink(&config, context.db(), &mam, id, &context.events)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            }
        }
        TorrentsBulkAction::Remove => {
            let (_guard, rw) = context
                .db()
                .rw_async()
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            for id in torrent_ids {
                let Some(torrent) = rw
                    .get()
                    .primary::<DbTorrent>(id)
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                else {
                    return Err(ServerFnError::new("Could not find torrent"));
                };
                rw.remove(torrent)
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            }
            rw.commit().map_err(|e| ServerFnError::new(e.to_string()))?;
        }
    }

    Ok(())
}

#[cfg(feature = "server")]
fn matches_filter(t: &DbTorrent, field: TorrentsPageFilter, value: &str) -> bool {
    match field {
        TorrentsPageFilter::Kind => t.meta.media_type.as_str() == value,
        TorrentsPageFilter::Category => {
            if value.is_empty() {
                t.meta.cat.is_none()
            } else if let Some(cat) = &t.meta.cat {
                let cats = value
                    .split(',')
                    .filter_map(|id| id.parse().ok())
                    .filter_map(OldCategory::from_one_id)
                    .collect::<Vec<_>>();
                cats.contains(cat) || cat.as_str() == value
            } else {
                false
            }
        }
        TorrentsPageFilter::Categories => {
            if value.is_empty() {
                t.meta.categories.is_empty()
            } else {
                value
                    .split(',')
                    .all(|cat| t.meta.categories.iter().any(|c| c.as_str() == cat.trim()))
            }
        }
        TorrentsPageFilter::Flags => {
            if value.is_empty() {
                t.meta.flags.is_none_or(|f| f.0 == 0)
            } else if let Some(flags) = &t.meta.flags {
                let flags = Flags::from_bitfield(flags.0);
                match value {
                    "violence" => flags.violence == Some(true),
                    "explicit" => flags.explicit == Some(true),
                    "some_explicit" => flags.some_explicit == Some(true),
                    "language" => flags.crude_language == Some(true),
                    "abridged" => flags.abridged == Some(true),
                    "lgbt" => flags.lgbt == Some(true),
                    _ => false,
                }
            } else {
                false
            }
        }
        TorrentsPageFilter::Title => t.meta.title == value,
        TorrentsPageFilter::Author => {
            if value.is_empty() {
                t.meta.authors.is_empty()
            } else {
                t.meta.authors.contains(&value.to_string())
            }
        }
        TorrentsPageFilter::Narrator => {
            if value.is_empty() {
                t.meta.narrators.is_empty()
            } else {
                t.meta.narrators.contains(&value.to_string())
            }
        }
        TorrentsPageFilter::Series => {
            if value.is_empty() {
                t.meta.series.is_empty()
            } else {
                t.meta.series.iter().any(|s| s.name == value)
            }
        }
        TorrentsPageFilter::Language => {
            if value.is_empty() {
                t.meta.language.is_none()
            } else {
                t.meta.language == Language::from_str(value).ok()
            }
        }
        TorrentsPageFilter::Filetype => t.meta.filetypes.iter().any(|f| f == value),
        TorrentsPageFilter::Linker => {
            if value.is_empty() {
                t.linker.is_none()
            } else {
                t.linker.as_deref() == Some(value)
            }
        }
        TorrentsPageFilter::QbitCategory => {
            if value.is_empty() {
                t.category.is_none()
            } else {
                t.category.as_deref() == Some(value)
            }
        }
        TorrentsPageFilter::Linked => t.library_path.is_some() == (value == "true"),
        TorrentsPageFilter::LibraryMismatch => {
            if value.is_empty() {
                t.library_mismatch.is_some()
            } else {
                match t.library_mismatch {
                    Some(LibraryMismatch::NewLibraryDir(ref path)) => {
                        value == "new_library" || value == path.to_string_lossy().as_ref()
                    }
                    Some(LibraryMismatch::NewPath(ref path)) => {
                        value == "new_path" || value == path.to_string_lossy().as_ref()
                    }
                    Some(LibraryMismatch::NoLibrary) => value == "no_library",
                    None => false,
                }
            }
        }
        TorrentsPageFilter::ClientStatus => match t.client_status {
            Some(ClientStatus::NotInClient) => value == "not_in_client",
            Some(ClientStatus::RemovedFromTracker) => value == "removed_from_tracker",
            None => false,
        },
        TorrentsPageFilter::Abs => t.meta.ids.contains_key(ids::ABS) == (value == "true"),
        TorrentsPageFilter::Query => true,
        TorrentsPageFilter::Source => match value {
            "mam" => t.meta.source == MetadataSource::Mam,
            "manual" => t.meta.source == MetadataSource::Manual,
            "file" => t.meta.source == MetadataSource::File,
            "match" => t.meta.source == MetadataSource::Match,
            _ => false,
        },
        TorrentsPageFilter::Metadata => {
            if value.is_empty() {
                !t.meta.ids.is_empty()
            } else {
                t.meta.ids.contains_key(value)
                    || t.meta
                        .ids
                        .iter()
                        .any(|(key, id)| key == value || id == value)
            }
        }
    }
}

#[cfg(feature = "server")]
fn convert_torrent_row(t: &DbTorrent) -> TorrentsRow {
    let flags = Flags::from_bitfield(t.meta.flags.map_or(0, |f| f.0));
    let mut flag_values = Vec::new();
    if flags.crude_language == Some(true) {
        flag_values.push("language".to_string());
    }
    if flags.violence == Some(true) {
        flag_values.push("violence".to_string());
    }
    if flags.some_explicit == Some(true) {
        flag_values.push("some_explicit".to_string());
    }
    if flags.explicit == Some(true) {
        flag_values.push("explicit".to_string());
    }
    if flags.abridged == Some(true) {
        flag_values.push("abridged".to_string());
    }
    if flags.lgbt == Some(true) {
        flag_values.push("lgbt".to_string());
    }

    let (cat_name, cat_id) = if let Some(cat) = &t.meta.cat {
        (cat.as_str().to_string(), Some(cat.as_id().to_string()))
    } else {
        ("N/A".to_string(), None)
    };

    let client_status = t.client_status.as_ref().map(|status| match status {
        ClientStatus::RemovedFromTracker => "removed_from_tracker".to_string(),
        ClientStatus::NotInClient => "not_in_client".to_string(),
    });

    let library_mismatch = t.library_mismatch.as_ref().map(|mismatch| match mismatch {
        LibraryMismatch::NewLibraryDir(path) => {
            TorrentLibraryMismatch::NewLibraryDir(path.to_string_lossy().to_string())
        }
        LibraryMismatch::NewPath(path) => {
            TorrentLibraryMismatch::NewPath(path.to_string_lossy().to_string())
        }
        LibraryMismatch::NoLibrary => TorrentLibraryMismatch::NoLibrary,
    });

    TorrentsRow {
        id: t.id.clone(),
        mam_id: t.mam_id,
        meta: TorrentsMeta {
            title: t.meta.title.clone(),
            media_type: t.meta.media_type.as_str().to_string(),
            cat_name,
            cat_id,
            categories: t.meta.categories.clone(),
            flags: flag_values,
            edition: t.meta.edition.as_ref().map(|(edition, _)| edition.clone()),
            authors: t.meta.authors.clone(),
            narrators: t.meta.narrators.clone(),
            series: t
                .meta
                .series
                .iter()
                .map(|series| TorrentsSeries {
                    name: series.name.clone(),
                    entries: series.entries.to_string(),
                })
                .collect(),
            language: t.meta.language.map(|l| l.to_str().to_string()),
            size: t.meta.size.to_string(),
            filetypes: t.meta.filetypes.clone(),
        },
        linker: t.linker.clone(),
        category: t.category.clone(),
        library_path: t
            .library_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        library_mismatch,
        client_status,
        linked: t.library_path.is_some(),
        created_at: format_timestamp_db(&t.created_at),
        uploaded_at: format_timestamp_db(&t.meta.uploaded_at),
        abs_id: t.meta.ids.get(ids::ABS).cloned(),
    }
}

#[cfg(feature = "server")]
fn fuzzy_score(query: &str, target: &str) -> isize {
    FuzzySearch::new(query, target)
        .case_insensitive()
        .best_match()
        .map_or(0, |m: sublime_fuzzy::Match| m.score())
}

fn filter_name(filter: TorrentsPageFilter) -> &'static str {
    match filter {
        TorrentsPageFilter::Kind => "Type",
        TorrentsPageFilter::Category => "Category",
        TorrentsPageFilter::Categories => "Categories",
        TorrentsPageFilter::Flags => "Flags",
        TorrentsPageFilter::Title => "Title",
        TorrentsPageFilter::Author => "Authors",
        TorrentsPageFilter::Narrator => "Narrators",
        TorrentsPageFilter::Series => "Series",
        TorrentsPageFilter::Language => "Language",
        TorrentsPageFilter::Filetype => "Filetypes",
        TorrentsPageFilter::Linker => "Linker",
        TorrentsPageFilter::QbitCategory => "Qbit Category",
        TorrentsPageFilter::Linked => "Linked",
        TorrentsPageFilter::LibraryMismatch => "Library mismatch",
        TorrentsPageFilter::ClientStatus => "Client status",
        TorrentsPageFilter::Abs => "ABS",
        TorrentsPageFilter::Query => "Query",
        TorrentsPageFilter::Source => "Source",
        TorrentsPageFilter::Metadata => "Metadata",
    }
}

fn flag_icon(flag: &str) -> Option<(&'static str, &'static str)> {
    match flag {
        "language" => Some(("/assets/icons/language.png", "Crude Language")),
        "violence" => Some(("/assets/icons/hand.png", "Violence")),
        "some_explicit" => Some((
            "/assets/icons/lipssmall.png",
            "Some Sexually Explicit Content",
        )),
        "explicit" => Some(("/assets/icons/flames.png", "Sexually Explicit Content")),
        "abridged" => Some(("/assets/icons/abridged.png", "Abridged")),
        "lgbt" => Some(("/assets/icons/lgbt.png", "LGBT")),
        _ => None,
    }
}

fn apply_filter(
    filters: &mut Signal<Vec<(TorrentsPageFilter, String)>>,
    field: TorrentsPageFilter,
    value: String,
) {
    let mut next = filters.read().clone();
    next.retain(|(f, _)| *f != field);
    next.push((field, value));
    filters.set(next);
}

#[derive(Clone)]
struct LegacyQueryState {
    query: String,
    sort: Option<TorrentsPageSort>,
    asc: bool,
    filters: Vec<(TorrentsPageFilter, String)>,
    from: usize,
    page_size: usize,
    show: TorrentsPageColumns,
}

impl Default for LegacyQueryState {
    fn default() -> Self {
        Self {
            query: String::new(),
            sort: None,
            asc: false,
            filters: Vec::new(),
            from: 0,
            page_size: 500,
            show: TorrentsPageColumns::default(),
        }
    }
}

#[cfg(feature = "web")]
fn parse_query_enum<T: DeserializeOwned>(value: &str) -> Option<T> {
    serde_json::from_str::<T>(&format!("\"{value}\"")).ok()
}

fn encode_query_enum<T: Serialize>(value: T) -> Option<String> {
    serde_json::to_string(&value)
        .ok()
        .map(|raw| raw.trim_matches('"').to_string())
}

#[cfg(feature = "web")]
fn decode_query_value(value: &str) -> String {
    let replaced = value.replace('+', " ");
    urlencoding::decode(&replaced)
        .map(|s| s.to_string())
        .unwrap_or(replaced)
}

fn show_to_query_value(show: TorrentsPageColumns) -> String {
    let mut values = Vec::new();
    if show.category {
        values.push("category");
    }
    if show.categories {
        values.push("categories");
    }
    if show.flags {
        values.push("flags");
    }
    if show.edition {
        values.push("edition");
    }
    if show.authors {
        values.push("author");
    }
    if show.narrators {
        values.push("narrator");
    }
    if show.series {
        values.push("series");
    }
    if show.language {
        values.push("language");
    }
    if show.size {
        values.push("size");
    }
    if show.filetypes {
        values.push("filetype");
    }
    if show.linker {
        values.push("linker");
    }
    if show.qbit_category {
        values.push("qbit_category");
    }
    if show.path {
        values.push("path");
    }
    if show.created_at {
        values.push("created_at");
    }
    if show.uploaded_at {
        values.push("uploaded_at");
    }
    values.join(",")
}

#[cfg(feature = "web")]
fn show_from_query_value(value: &str) -> TorrentsPageColumns {
    let mut show = TorrentsPageColumns {
        category: false,
        categories: false,
        flags: false,
        edition: false,
        authors: false,
        narrators: false,
        series: false,
        language: false,
        size: false,
        filetypes: false,
        linker: false,
        qbit_category: false,
        path: false,
        created_at: false,
        uploaded_at: false,
    };
    for item in value.split(',') {
        match item {
            "category" => show.category = true,
            "categories" => show.categories = true,
            "flags" => show.flags = true,
            "edition" => show.edition = true,
            "author" => show.authors = true,
            "narrator" => show.narrators = true,
            "series" => show.series = true,
            "language" => show.language = true,
            "size" => show.size = true,
            "filetype" => show.filetypes = true,
            "linker" => show.linker = true,
            "qbit_category" => show.qbit_category = true,
            "path" => show.path = true,
            "created_at" => show.created_at = true,
            "uploaded_at" => show.uploaded_at = true,
            _ => {}
        }
    }
    show
}

fn parse_legacy_query_state() -> LegacyQueryState {
    #[cfg(feature = "web")]
    {
        let mut state = LegacyQueryState::default();
        let Some(window) = web_sys::window() else {
            return state;
        };
        let Ok(search) = window.location().search() else {
            return state;
        };
        let search = search.trim_start_matches('?');
        if search.is_empty() {
            return state;
        }
        for pair in search.split('&') {
            let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
            let key = decode_query_value(raw_key);
            let value = decode_query_value(raw_value);
            match key.as_str() {
                "sort_by" => {
                    state.sort = parse_query_enum::<TorrentsPageSort>(&value);
                }
                "asc" => {
                    state.asc = value == "true";
                }
                "from" => {
                    if let Ok(v) = value.parse::<usize>() {
                        state.from = v;
                    }
                }
                "page_size" => {
                    if let Ok(v) = value.parse::<usize>() {
                        state.page_size = v;
                    }
                }
                "show" => {
                    state.show = show_from_query_value(&value);
                }
                "query" => {
                    state.query = value;
                }
                _ => {
                    if let Some(field) = parse_query_enum::<TorrentsPageFilter>(&key) {
                        state.filters.push((field, value));
                    }
                }
            }
        }
        state
    }
    #[cfg(not(feature = "web"))]
    {
        LegacyQueryState::default()
    }
}

fn build_legacy_query_string(
    query: &str,
    sort: Option<TorrentsPageSort>,
    asc: bool,
    filters: &[(TorrentsPageFilter, String)],
    from: usize,
    page_size: usize,
    show: TorrentsPageColumns,
) -> String {
    let mut params: Vec<(String, String)> = Vec::new();
    if let Some(sort) = sort.and_then(encode_query_enum) {
        params.push(("sort_by".to_string(), sort));
    }
    if asc {
        params.push(("asc".to_string(), "true".to_string()));
    }
    if from > 0 {
        params.push(("from".to_string(), from.to_string()));
    }
    if page_size != 500 {
        params.push(("page_size".to_string(), page_size.to_string()));
    }
    if show != TorrentsPageColumns::default() {
        params.push(("show".to_string(), show_to_query_value(show)));
    }
    if !query.is_empty() {
        params.push(("query".to_string(), query.to_string()));
    }
    for (field, value) in filters {
        if let Some(name) = encode_query_enum(*field) {
            params.push((name, value.clone()));
        }
    }
    params
        .into_iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)))
        .collect::<Vec<_>>()
        .join("&")
}

#[component]
pub fn TorrentsPage() -> Element {
    let mut query_input = use_signal(String::new);
    let mut submitted_query = use_signal(String::new);
    let mut sort = use_signal(|| None::<TorrentsPageSort>);
    let mut asc = use_signal(|| false);
    let mut filters = use_signal(Vec::<(TorrentsPageFilter, String)>::new);
    let mut from = use_signal(|| 0usize);
    let mut page_size = use_signal(|| 500usize);
    let mut show = use_signal(TorrentsPageColumns::default);
    let mut selected = use_signal(BTreeSet::<String>::new);
    let mut status_msg = use_signal(|| None::<(String, bool)>);
    let mut cached = use_signal(|| None::<TorrentsData>);
    let loading_action = use_signal(|| false);
    let mut last_request_key = use_signal(String::new);
    let mut url_init_done = use_signal(|| false);

    let mut torrents_data = match use_server_future(move || async move {
        let mut server_filters = filters.read().clone();
        let query = submitted_query.read().trim().to_string();
        if !query.is_empty() {
            server_filters.push((TorrentsPageFilter::Query, query));
        }
        get_torrents_data(
            *sort.read(),
            *asc.read(),
            server_filters,
            Some(*from.read()),
            Some(*page_size.read()),
            *show.read(),
        )
        .await
    }) {
        Ok(resource) => resource,
        Err(_) => {
            return rsx! {
                div { class: "torrents-page",
                    div { class: "row",
                        h1 { "Torrents" }
                    }
                    p { "Loading torrents..." }
                }
            };
        }
    };

    let value = torrents_data.value();
    let pending = torrents_data.pending();

    {
        let value = value.read();
        if let Some(Ok(data)) = &*value {
            cached.set(Some(data.clone()));
        }
    }

    let data_to_show = {
        let value = value.read();
        match &*value {
            Some(Ok(data)) => Some(data.clone()),
            _ => cached.read().clone(),
        }
    };

    use_effect(move || {
        if *url_init_done.read() {
            return;
        }
        let parsed = parse_legacy_query_state();
        query_input.set(parsed.query.clone());
        submitted_query.set(parsed.query);
        sort.set(parsed.sort);
        asc.set(parsed.asc);
        filters.set(parsed.filters);
        from.set(parsed.from);
        page_size.set(parsed.page_size);
        show.set(parsed.show);
        url_init_done.set(true);
    });

    use_effect(move || {
        if !*url_init_done.read() {
            return;
        }
        let query = submitted_query.read().trim().to_string();
        let sort = *sort.read();
        let asc = *asc.read();
        let filters = filters.read().clone();
        let from = *from.read();
        let page_size = *page_size.read();
        let show = *show.read();

        let query_string =
            build_legacy_query_string(&query, sort, asc, &filters, from, page_size, show);
        let should_restart = *last_request_key.read() != query_string;
        if should_restart {
            last_request_key.set(query_string.clone());
            torrents_data.restart();
        }
    });

    let sort_header = |label: &'static str, key: TorrentsPageSort| {
        let active = *sort.read() == Some(key);
        let arrow = if active {
            if *asc.read() { "↑" } else { "↓" }
        } else {
            ""
        };
        rsx! {
            div { class: "header",
                button {
                    r#type: "button",
                    class: "link",
                    onclick: {
                        let mut sort = sort;
                        let mut asc = asc;
                        let mut from = from;
                        move |_| {
                            if *sort.read() == Some(key) {
                                let next_asc = !*asc.read();
                                asc.set(next_asc);
                            } else {
                                sort.set(Some(key));
                                asc.set(false);
                            }
                            from.set(0);
                        }
                    },
                    "{label}"
                    "{arrow}"
                }
            }
        }
    };

    rsx! {
        div { class: "torrents-page",
            form {
                class: "row",
                onsubmit: move |ev: Event<FormData>| {
                    ev.prevent_default();
                    submitted_query.set(query_input.read().trim().to_string());
                    from.set(0);
                },
                h1 { "Torrents" }
                label {
                    input {
                        r#type: "submit",
                        value: "Search",
                        style: "display: none;",
                    }
                    "Search: "
                    input {
                        r#type: "text",
                        name: "query",
                        value: "{query_input}",
                        oninput: move |ev| query_input.set(ev.value()),
                    }
                    button {
                        r#type: "button",
                        onclick: move |_| {
                            query_input.set(String::new());
                            submitted_query.set(String::new());
                            from.set(0);
                        },
                        "×"
                    }
                }
                div { class: "table_options",
                    div { class: "option_group query",
                        "Columns:"
                        div {
                            label {
                                "Category"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().category,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.category = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Categories"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().categories,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.categories = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Flags"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().flags,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.flags = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Edition"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().edition,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.edition = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Authors"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().authors,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.authors = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Narrators"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().narrators,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.narrators = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Series"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().series,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.series = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Language"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().language,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.language = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Size"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().size,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.size = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Filetypes"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().filetypes,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.filetypes = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Linker"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().linker,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.linker = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Qbit Category"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().qbit_category,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.qbit_category = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Path"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().path,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.path = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Added At"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().created_at,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.created_at = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                            label {
                                "Uploaded At"
                                input {
                                    r#type: "checkbox",
                                    checked: show.read().uploaded_at,
                                    onchange: move |ev| {
                                        let mut next = *show.read();
                                        next.uploaded_at = ev.value() == "true";
                                        show.set(next);
                                    },
                                }
                            }
                        }
                    }
                    div { class: "option_group query",
                        "Page size: "
                        select {
                            value: "{page_size}",
                            onchange: move |ev| {
                                if let Ok(v) = ev.value().parse::<usize>() {
                                    page_size.set(v);
                                    from.set(0);
                                }
                            },
                            option { value: "100", "100" }
                            option { value: "500", "500" }
                            option { value: "1000", "1000" }
                            option { value: "5000", "5000" }
                            option { value: "0", "all" }
                        }
                    }
                }
            }

            if let Some((msg, is_error)) = status_msg.read().as_ref() {
                p { class: if *is_error { "error" } else { "loading-indicator" },
                    "{msg}"
                    button {
                        r#type: "button",
                        style: "margin-left: 10px; cursor: pointer;",
                        onclick: move |_| status_msg.set(None),
                        "⨯"
                    }
                }
            }

            div { class: "option_group query",
                if !submitted_query.read().is_empty() {
                    span { class: "item",
                        "Query: {submitted_query}"
                        button {
                            r#type: "button",
                            onclick: move |_| {
                                submitted_query.set(String::new());
                                query_input.set(String::new());
                                from.set(0);
                            },
                            " ×"
                        }
                    }
                }
                for (field , value) in filters.read().clone() {
                    span { class: "item",
                        "{filter_name(field)}: {value}"
                        button {
                            r#type: "button",
                            onclick: {
                                let value = value.clone();
                                move |_| {
                                    filters.write().retain(|(f, v)| !(*f == field && *v == value));
                                    from.set(0);
                                }
                            },
                            " ×"
                        }
                    }
                }
                if !filters.read().is_empty() || !submitted_query.read().is_empty() {
                    button {
                        r#type: "button",
                        onclick: move |_| {
                            filters.set(Vec::new());
                            submitted_query.set(String::new());
                            query_input.set(String::new());
                            from.set(0);
                        },
                        "Clear filters"
                    }
                }
            }

            if let Some(data) = data_to_show {
                if data.torrents.is_empty() {
                    p {
                        i { "You have no torrents selected by MLM" }
                    }
                } else {
                    div { class: "actions actions_torrent",
                        for action in [
                            TorrentsBulkAction::Refresh,
                            TorrentsBulkAction::RefreshRelink,
                            TorrentsBulkAction::Clean,
                            TorrentsBulkAction::Remove,
                        ]
                        {
                            button {
                                r#type: "button",
                                disabled: *loading_action.read(),
                                onclick: {
                                    let mut loading_action = loading_action;
                                    let mut status_msg = status_msg;
                                    let mut torrents_data = torrents_data;
                                    let mut selected = selected;
                                    move |_| {
                                        let ids: Vec<String> = selected.read().iter().cloned().collect();
                                        if ids.is_empty() {
                                            status_msg.set(Some(("Select at least one torrent".to_string(), true)));
                                            return;
                                        }
                                        loading_action.set(true);
                                        status_msg.set(None);
                                        spawn(async move {
                                            match apply_torrents_action(action, ids).await {
                                                Ok(_) => {
                                                    status_msg
                                                        .set(Some((action.success_label().to_string(), false)));
                                                    selected.set(BTreeSet::new());
                                                    torrents_data.restart();
                                                }
                                                Err(e) => {
                                                    status_msg
                                                        .set(
                                                            Some((format!("{} failed: {e}", action.label()), true)),
                                                        );
                                                }
                                            }
                                            loading_action.set(false);
                                        });
                                    }
                                },
                                "{action.label()}"
                            }
                        }
                    }

                    if pending && cached.read().is_some() {
                        p { class: "loading-indicator", "Refreshing torrent list..." }
                    }
                    div {
                        class: "TorrentsTable table2",
                        style: "--torrents-grid: {show.read().table_grid_template()};",
                        {
                            let all_selected = data
                                .torrents
                                .iter()
                                .all(|torrent| selected.read().contains(&torrent.id));
                            rsx! {
                                div { class: "torrents-grid-row",
                                    div { class: "header",
                                        input {
                                            r#type: "checkbox",
                                            checked: all_selected,
                                            onchange: {
                                                let row_ids = data
                                                    .torrents
                                                    .iter()
                                                    .map(|torrent| torrent.id.clone())
                                                    .collect::<Vec<_>>();
                                                move |ev| {
                                                    if ev.value() == "true" {
                                                        let mut next = selected.read().clone();
                                                        for id in &row_ids {
                                                            next.insert(id.clone());
                                                        }
                                                        selected.set(next);
                                                    } else {
                                                        let mut next = selected.read().clone();
                                                        for id in &row_ids {
                                                            next.remove(id);
                                                        }
                                                        selected.set(next);
                                                    }
                                                }
                                            },
                                        }
                                    }
                                    {sort_header("Type", TorrentsPageSort::Kind)}
                                    if show.read().categories {
                                        div { class: "header", "Categories" }
                                    }
                                    if show.read().flags {
                                        div { class: "header", "Flags" }
                                    }
                                    {sort_header("Title", TorrentsPageSort::Title)}
                                    if show.read().edition {
                                        {sort_header("Edition", TorrentsPageSort::Edition)}
                                    }
                                    if show.read().authors {
                                        {sort_header("Authors", TorrentsPageSort::Authors)}
                                    }
                                    if show.read().narrators {
                                        {sort_header("Narrators", TorrentsPageSort::Narrators)}
                                    }
                                    if show.read().series {
                                        {sort_header("Series", TorrentsPageSort::Series)}
                                    }
                                    if show.read().language {
                                        {sort_header("Language", TorrentsPageSort::Language)}
                                    }
                                    if show.read().size {
                                        {sort_header("Size", TorrentsPageSort::Size)}
                                    }
                                    if show.read().filetypes {
                                        div { class: "header", "Filetypes" }
                                    }
                                    if show.read().linker {
                                        {sort_header("Linker", TorrentsPageSort::Linker)}
                                    }
                                    if show.read().qbit_category {
                                        {sort_header("Qbit Category", TorrentsPageSort::QbitCategory)}
                                    }
                                    {
                                        sort_header(
                                            if show.read().path { "Path" } else { "Linked" },
                                            TorrentsPageSort::Linked,
                                        )
                                    }
                                    if show.read().created_at {
                                        {sort_header("Added At", TorrentsPageSort::CreatedAt)}
                                    }
                                    if show.read().uploaded_at {
                                        {sort_header("Uploaded At", TorrentsPageSort::UploadedAt)}
                                    }
                                    div { class: "header", "" }
                                }
                            }
                        }

                        for torrent in data.torrents.clone() {
                            {
                                let row_id = torrent.id.clone();
                                let row_selected = selected.read().contains(&row_id);
                                rsx! {
                                    div { class: "torrents-grid-row", key: "{row_id}",
                                        div {
                                            input {
                                                r#type: "checkbox",
                                                checked: row_selected,
                                                onchange: {
                                                    let row_id = row_id.clone();
                                                    move |ev| {
                                                        let mut next = selected.read().clone();
                                                        if ev.value() == "true" {
                                                            next.insert(row_id.clone());
                                                        } else {
                                                            next.remove(&row_id);
                                                        }
                                                        selected.set(next);
                                                    }
                                                },
                                            }
                                        }
                                        div {
                                            button {
                                                r#type: "button",
                                                class: "link",
                                                title: "{torrent.meta.cat_name}",
                                                onclick: {
                                                    let value = torrent.meta.media_type.clone();
                                                    move |_| {
                                                        apply_filter(&mut filters, TorrentsPageFilter::Kind, value.clone());
                                                        from.set(0);
                                                    }
                                                },
                                                "{torrent.meta.media_type}"
                                            }
                                            if show.read().category {
                                                if let Some(cat_id) = torrent.meta.cat_id.clone() {
                                                    div {
                                                        button {
                                                            r#type: "button",
                                                            class: "link",
                                                            onclick: {
                                                                let label = cat_id.clone();
                                                                move |_| {
                                                                    apply_filter(&mut filters, TorrentsPageFilter::Category, label.clone());
                                                                    from.set(0);
                                                                }
                                                            },
                                                            "{torrent.meta.cat_name}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().categories {
                                            div {
                                                for category in torrent.meta.categories.clone() {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let category = category.clone();
                                                            move |_| {
                                                                apply_filter(&mut filters, TorrentsPageFilter::Categories, category.clone());
                                                                from.set(0);
                                                            }
                                                        },
                                                        "{category}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().flags {
                                            div {
                                                for flag in torrent.meta.flags.clone() {
                                                    if let Some((src, title)) = flag_icon(&flag) {
                                                        button {
                                                            r#type: "button",
                                                            class: "link",
                                                            onclick: {
                                                                let flag = flag.clone();
                                                                move |_| {
                                                                    apply_filter(&mut filters, TorrentsPageFilter::Flags, flag.clone());
                                                                    from.set(0);
                                                                }
                                                            },
                                                            img {
                                                                class: "flag",
                                                                src: "{src}",
                                                                alt: "{title}",
                                                                title: "{title}",
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div {
                                            button {
                                                r#type: "button",
                                                class: "link",
                                                onclick: {
                                                    let title = torrent.meta.title.clone();
                                                    move |_| {
                                                        apply_filter(&mut filters, TorrentsPageFilter::Title, title.clone());
                                                        from.set(0);
                                                    }
                                                },
                                                "{torrent.meta.title}"
                                            }
                                            if torrent.client_status.as_deref() == Some("removed_from_tracker") {
                                                span {
                                                    class: "warn",
                                                    title: "Torrent is removed from tracker but still seeding",
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: move |_| {
                                                            apply_filter(
                                                                &mut filters,
                                                                TorrentsPageFilter::ClientStatus,
                                                                "removed_from_tracker".to_string(),
                                                            );
                                                            from.set(0);
                                                        },
                                                        "⚠"
                                                    }
                                                }
                                            }
                                            if torrent.client_status.as_deref() == Some("not_in_client") {
                                                span { title: "Torrent is not seeding",
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: move |_| {
                                                            apply_filter(
                                                                &mut filters,
                                                                TorrentsPageFilter::ClientStatus,
                                                                "not_in_client".to_string(),
                                                            );
                                                            from.set(0);
                                                        },
                                                        "ℹ"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().edition {
                                            div { "{torrent.meta.edition.clone().unwrap_or_default()}" }
                                        }
                                        if show.read().authors {
                                            div {
                                                for author in torrent.meta.authors.clone() {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let author = author.clone();
                                                            move |_| {
                                                                apply_filter(&mut filters, TorrentsPageFilter::Author, author.clone());
                                                                from.set(0);
                                                            }
                                                        },
                                                        "{author}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().narrators {
                                            div {
                                                for narrator in torrent.meta.narrators.clone() {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let narrator = narrator.clone();
                                                            move |_| {
                                                                apply_filter(&mut filters, TorrentsPageFilter::Narrator, narrator.clone());
                                                                from.set(0);
                                                            }
                                                        },
                                                        "{narrator}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().series {
                                            div {
                                                for series in torrent.meta.series.clone() {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let series_name = series.name.clone();
                                                            move |_| {
                                                                apply_filter(&mut filters, TorrentsPageFilter::Series, series_name.clone());
                                                                from.set(0);
                                                            }
                                                        },
                                                        if series.entries.is_empty() {
                                                            "{series.name}"
                                                        } else {
                                                            "{series.name} #{series.entries}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().language {
                                            div {
                                                button {
                                                    r#type: "button",
                                                    class: "link",
                                                    onclick: {
                                                        let value = torrent.meta.language.clone().unwrap_or_default();
                                                        move |_| {
                                                            apply_filter(&mut filters, TorrentsPageFilter::Language, value.clone());
                                                            from.set(0);
                                                        }
                                                    },
                                                    "{torrent.meta.language.clone().unwrap_or_default()}"
                                                }
                                            }
                                        }
                                        if show.read().size {
                                            div { "{torrent.meta.size}" }
                                        }
                                        if show.read().filetypes {
                                            div {
                                                for filetype in torrent.meta.filetypes.clone() {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let filetype = filetype.clone();
                                                            move |_| {
                                                                apply_filter(&mut filters, TorrentsPageFilter::Filetype, filetype.clone());
                                                                from.set(0);
                                                            }
                                                        },
                                                        "{filetype}"
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().linker {
                                            div {
                                                button {
                                                    r#type: "button",
                                                    class: "link",
                                                    onclick: {
                                                        let linker = torrent.linker.clone().unwrap_or_default();
                                                        move |_| {
                                                            apply_filter(&mut filters, TorrentsPageFilter::Linker, linker.clone());
                                                            from.set(0);
                                                        }
                                                    },
                                                    "{torrent.linker.clone().unwrap_or_default()}"
                                                }
                                            }
                                        }
                                        if show.read().qbit_category {
                                            div {
                                                button {
                                                    r#type: "button",
                                                    class: "link",
                                                    onclick: {
                                                        let category = torrent.category.clone().unwrap_or_default();
                                                        move |_| {
                                                            apply_filter(
                                                                &mut filters,
                                                                TorrentsPageFilter::QbitCategory,
                                                                category.clone(),
                                                            );
                                                            from.set(0);
                                                        }
                                                    },
                                                    "{torrent.category.clone().unwrap_or_default()}"
                                                }
                                            }
                                        }
                                        if show.read().path {
                                            div {
                                                "{torrent.library_path.clone().unwrap_or_default()}"
                                                if let Some(mismatch) = torrent.library_mismatch.clone() {
                                                    span { class: "warn", title: "{mismatch.title()}",
                                                        button {
                                                            r#type: "button",
                                                            class: "link",
                                                            onclick: move |_| {
                                                                apply_filter(
                                                                    &mut filters,
                                                                    TorrentsPageFilter::LibraryMismatch,
                                                                    mismatch.filter_value().to_string(),
                                                                );
                                                                from.set(0);
                                                            },
                                                            "⚠"
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            div {
                                                if let Some(path) = torrent.library_path.clone() {
                                                    span { title: "{path}",
                                                        button {
                                                            r#type: "button",
                                                            class: "link",
                                                            onclick: {
                                                                let linked = torrent.linked;
                                                                move |_| {
                                                                    apply_filter(&mut filters, TorrentsPageFilter::Linked, linked.to_string());
                                                                    from.set(0);
                                                                }
                                                            },
                                                            "{torrent.linked}"
                                                        }
                                                    }
                                                } else {
                                                    button {
                                                        r#type: "button",
                                                        class: "link",
                                                        onclick: {
                                                            let linked = torrent.linked;
                                                            move |_| {
                                                                apply_filter(&mut filters, TorrentsPageFilter::Linked, linked.to_string());
                                                                from.set(0);
                                                            }
                                                        },
                                                        "{torrent.linked}"
                                                    }
                                                }
                                                if let Some(mismatch) = torrent.library_mismatch.clone() {
                                                    span { class: "warn", title: "{mismatch.title()}",
                                                        button {
                                                            r#type: "button",
                                                            class: "link",
                                                            onclick: move |_| {
                                                                apply_filter(
                                                                    &mut filters,
                                                                    TorrentsPageFilter::LibraryMismatch,
                                                                    mismatch.filter_value().to_string(),
                                                                );
                                                                from.set(0);
                                                            },
                                                            "⚠"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if show.read().created_at {
                                            div { "{torrent.created_at}" }
                                        }
                                        if show.read().uploaded_at {
                                            div { "{torrent.uploaded_at}" }
                                        }
                                        div {
                                            a { href: "/dioxus/torrents/{torrent.id}", "open" }
                                            if let Some(mam_id) = torrent.mam_id {
                                                a {
                                                    href: "https://www.myanonamouse.net/t/{mam_id}",
                                                    target: "_blank",
                                                    "MaM"
                                                }
                                            }
                                            if let (Some(abs_url), Some(abs_id)) = (&data.abs_url, &torrent.abs_id) {
                                                a {
                                                    href: "{abs_url}/audiobookshelf/item/{abs_id}",
                                                    target: "_blank",
                                                    "ABS"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    p { class: "faint",
                        "Showing {data.from} to {data.from + data.torrents.len()} of {data.total}"
                    }
                    Pagination {
                        total: data.total,
                        from: data.from,
                        page_size: data.page_size,
                        on_change: move |new_from| {
                            from.set(new_from);
                        },
                    }
                }
            } else if let Some(Err(e)) = &*value.read() {
                p { class: "error", "Error: {e}" }
            } else {
                p { "Loading torrents..." }
            }
        }
    }
}
