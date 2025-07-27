mod autograbber;
mod cleaner;
mod config;
mod config_impl;
mod data;
mod data_impl;
mod exporter;
mod goodreads;
mod linker;
mod logging;
mod mam;
mod mam_enums;
mod qbittorrent;
mod stats;
#[cfg(target_family = "windows")]
mod tray;
mod web;

use std::{env, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use autograbber::{grab_selected_torrents, run_autograbbers};
use cleaner::run_library_cleaner;
use exporter::export_db;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use goodreads::run_goodreads_import;
use stats::{Stats, Triggers};
use time::OffsetDateTime;
use tokio::{
    select,
    sync::{Mutex, watch},
    time::sleep,
};
use tracing::{error, info};
#[cfg(target_family = "windows")]
use tray::start_tray_icon;
use web::start_webserver;

use crate::{config::Config, linker::link_torrents_to_library, mam::MaM, qbittorrent::QbitError};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config_file = env::var("CONFIG_FILE").unwrap_or("config.toml".to_owned());
    let database_file = env::var("DB_FILE").unwrap_or("data.db".to_owned());
    let config: Config = Figment::new()
        .merge(Toml::file(&config_file))
        .merge(Env::prefixed("MLM_"))
        .extract()?;
    let config = Arc::new(config);

    info!("config: {config:#?}");

    let db = native_db::Builder::new().create(&data::MODELS, database_file)?;
    data::migrate(&db)?;
    // export_db(&db)?;
    // return Ok(());
    let db = Arc::new(db);

    let mam = MaM::new(&config, db.clone()).await?;
    let mam = Arc::new(mam);
    let stats: Arc<Mutex<Stats>> = Default::default();

    #[cfg(target_family = "windows")]
    let _tray = start_tray_icon(config_file, config.clone())?;

    let (search_tx, mut search_rx) = watch::channel(());
    let (linker_tx, linker_rx) = watch::channel(());
    let (goodreads_tx, mut goodreads_rx) = watch::channel(());
    let (downloader_tx, mut downloader_rx) = watch::channel(());

    if let Some(qbit_conf) = config.qbittorrent.first() {
        let config = config.clone();
        let db = db.clone();
        let mam = mam.clone();
        let stats = stats.clone();
        let qbit = qbit::Api::login(&qbit_conf.url, &qbit_conf.username, &qbit_conf.password)
            .await
            .map_err(QbitError)?;
        tokio::spawn(async move {
            loop {
                if downloader_rx.changed().await.is_err() {
                    break;
                }
                {
                    let mut stats = stats.lock().await;
                    stats.downloader_run_at = Some(OffsetDateTime::now_utc());
                    stats.downloader_result = None;
                }
                let result = grab_selected_torrents(&config, &db, &qbit, &mam)
                    .await
                    .context("grab_selected_torrents");

                if let Err(err) = &result {
                    error!("Error grabbing selected torrents: {err:?}");
                }
                {
                    let mut stats = stats.lock().await;
                    stats.downloader_result = Some(result);
                }
            }
        });
    }

    if !config.autograbs.is_empty() {
        let config = config.clone();
        let db = db.clone();
        let mam = mam.clone();
        let downloader_tx = downloader_tx.clone();
        let stats = stats.clone();
        tokio::spawn(async move {
            loop {
                {
                    let mut stats = stats.lock().await;
                    stats.autograbber_run_at = Some(OffsetDateTime::now_utc());
                    stats.autograbber_result = None;
                }
                let result = run_autograbbers(
                    config.clone(),
                    db.clone(),
                    mam.clone(),
                    downloader_tx.clone(),
                )
                .await
                .context("autograbbers");
                if let Err(err) = &result {
                    error!("Error running autograbbers: {err:?}");
                }
                {
                    let mut stats = stats.lock().await;
                    stats.autograbber_result = Some(result);
                }
                select! {
                    () = sleep(Duration::from_secs(60 * config.search_interval)) => {},
                    result = search_rx.changed() => {
                        if let Err(err) = result {
                            error!("Error listening on search_rx: {err:?}");
                            let mut stats = stats.lock().await;
                            stats.autograbber_result = Some(Err(err.into()));
                        }
                    },
                }
            }
        });
    }

    if !config.goodreads_lists.is_empty() {
        let config = config.clone();
        let db = db.clone();
        let mam = mam.clone();
        let stats = stats.clone();
        let downloader_tx = downloader_tx.clone();
        tokio::spawn(async move {
            loop {
                {
                    let mut stats = stats.lock().await;
                    stats.goodreads_run_at = Some(OffsetDateTime::now_utc());
                    stats.goodreads_result = None;
                }
                let result = run_goodreads_import(
                    config.clone(),
                    db.clone(),
                    mam.clone(),
                    downloader_tx.clone(),
                )
                .await
                .context("goodreads_import");
                if let Err(err) = &result {
                    error!("Error running goodreads import: {err:?}");
                }
                {
                    let mut stats = stats.lock().await;
                    stats.goodreads_result = Some(result);
                }
                select! {
                    () = sleep(Duration::from_secs(60 * config.goodreads_interval)) => {},
                    result = goodreads_rx.changed() => {
                        if let Err(err) = result {
                            error!("Error listening on goodreads_rx: {err:?}");
                            let mut stats = stats.lock().await;
                            stats.goodreads_result = Some(Err(err.into()));
                        }
                    },
                }
            }
        });
    }

    {
        for qbit_conf in config.qbittorrent.clone() {
            let config = config.clone();
            let db = db.clone();
            let mam = mam.clone();
            let stats = stats.clone();
            let mut linker_rx = linker_rx.clone();
            tokio::spawn(async move {
                let qbit = match qbit::Api::login(
                    &qbit_conf.url,
                    &qbit_conf.username,
                    &qbit_conf.password,
                )
                .await
                .map_err(QbitError)
                {
                    Ok(qbit) => qbit,
                    Err(err) => {
                        error!("Error logging in to qbit {}: {err}", qbit_conf.url);
                        return;
                    }
                };
                loop {
                    {
                        let mut stats = stats.lock().await;
                        stats.linker_run_at = Some(OffsetDateTime::now_utc());
                        stats.linker_result = None;
                    }
                    let result = link_torrents_to_library(
                        config.clone(),
                        db.clone(),
                        (&qbit_conf, &qbit),
                        mam.clone(),
                    )
                    .await
                    .context("link_torrents_to_library");
                    if let Err(err) = &result {
                        error!("Error running linker: {err:?}");
                    }
                    {
                        let mut stats = stats.lock().await;
                        stats.linker_result = Some(result);
                        stats.cleaner_run_at = Some(OffsetDateTime::now_utc());
                        stats.cleaner_result = None;
                    }
                    let result = run_library_cleaner(config.clone(), db.clone())
                        .await
                        .context("library_cleaner");
                    if let Err(err) = &result {
                        error!("Error running library_cleaner: {err:?}");
                    }
                    {
                        let mut stats = stats.lock().await;
                        stats.cleaner_result = Some(result);
                    }
                    select! {
                        () = sleep(Duration::from_secs(60 * config.link_interval)) => {},
                        result = linker_rx.changed() => {
                            if let Err(err) = result {
                                error!("Error listening on link_rx: {err:?}");
                                let mut stats = stats.lock().await;
                                stats.linker_result = Some(Err(err.into()));
                            }
                        },
                    }
                }
            });
        }
    }

    let triggers = Triggers {
        search_tx,
        linker_tx,
        goodreads_tx,
        downloader_tx,
    };

    start_webserver(config, db, stats, mam, triggers).await?;

    Ok(())
}
