use std::sync::Arc;

use anyhow::{Error, Result};
use askama::Template;
use axum::{
    extract::{OriginalUri, Query, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use native_db::Database;
use serde::Deserialize;
use tracing::info;

use crate::{
    config::Config,
    data::{DatabaseExt as _, SelectedTorrent, Timestamp, Torrent, TorrentCost, TorrentKey},
    mam::{
        enums::SearchTarget,
        meta::normalize_title,
        search::{SearchQuery, Tor},
    },
    stats::Triggers,
    web::{AppError, MaMState, MaMTorrentsTemplate, Page},
};

pub async fn search_page(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    Query(query): Query<SearchPageQuery>,
) -> std::result::Result<Html<String>, AppError> {
    let Ok(mam) = mam.as_ref() else {
        return Err(anyhow::Error::msg("mam_id error").into());
    };
    let result = mam
        .search(&SearchQuery {
            media_info: true,
            tor: Tor {
                target: query.uploader.map(SearchTarget::Uploader),
                text: query.q.clone(),
                ..Default::default()
            },
            ..Default::default()
        })
        .await?;

    let r = db.r_transaction()?;
    let mut torrents = result
        .data
        .into_iter()
        .map(|mam_torrent| {
            let meta = mam_torrent.as_meta()?;
            let torrent = r
                .get()
                .secondary::<Torrent>(TorrentKey::mam_id, meta.mam_id)?;
            let selected_torrent = r.get().primary(mam_torrent.id)?;

            Ok((mam_torrent, meta, torrent, selected_torrent))
        })
        .collect::<Result<Vec<_>>>()?;

    if query.sort == "series" {
        torrents.sort_by(|(_, a, _, _), (_, b, _, _)| {
            a.series
                .cmp(&b.series)
                .then(a.media_type.cmp(&b.media_type))
        })
    }

    let template = SearchPageTemplate {
        query,
        torrents: MaMTorrentsTemplate {
            config: config.search.clone(),
            torrents,
        },
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn search_page_post(
    State((config, db, mam, triggers)): State<(
        Arc<Config>,
        Arc<Database<'static>>,
        MaMState,
        Triggers,
    )>,
    uri: OriginalUri,
    Form(form): Form<SearchPageForm>,
) -> Result<Redirect, AppError> {
    match form.action.as_str() {
        "select" | "wedge" => {
            select_torrent(
                &config,
                &db,
                mam,
                &triggers,
                form.mam_id,
                form.action == "wedge",
            )
            .await?;
        }
        action => {
            eprintln!("unknown action: {action}");
        }
    }

    Ok(Redirect::to(&uri.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct SearchPageForm {
    action: String,
    mam_id: u64,
}

#[derive(Template)]
#[template(path = "pages/search.html")]
struct SearchPageTemplate {
    query: SearchPageQuery,
    torrents: MaMTorrentsTemplate,
}

impl Page for SearchPageTemplate {}

#[derive(Deserialize)]
pub struct SearchPageQuery {
    #[serde(default)]
    q: String,
    #[serde(default)]
    sort: String,
    #[serde(default)]
    uploader: Option<u64>,
}

pub async fn select_torrent(
    config: &Config,
    db: &Database<'_>,
    mam: MaMState,
    triggers: &Triggers,
    mam_id: u64,
    wedge: bool,
) -> Result<(), AppError> {
    let Ok(mam) = mam.as_ref() else {
        return Err(anyhow::Error::msg("mam_id error").into());
    };
    let Some(torrent) = mam.get_torrent_info_by_id(mam_id).await? else {
        return Err(AppError::NotFound);
    };

    let meta = torrent.as_meta()?;
    let tags: Vec<_> = config
        .tags
        .iter()
        .filter(|t| t.filter.matches(&torrent))
        .collect();
    let category = tags.iter().find_map(|t| t.category.clone());
    let tags: Vec<String> = tags.iter().flat_map(|t| t.tags.clone()).collect();
    let cost = if torrent.vip {
        TorrentCost::Vip
    } else if torrent.personal_freeleech {
        TorrentCost::PersonalFreeleech
    } else if torrent.free {
        TorrentCost::GlobalFreeleech
    } else if wedge {
        TorrentCost::UseWedge
    } else {
        TorrentCost::Ratio
    };
    info!(
        "Selecting torrent \"{}\" in format {}, cost: {:?}, with category {:?} and tags {:?}",
        torrent.title, torrent.filetype, cost, category, tags
    );
    {
        let (_guard, rw) = db.rw_async().await?;
        rw.insert(SelectedTorrent {
            mam_id: torrent.id,
            goodreads_id: None,
            hash: None,
            dl_link: torrent
                .dl
                .clone()
                .ok_or_else(|| Error::msg(format!("no dl field for torrent {}", torrent.id)))?,
            unsat_buffer: None,
            wedge_buffer: None,
            cost,
            category,
            tags,
            title_search: normalize_title(&meta.title),
            meta,
            grabber: None,
            created_at: Timestamp::now(),
            started_at: None,
            removed_at: None,
        })?;
        rw.commit()?;
    }
    triggers.downloader_tx.send(())?;

    Ok(())
}
