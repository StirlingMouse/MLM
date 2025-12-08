use std::{collections::BTreeMap, sync::Arc};

use anyhow::Result;
use askama::Template;
use axum::{
    extract::{OriginalUri, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::{
    config::{Config, TorrentFilter},
    data::Timestamp,
    mam::api::MaM,
    stats::{Stats, Triggers},
    web::{AppError, Page, time},
};

pub async fn index_page(
    State((config, stats, mam)): State<(
        Arc<Config>,
        Arc<Mutex<Stats>>,
        Arc<Result<Arc<MaM<'static>>>>,
    )>,
) -> std::result::Result<Html<String>, AppError> {
    let stats = stats.lock().await;
    let username = match mam.as_ref() {
        Ok(mam) => mam.cached_user_info().await.map(|u| u.username),
        Err(_) => None,
    };
    let template = IndexPageTemplate {
        config: config.clone(),
        mam_error: mam.as_ref().as_ref().err().map(|e| format!("{e}")),
        has_no_qbits: config.qbittorrent.is_empty(),
        username,
        autograbber_run_at: stats
            .autograbber_run_at
            .iter()
            .map(|(i, time)| (*i, Timestamp::from(*time)))
            .collect(),
        autograbber_result: stats
            .autograbber_result
            .iter()
            .map(|(i, r)| (*i, r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))))
            .collect(),
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
        downloader_run_at: stats.downloader_run_at.map(Into::into),
        downloader_result: stats
            .downloader_result
            .as_ref()
            .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        audiobookshelf_run_at: stats.audiobookshelf_run_at.map(Into::into),
        audiobookshelf_result: stats
            .audiobookshelf_result
            .as_ref()
            .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn index_page_post(
    State(triggers): State<Triggers>,
    uri: OriginalUri,
    Form(form): Form<IndexPageForm>,
) -> Result<Redirect, AppError> {
    match form.action.as_str() {
        "run_linker" => {
            triggers.linker_tx.send(())?;
        }
        "run_search" => {
            if let Some(tx) = triggers.search_tx.get(
                &form
                    .index
                    .ok_or_else(|| anyhow::Error::msg("Invalid index"))?,
            ) {
                tx.send(())?;
            } else {
                return Err(anyhow::Error::msg("Invalid index").into());
            }
        }
        "run_goodreads" => {
            triggers.goodreads_tx.send(())?;
        }
        "run_downloader" => {
            triggers.downloader_tx.send(())?;
        }
        "run_abs_matcher" => {
            triggers.audiobookshelf_tx.send(())?;
        }
        action => {
            eprintln!("unknown action: {action}");
        }
    }

    Ok(Redirect::to(&uri.to_string()))
}

#[derive(Template)]
#[template(path = "pages/index.html")]
struct IndexPageTemplate {
    config: Arc<Config>,
    mam_error: Option<String>,
    has_no_qbits: bool,
    username: Option<String>,
    autograbber_run_at: BTreeMap<usize, Timestamp>,
    autograbber_result: BTreeMap<usize, Result<(), String>>,
    linker_run_at: Option<Timestamp>,
    linker_result: Option<Result<(), String>>,
    cleaner_run_at: Option<Timestamp>,
    cleaner_result: Option<Result<(), String>>,
    goodreads_run_at: Option<Timestamp>,
    goodreads_result: Option<Result<(), String>>,
    downloader_run_at: Option<Timestamp>,
    downloader_result: Option<Result<(), String>>,
    audiobookshelf_run_at: Option<Timestamp>,
    audiobookshelf_result: Option<Result<(), String>>,
}

impl Page for IndexPageTemplate {}

#[derive(Debug, Deserialize)]
pub struct IndexPageForm {
    action: String,
    index: Option<usize>,
}

impl TorrentFilter {
    fn display_name(&self, i: usize) -> String {
        self.name.clone().unwrap_or_else(|| format!("{i}"))
    }
}
