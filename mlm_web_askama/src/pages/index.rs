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

use mlm_core::{
    config::Config,
    lists::{List, get_lists},
    stats::Context,
};
use crate::{AppError, Page, time};

pub async fn index_page(
    State(context): State<Context>,
) -> std::result::Result<Html<String>, AppError> {
    let stats = context.stats.values.lock().await;
    let username = match context.mam.as_ref() {
        Ok(mam) => mam.cached_user_info().await.map(|u| u.username),
        Err(_) => None,
    };
    let config = context.config().await;
    let template = IndexPageTemplate {
        config: config.clone(),
        lists: get_lists(&config),
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
        import_run_at: stats
            .import_run_at
            .iter()
            .map(|(i, time)| (*i, Timestamp::from(*time)))
            .collect(),
        import_result: stats
            .import_result
            .iter()
            .map(|(i, r)| (*i, r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))))
            .collect(),
        torrent_linker_run_at: stats.torrent_linker_run_at.map(Into::into),
        torrent_linker_result: stats
            .torrent_linker_result
            .as_ref()
            .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        folder_linker_run_at: stats.folder_linker_run_at.map(Into::into),
        folder_linker_result: stats
            .folder_linker_result
            .as_ref()
            .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        cleaner_run_at: stats.cleaner_run_at.map(Into::into),
        cleaner_result: stats
            .cleaner_result
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
        "run_torrent_linker" => {
            context.triggers.torrent_linker_tx.send(())?;
        }
        "run_folder_linker" => {
            context.triggers.folder_linker_tx.send(())?;
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
        "run_import" => {
            if let Some(tx) = context.triggers.import_tx.get(
                &form
                    .index
                    .ok_or_else(|| anyhow::Error::msg("Invalid index"))?,
            ) {
                tx.send(())?;
            } else {
                return Err(anyhow::Error::msg("Invalid index").into());
            }
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
    lists: Vec<List>,
    mam_error: Option<String>,
    has_no_qbits: bool,
    username: Option<String>,
    autograbber_run_at: BTreeMap<usize, Timestamp>,
    autograbber_result: BTreeMap<usize, Result<(), String>>,
    import_run_at: BTreeMap<usize, Timestamp>,
    import_result: BTreeMap<usize, Result<(), String>>,
    torrent_linker_run_at: Option<Timestamp>,
    torrent_linker_result: Option<Result<(), String>>,
    folder_linker_run_at: Option<Timestamp>,
    folder_linker_result: Option<Result<(), String>>,
    cleaner_run_at: Option<Timestamp>,
    cleaner_result: Option<Result<(), String>>,
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
