use std::{
    cell::{Ref, RefCell},
    str::FromStr as _,
    sync::Arc,
};

use anyhow::Result;
use askama::Template;
use axum::{
    extract::{OriginalUri, Query, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use mlm_db::{DatabaseExt as _, Flags, Language, OldCategory, SelectedTorrent, Size, Timestamp};
use mlm_mam::user_data::UserResponse;
use native_db::Database;
use serde::{Deserialize, Serialize};
use tracing::info;

use mlm_core::stats::Context;
use crate::{
    AppError, Page, flag_icons,
    tables::{self, Flex, HidableColumns, Key, SortOn, Sortable},
    time,
};

pub async fn selected_page(
    State(context): State<Context>,
    Query(sort): Query<SortOn<SelectedPageSort>>,
    Query(filter): Query<Vec<(SelectedPageFilter, String)>>,
    Query(show): Query<TorrentsPageColumnsQuery>,
) -> std::result::Result<Html<String>, AppError> {
    let config = context.config().await;
    let show = show.show.unwrap_or_default();

    let mut torrents = context
        .db
        .r_transaction()?
        .scan()
        .primary::<SelectedTorrent>()?
        .all()?
        .filter(|t| show.removed_at || t.as_ref().is_ok_and(|t| t.removed_at.is_none()))
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            for (field, value) in filter.iter() {
                let ok = match field {
                    SelectedPageFilter::Kind => t.meta.media_type.as_str() == value,
                    SelectedPageFilter::Category => {
                        if value.is_empty() {
                            t.meta.cat.is_none()
                        } else if let Some(cat) = &t.meta.cat {
                            let cats = value
                                .split(",")
                                .filter_map(|id| id.parse().ok())
                                .filter_map(OldCategory::from_one_id)
                                .collect::<Vec<_>>();
                            cats.contains(cat) || cat.as_str() == value
                        } else {
                            false
                        }
                    }
                    SelectedPageFilter::Flags => {
                        if value.is_empty() {
                            t.meta.flags.is_none_or(|f| f.0 == 0)
                        } else if let Some(flags) = &t.meta.flags {
                            let flags = Flags::from_bitfield(flags.0);
                            match value.as_str() {
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
                    SelectedPageFilter::Title => &t.meta.title == value,
                    SelectedPageFilter::Author => t.meta.authors.contains(value),
                    SelectedPageFilter::Narrator => t.meta.narrators.contains(value),
                    SelectedPageFilter::Series => t.meta.series.iter().any(|s| &s.name == value),
                    SelectedPageFilter::Language => {
                        if value.is_empty() {
                            t.meta.language.is_none()
                        } else {
                            t.meta.language == Language::from_str(value).ok()
                        }
                    }
                    SelectedPageFilter::Filetype => t.meta.filetypes.contains(value),
                    SelectedPageFilter::Cost => t.cost.as_str() == value,
                    SelectedPageFilter::Grabber => {
                        if value.is_empty() {
                            t.grabber.is_none()
                        } else {
                            t.grabber.as_ref() == Some(value)
                        }
                    }
                    SelectedPageFilter::SortBy => true,
                    SelectedPageFilter::Asc => true,
                    SelectedPageFilter::Show => true,
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
                SelectedPageSort::Kind => a.meta.media_type.cmp(&b.meta.media_type),
                SelectedPageSort::Title => a.meta.title.cmp(&b.meta.title),
                SelectedPageSort::Authors => a.meta.authors.cmp(&b.meta.authors),
                SelectedPageSort::Narrators => a.meta.narrators.cmp(&b.meta.narrators),
                SelectedPageSort::Series => a.meta.series.cmp(&b.meta.series),
                SelectedPageSort::Language => a.meta.language.cmp(&b.meta.language),
                SelectedPageSort::Size => a.meta.size.cmp(&b.meta.size),
                SelectedPageSort::Cost => a.cost.cmp(&b.cost),
                SelectedPageSort::Buffer => a
                    .unsat_buffer
                    .unwrap_or(config.unsat_buffer)
                    .cmp(&b.unsat_buffer.unwrap_or(config.unsat_buffer)),
                SelectedPageSort::Grabber => a.grabber.cmp(&b.grabber),
                SelectedPageSort::CreatedAt => a.created_at.cmp(&b.created_at),
                SelectedPageSort::StartedAt => a.started_at.cmp(&b.started_at),
            };
            if sort.asc { ord.reverse() } else { ord }
        });
    }
    let downloading_size: f64 = context
        .db
        .r_transaction()?
        .scan()
        .primary::<SelectedTorrent>()?
        .all()?
        .filter_map(|t| {
            let t = t.ok()?;
            if t.removed_at.is_none() && t.started_at.is_some() {
                Some(t)
            } else {
                None
            }
        })
        .map(|t| t.meta.size.bytes() as f64)
        .sum();
    let user_info = match context.mam.as_ref() {
        Ok(mam) => mam.user_info().await.ok(),
        _ => None,
    };

    let remaining_buffer = user_info.as_ref().map(|user_info| {
        Size::from_bytes(
            ((user_info.uploaded_bytes - user_info.downloaded_bytes - downloading_size)
                / config.min_ratio) as u64,
        )
    });
    let template = SelectedPageTemplate {
        user_info,
        remaining_buffer,
        unsat_buffer: config.unsat_buffer,
        sort,
        show,
        cols: Default::default(),
        torrents,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn selected_torrents_page_post(
    State(db): State<Arc<Database<'static>>>,
    uri: OriginalUri,
    Form(form): Form<TorrentsPageForm>,
) -> Result<Redirect, AppError> {
    match form.action.as_str() {
        "remove" => {
            for torrent in form.torrents {
                let (_guard, rw) = db.rw_async().await?;
                let Some(mut torrent) = rw.get().primary::<SelectedTorrent>(torrent)? else {
                    return Err(anyhow::Error::msg("Could not find torrent").into());
                };
                if torrent.removed_at.is_none() {
                    torrent.removed_at = Some(Timestamp::now());
                    rw.upsert(torrent)?;
                } else {
                    info!("Hard-removing selected torrent {}", torrent.mam_id);
                    rw.remove(torrent)?;
                }
                rw.commit()?;
            }
        }
        "update" => {
            for torrent in form.torrents {
                let (_guard, rw) = db.rw_async().await?;
                let Some(mut torrent) = rw.get().primary::<SelectedTorrent>(torrent)? else {
                    return Err(anyhow::Error::msg("Could not find torrent").into());
                };
                torrent.unsat_buffer = Some(form.unsats.unwrap_or_default());
                torrent.removed_at = None;
                rw.upsert(torrent)?;
                rw.commit()?;
            }
        }
        action => {
            eprintln!("unknown action: {action}");
        }
    }

    Ok(Redirect::to(&uri.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct TorrentsPageForm {
    action: String,
    unsats: Option<u64>,
    #[serde(default, rename = "torrent")]
    torrents: Vec<u64>,
}

#[derive(Template)]
#[template(path = "pages/selected.html")]
struct SelectedPageTemplate {
    user_info: Option<UserResponse>,
    remaining_buffer: Option<Size>,
    unsat_buffer: u64,
    sort: SortOn<SelectedPageSort>,
    show: TorrentsPageColumns,
    cols: RefCell<Vec<Box<dyn tables::Size>>>,
    torrents: Vec<SelectedTorrent>,
}

impl SelectedPageTemplate {
    fn queued(&self) -> usize {
        self.torrents
            .iter()
            .filter(|t| t.started_at.is_none())
            .count()
    }

    fn downloading(&self) -> usize {
        self.torrents
            .iter()
            .filter(|t| t.started_at.is_some())
            .count()
    }
}

impl Page for SelectedPageTemplate {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectedPageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Cost,
    Buffer,
    Grabber,
    CreatedAt,
    StartedAt,
}

impl Key for SelectedPageSort {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectedPageFilter {
    Kind,
    Category,
    Flags,
    Title,
    Author,
    Narrator,
    Series,
    Language,
    Filetype,
    Cost,
    Grabber,
    // Workaround sort decode failure
    SortBy,
    Asc,
    Show,
}

impl Key for SelectedPageFilter {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
struct TorrentsPageColumns {
    category: bool,
    flags: bool,
    authors: bool,
    narrators: bool,
    series: bool,
    language: bool,
    size: bool,
    filetypes: bool,
    grabber: bool,
    created_at: bool,
    started_at: bool,
    removed_at: bool,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TorrentsPageColumnsQuery {
    show: Option<TorrentsPageColumns>,
}

impl Default for TorrentsPageColumns {
    fn default() -> Self {
        TorrentsPageColumns {
            category: false,
            flags: false,
            authors: true,
            narrators: false,
            series: true,
            language: false,
            size: true,
            filetypes: true,
            grabber: true,
            created_at: true,
            started_at: true,
            removed_at: false,
        }
    }
}

impl TryFrom<String> for TorrentsPageColumns {
    type Error = String;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let mut columns = TorrentsPageColumns {
            category: false,
            flags: false,
            authors: false,
            narrators: false,
            series: false,
            language: false,
            size: false,
            filetypes: false,
            grabber: false,
            created_at: false,
            started_at: false,
            removed_at: false,
        };
        for column in value.split(",") {
            match column {
                "category" => columns.category = true,
                "flags" => columns.flags = true,
                "author" => columns.authors = true,
                "narrator" => columns.narrators = true,
                "series" => columns.series = true,
                "language" => columns.language = true,
                "size" => columns.size = true,
                "filetype" => columns.filetypes = true,
                "grabber" => columns.grabber = true,
                "created_at" => columns.created_at = true,
                "started_at" => columns.started_at = true,
                "removed_at" => columns.removed_at = true,
                "" => {}
                _ => {
                    return Err(format!("Unknown column {column}"));
                }
            }
        }
        Ok(columns)
    }
}

impl Sortable for SelectedPageTemplate {
    type SortKey = SelectedPageSort;

    fn get_current_sort(&self) -> SortOn<Self::SortKey> {
        self.sort
    }
}

impl HidableColumns for SelectedPageTemplate {
    fn add_column(&self, size: Box<dyn tables::Size>) {
        self.cols.borrow_mut().push(size);
    }

    fn get_columns(&self) -> Ref<'_, Vec<Box<dyn tables::Size>>> {
        self.cols.borrow()
    }
}
