use dioxus::prelude::*;
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

mod components;
mod query;

pub use components::TorrentsPage;
