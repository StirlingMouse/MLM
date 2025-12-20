use std::{collections::BTreeMap, convert::Infallible, sync::Arc, time::Duration};

use anyhow::Result;
use askama::Template;
use axum::{
    extract::{OriginalUri, State},
    response::{
        Html, Redirect, Sse,
        sse::{Event, KeepAlive},
    },
};
use axum_extra::extract::Form;
use futures::Stream;
use mlm_db::Timestamp;
use serde::Deserialize;
use tokio_stream::{StreamExt as _, wrappers::WatchStream};

use crate::{
    config::{Config, TorrentFilter},
    stats::Context,
    web::{AppError, Page, time},
};

pub async fn index_page(
    State(context): State<Context>,
) -> std::result::Result<Html<String>, AppError> {
    let stats = context.stats.values.lock().await;
    let username = match context.mam.as_ref() {
        Ok(mam) => mam.cached_user_info().await.map(|u| u.username),
        Err(_) => None,
    };
    let config = context.config.lock().await;
    let template = IndexPageTemplate {
        config: config.clone(),
        mam_error: context.mam.as_ref().as_ref().err().map(|e| format!("{e}")),
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

pub async fn stats_updates(
    State(context): State<Context>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = WatchStream::new(context.stats.updates())
        .map(|time| Ok(Event::default().data(time.to_string())));
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}

pub async fn index_page_post(
    State(context): State<Context>,
    uri: OriginalUri,
    Form(form): Form<IndexPageForm>,
) -> Result<Redirect, AppError> {
    match form.action.as_str() {
        "run_linker" => {
            context.triggers.linker_tx.send(())?;
        }
        "run_search" => {
            if let Some(tx) = context.triggers.search_tx.get(
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
            context.triggers.goodreads_tx.send(())?;
        }
        "run_downloader" => {
            context.triggers.downloader_tx.send(())?;
        }
        "run_abs_matcher" => {
            context.triggers.audiobookshelf_tx.send(())?;
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
