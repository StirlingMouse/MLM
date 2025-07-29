use std::sync::Arc;

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
    config::Config,
    data::Timestamp,
    mam::MaM,
    stats::{Stats, Triggers},
    web::{AppError, time},
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
        mam_error: mam.as_ref().as_ref().err().map(|e| format!("{e}")),
        has_no_qbits: config.qbittorrent.is_empty(),
        username,
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
        downloader_run_at: stats.downloader_run_at.map(Into::into),
        downloader_result: stats
            .downloader_result
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
            triggers.search_tx.send(())?;
        }
        "run_goodreads" => {
            triggers.goodreads_tx.send(())?;
        }
        "run_downloader" => {
            triggers.downloader_tx.send(())?;
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
    mam_error: Option<String>,
    has_no_qbits: bool,
    username: Option<String>,
    autograbber_run_at: Option<Timestamp>,
    autograbber_result: Option<Result<(), String>>,
    linker_run_at: Option<Timestamp>,
    linker_result: Option<Result<(), String>>,
    cleaner_run_at: Option<Timestamp>,
    cleaner_result: Option<Result<(), String>>,
    goodreads_run_at: Option<Timestamp>,
    goodreads_result: Option<Result<(), String>>,
    downloader_run_at: Option<Timestamp>,
    downloader_result: Option<Result<(), String>>,
}

#[derive(Debug, Deserialize)]
pub struct IndexPageForm {
    action: String,
}
