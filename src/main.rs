#![windows_subsystem = "windows"]

mod audiobookshelf;
mod autograbber;
mod cleaner;
mod config;
mod config_impl;
mod data;
mod exporter;
mod goodreads;
mod linker;
mod logging;
mod mam;
mod qbittorrent;
mod snatchlist;
mod stats;
mod web;
#[cfg(target_family = "windows")]
mod windows;

use std::{
    collections::BTreeMap,
    env,
    fs::{self, create_dir_all},
    io,
    path::PathBuf,
    process,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use audiobookshelf::match_torrents_to_abs;
use autograbber::{grab_selected_torrents, run_autograbber};
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
use tracing::error;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    EnvFilter, Layer as _, fmt::time::LocalTime, layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
};
use web::start_webserver;

use crate::{
    config::Config, linker::link_torrents_to_library, mam::api::MaM,
    snatchlist::run_snatchlist_search,
};

#[tokio::main]
async fn main() {
    if let Err(err) = app_main().await {
        #[cfg(target_family = "windows")]
        windows::error_window::ErrorWindow::create_and_run(
            "MLM App Error".to_string(),
            err.to_string(),
            None,
        )
        .unwrap();
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
            Result::<_, anyhow::Error>::Ok(
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
                    .with_default_directive("mlm=debug".parse()?)
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
                        .with_default_directive("mlm=debug".parse().unwrap())
                        .with_env_var("MLM_LOG")
                        .from_env_lossy(),
                )
        }))
        .try_init()?;
    #[cfg(target_family = "windows")]
    std::panic::set_hook(Box::new(tracing_panic::panic_hook));

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
    if !database_file.exists()
        && let Some(dir) = database_file.parent()
    {
        create_dir_all(dir)?;
    }
    let config: Result<Config, _> = Figment::new()
        .merge(Toml::file_exact(&config_file))
        .merge(Env::prefixed("MLM_CONF_"))
        .extract();
    #[cfg(target_family = "windows")]
    if let Err(err) = &config {
        windows::error_window::ErrorWindow::create_and_run(
            "MLM Config Error".to_string(),
            err.to_string(),
            Some(config_file.clone()),
        )
        .unwrap();
        return Ok(());
    }
    let config = config?;
    let config = Arc::new(config);

    let db = native_db::Builder::new().create(&data::MODELS, database_file)?;
    data::migrate(&db)?;
    // export_db(&db)?;
    // return Ok(());
    let db = Arc::new(db);

    #[cfg(target_family = "windows")]
    let _tray = windows::tray::start_tray_icon(log_dir, config_file.clone(), config.clone())?;

    let stats: Arc<Mutex<Stats>> = Default::default();

    let (mut search_tx, mut search_rx) = (BTreeMap::new(), BTreeMap::new());
    let (linker_tx, linker_rx) = watch::channel(());
    let (goodreads_tx, mut goodreads_rx) = watch::channel(());
    let (downloader_tx, mut downloader_rx) = watch::channel(());
    let (audiobookshelf_tx, mut audiobookshelf_rx) = watch::channel(());

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
                    loop {
                        let mut qbit: Option<qbit::Api> = None;
                        if downloader_rx.changed().await.is_err() {
                            break;
                        }
                        if qbit.is_none() {
                            match qbit::Api::new_login_username_password(
                                &qbit_conf.url,
                                &qbit_conf.username,
                                &qbit_conf.password,
                            )
                            .await
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

        for (i, grab) in config.autograbs.iter().enumerate() {
            let config = config.clone();
            let db = db.clone();
            let mam = mam.clone();
            let downloader_tx = downloader_tx.clone();
            let (tx, mut rx) = watch::channel(());
            search_tx.insert(i, tx);
            search_rx.insert(i, rx.clone());
            let stats = stats.clone();
            let grab = Arc::new(grab.clone());
            tokio::spawn(async move {
                loop {
                    let interval = grab.search_interval.unwrap_or(config.search_interval);
                    if interval > 0 {
                        select! {
                            () = sleep(Duration::from_secs(60 * grab.search_interval.unwrap_or(config.search_interval))) => {},
                            result = rx.changed() => {
                                if let Err(err) = result {
                                    error!("Error listening on search_rx: {err:?}");
                                    let mut stats = stats.lock().await;
                                    stats.autograbber_result.insert(i, Err(err.into()));
                                }
                            },
                        }
                    } else {
                        let result = rx.changed().await;
                        if let Err(err) = result {
                            error!("Error listening on search_rx: {err:?}");
                            let mut stats = stats.lock().await;
                            stats.autograbber_result.insert(i, Err(err.into()));
                        }
                    }
                    {
                        let mut stats = stats.lock().await;
                        stats
                            .autograbber_run_at
                            .insert(i, OffsetDateTime::now_utc());
                        stats.autograbber_result.remove(&i);
                    }
                    let result = run_autograbber(
                        config.clone(),
                        db.clone(),
                        mam.clone(),
                        downloader_tx.clone(),
                        i,
                        grab.clone(),
                    )
                    .await
                    .context("autograbbers");
                    if let Err(err) = &result {
                        error!("Error running autograbbers: {err:?}");
                    }
                    {
                        let mut stats = stats.lock().await;
                        stats.autograbber_result.insert(i, result);
                    }
                }
            });
        }

        for (i, grab) in config.snatchlist.iter().enumerate() {
            let i = i + config.autograbs.len();
            let config = config.clone();
            let db = db.clone();
            let mam = mam.clone();
            let (tx, mut rx) = watch::channel(());
            search_tx.insert(i, tx);
            search_rx.insert(i, rx.clone());
            let stats = stats.clone();
            let grab = Arc::new(grab.clone());
            tokio::spawn(async move {
                loop {
                    let interval = grab.search_interval.unwrap_or(config.search_interval);
                    if interval > 0 {
                        select! {
                            () = sleep(Duration::from_secs(60 * interval)) => {},
                            result = rx.changed() => {
                                if let Err(err) = result {
                                    error!("Error listening on search_rx for snatchlist: {err:?}");
                                    let mut stats = stats.lock().await;
                                    stats.autograbber_result.insert(i, Err(err.into()));
                                }
                            },
                        }
                    } else {
                        let result = rx.changed().await;
                        if let Err(err) = result {
                            error!("Error listening on search_rx for snatchlist: {err:?}");
                            let mut stats = stats.lock().await;
                            stats.autograbber_result.insert(i, Err(err.into()));
                        }
                    }
                    {
                        let mut stats = stats.lock().await;
                        stats
                            .autograbber_run_at
                            .insert(i, OffsetDateTime::now_utc());
                        stats.autograbber_result.remove(&i);
                    }
                    let result = run_snatchlist_search(
                        config.clone(),
                        db.clone(),
                        mam.clone(),
                        i,
                        grab.clone(),
                    )
                    .await
                    .context("snatchlist_search");
                    if let Err(err) = &result {
                        error!("Error running snatchlist_search: {err:?}");
                    }
                    {
                        let mut stats = stats.lock().await;
                        stats.autograbber_result.insert(i, result);
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
                    loop {
                        let qbit = match qbit::Api::new_login_username_password(
                            &qbit_conf.url,
                            &qbit_conf.username,
                            &qbit_conf.password,
                        )
                        .await
                        {
                            Ok(qbit) => qbit,
                            Err(err) => {
                                error!("Error logging in to qbit {}: {err}", qbit_conf.url);
                                let mut stats = stats.lock().await;
                                stats.linker_run_at = Some(OffsetDateTime::now_utc());
                                stats.linker_result = Some(Err(anyhow::Error::msg(format!(
                                    "Error logging in to qbit {}: {err}",
                                    qbit_conf.url,
                                ))));
                                return;
                            }
                        };
                        select! {
                            () = sleep(Duration::from_secs(60 * config.link_interval)) => {},
                            result = linker_rx.changed() => {
                                if let Err(err) = result {
                                    error!("Error listening on link_rx: {err:?}");
                                    let mut stats = stats.lock().await;
                                    stats.linker_run_at = Some(OffsetDateTime::now_utc());
                                    stats.linker_result = Some(Err(err.into()));
                                }
                            },
                        }
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
                    }
                });
            }
        }
    }

    if let Some(config) = &config.audiobookshelf {
        let config = config.clone();
        let db = db.clone();
        let stats = stats.clone();
        tokio::spawn(async move {
            loop {
                select! {
                    () = sleep(Duration::from_secs(60 * config.interval)) => {},
                    result = audiobookshelf_rx.changed() => {
                        if let Err(err) = result {
                            error!("Error listening on audiobookshelf_rx: {err:?}");
                            let mut stats = stats.lock().await;
                            stats.audiobookshelf_result = Some(Err(err.into()));
                        }
                    },
                }
                {
                    let mut stats = stats.lock().await;
                    stats.audiobookshelf_run_at = Some(OffsetDateTime::now_utc());
                    stats.audiobookshelf_result = None;
                }
                let result = match_torrents_to_abs(&config, db.clone())
                    .await
                    .context("audiobookshelf_matcher");
                if let Err(err) = &result {
                    error!("Error running audiobookshelf matcher: {err:?}");
                }
                {
                    let mut stats = stats.lock().await;
                    stats.audiobookshelf_result = Some(result);
                }
            }
        });
    }

    let triggers = Triggers {
        search_tx,
        linker_tx,
        goodreads_tx,
        downloader_tx,
        audiobookshelf_tx,
    };

    let result = start_webserver(config.clone(), db, stats, Arc::new(mam), triggers).await;

    #[cfg(target_family = "windows")]
    if let Err(err) = &result {
        windows::error_window::ErrorWindow::create_and_run(
            "MLM Webserver Error".to_string(),
            format!(
                "{err}\r\n\r\nThis usually mean that your port is in use.\r\nConfigured port: {}",
                config.web_port
            ),
            Some(config_file.clone()),
        )
        .unwrap();
        return Ok(());
    }
    result?;

    Ok(())
}
