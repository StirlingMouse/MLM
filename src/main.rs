#![windows_subsystem = "windows"]

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

use std::{
    env,
    fs::{self, create_dir_all},
    io,
    path::PathBuf,
    process,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use autograbber::{grab_selected_torrents, run_autograbbers};
use cleaner::run_library_cleaner;
use dirs::{config_dir, data_local_dir};
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
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_panic::panic_hook;
use tracing_subscriber::{
    EnvFilter, Layer as _, fmt::time::LocalTime, layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
};
use web::start_webserver;

use crate::{config::Config, linker::link_torrents_to_library, mam::MaM, qbittorrent::QbitError};

#[tokio::main]
async fn main() {
    if let Err(err) = app_main().await {
        error!("AppError: {err:?}");
        eprintln!("{:?}", err);
        process::exit(1);
    }
}

async fn app_main() -> Result<()> {
    let log_dir = env::var("MLM_LOG_DIR")
        .map(|path| {
            if path.is_empty() {
                None
            } else {
                Some(PathBuf::from(path))
            }
        })
        .unwrap_or_else(|_| {
            #[cfg(all(debug_assertions, not(windows)))]
            return None;
            #[allow(unused)]
            Some(
                data_local_dir()
                    .map(|d| d.join("MLM").join("logs"))
                    .unwrap_or_else(|| "logs".into()),
            )
        });

    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stderr);

    let file_layer = log_dir
        .as_ref()
        .map(|log_dir| {
            Result::<_>::Ok(
                tracing_subscriber::fmt::layer().pretty().with_writer(
                    RollingFileAppender::builder()
                        .rotation(Rotation::DAILY)
                        .filename_prefix("mlm")
                        .filename_suffix("log")
                        .build(log_dir)?,
                ),
            )
        })
        .transpose()?;

    tracing_subscriber::registry()
        .with(
            stderr_layer.with_timer(LocalTime::rfc_3339()).with_filter(
                EnvFilter::builder()
                    .with_default_directive("mlm=trace".parse()?)
                    .with_env_var("MLM_LOG")
                    .from_env_lossy(),
            ),
        )
        .with(file_layer.map(|file_layer| {
            file_layer
                .with_timer(LocalTime::rfc_3339())
                .with_ansi(false)
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive("mlm=trace".parse().unwrap())
                        .with_env_var("MLM_LOG")
                        .from_env_lossy(),
                )
        }))
        .try_init()?;
    std::panic::set_hook(Box::new(panic_hook));

    let config_file = env::var("MLM_CONFIG_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            #[cfg(debug_assertions)]
            return "config.toml".into();
            #[allow(unused)]
            config_dir()
                .map(|d| d.join("MLM").join("config.toml"))
                .unwrap_or_else(|| "config.toml".into())
        });
    let database_file = env::var("MLM_DB_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            #[cfg(debug_assertions)]
            return "data.db".into();
            #[allow(unused)]
            data_local_dir()
                .map(|d| d.join("MLM").join("data.db"))
                .unwrap_or_else(|| "data.db".into())
        });
    if !config_file.exists() {
        if let Some(dir) = config_file.parent() {
            create_dir_all(dir)?;
        }
        let default_config = r#"mam_id = """#;
        fs::write(&config_file, default_config)?;
    }
    if !database_file.exists() {
        if let Some(dir) = database_file.parent() {
            create_dir_all(dir)?;
        }
    }
    let config: Config = Figment::new()
        .merge(Toml::file_exact(&config_file))
        .merge(Env::prefixed("MLM_CONF_"))
        .extract()?;
    let config = Arc::new(config);

    info!("config: {config:#?}");

    let db = native_db::Builder::new().create(&data::MODELS, database_file)?;
    data::migrate(&db)?;
    // export_db(&db)?;
    // return Ok(());
    let db = Arc::new(db);

    #[cfg(target_family = "windows")]
    let _tray = tray::start_tray_icon(log_dir, config_file, config.clone())?;

    let stats: Arc<Mutex<Stats>> = Default::default();

    let (search_tx, mut search_rx) = watch::channel(());
    let (linker_tx, linker_rx) = watch::channel(());
    let (goodreads_tx, mut goodreads_rx) = watch::channel(());
    let (downloader_tx, mut downloader_rx) = watch::channel(());

    let mam = if config.mam_id.is_empty() {
        Err(anyhow::Error::msg("No mam_id set"))
    } else {
        MaM::new(&config, db.clone()).await.map(Arc::new)
    };
    if let Ok(mam) = &mam {
        {
            let config = config.clone();
            let db = db.clone();
            let mam = mam.clone();
            let stats = stats.clone();
            tokio::spawn(async move {
                if let Some(qbit_conf) = config.qbittorrent.first() {
                    let mut qbit: Option<qbit::Api> = None;
                    loop {
                        if downloader_rx.changed().await.is_err() {
                            break;
                        }
                        if qbit.is_none() {
                            match qbit::Api::login(
                                &qbit_conf.url,
                                &qbit_conf.username,
                                &qbit_conf.password,
                            )
                            .await
                            .map_err(QbitError)
                            {
                                Ok(q) => qbit = Some(q),
                                Err(err) => {
                                    error!("Error logging in to qbit {}: {err}", qbit_conf.url);
                                    let mut stats = stats.lock().await;
                                    stats.downloader_run_at = Some(OffsetDateTime::now_utc());
                                    stats.downloader_result = Some(Err(err.into()));
                                }
                            };
                        }
                        let Some(qbit) = &qbit else {
                            continue;
                        };
                        {
                            let mut stats = stats.lock().await;
                            stats.downloader_run_at = Some(OffsetDateTime::now_utc());
                            stats.downloader_result = None;
                        }
                        let result = grab_selected_torrents(&config, &db, qbit, &mam)
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
    }

    let triggers = Triggers {
        search_tx,
        linker_tx,
        goodreads_tx,
        downloader_tx,
    };

    start_webserver(config, db, stats, Arc::new(mam), triggers).await?;

    Ok(())
}
