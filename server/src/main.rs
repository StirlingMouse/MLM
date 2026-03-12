#![windows_subsystem = "windows"]

use std::{
    collections::BTreeMap,
    env,
    fs::{self, create_dir_all},
    io, mem,
    path::PathBuf,
    process,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context as _, Result};
use dirs::{config_dir, data_local_dir};
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use mlm_mam::api::MaM;
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

use mlm::{
    audiobookshelf::match_torrents_to_abs,
    autograbber::run_autograbber,
    cleaner::run_library_cleaner,
    config::Config,
    linker::{folder::link_folders_to_library, torrent::link_torrents_to_library},
    lists::{get_lists, run_list_import},
    snatchlist::run_snatchlist_search,
    stats::{Context, Stats, Triggers},
    torrent_downloader::grab_selected_torrents,
    web::start_webserver,
};

#[cfg(target_family = "windows")]
use mlm::windows;

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
    let mut config = config?;
    for autograb in &mut config.autograbs {
        autograb.filter.edition = mem::take(&mut autograb.edition);
    }
    for snatchlist in &mut config.snatchlist {
        snatchlist.filter.edition = mem::take(&mut snatchlist.edition);
    }
    for list in &mut config.goodreads_lists {
        for grab in &mut list.grab {
            grab.filter.edition = mem::take(&mut grab.edition);
        }
    }
    for list in &mut config.notion_lists {
        for grab in &mut list.grab {
            grab.filter.edition = mem::take(&mut grab.edition);
        }
    }
    for tag in &mut config.tags {
        tag.filter.edition = mem::take(&mut tag.edition);
    }
    let config = Arc::new(config);

    let db = native_db::Builder::new().create(&mlm_db::MODELS, database_file)?;
    mlm_db::migrate(&db)?;

    if env::args().any(|arg| arg == "--update-search-title") {
        mlm_db::update_search_title(&db)?;
        return Ok(());
    }

    let db = Arc::new(db);

    #[cfg(target_family = "windows")]
    let _tray = windows::tray::start_tray_icon(log_dir, config_file.clone(), config.clone())?;

    let stats = Stats::new();

    let (mut search_tx, mut search_rx) = (BTreeMap::new(), BTreeMap::new());
    let (mut import_tx, mut import_rx) = (BTreeMap::new(), BTreeMap::new());
    let (torrent_linker_tx, torrent_linker_rx) = watch::channel(());
    let (folder_linker_tx, folder_linker_rx) = watch::channel(());
    let (downloader_tx, mut downloader_rx) = watch::channel(());
    let (audiobookshelf_tx, mut audiobookshelf_rx) = watch::channel(());

    let mam = if config.mam_id.is_empty() {
        Err(anyhow::Error::msg("No mam_id set"))
    } else {
        MaM::new(&config.mam_id, db.clone()).await.map(Arc::new)
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
                                    stats
                                        .update(|stats| {
                                            stats.downloader_run_at =
                                                Some(OffsetDateTime::now_utc());
                                            stats.downloader_result = Some(Err(err.into()));
                                        })
                                        .await;
                                }
                            };
                        }
                        let Some(qbit) = &qbit else {
                            continue;
                        };
                        {
                            stats
                                .update(|stats| {
                                    stats.downloader_run_at = Some(OffsetDateTime::now_utc());
                                    stats.downloader_result = None;
                                })
                                .await;
                        }
                        let result =
                            grab_selected_torrents(&config, &db, qbit, &qbit_conf.url, &mam)
                                .await
                                .context("grab_selected_torrents");

                        if let Err(err) = &result {
                            error!("Error grabbing selected torrents: {err:?}");
                        }
                        {
                            stats
                                .update(|stats| {
                                    stats.downloader_result = Some(result);
                                })
                                .await;
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
                                    stats.update(|stats| {
                                        stats.autograbber_result.insert(i, Err(err.into()));
                                    }).await;
                                }
                            },
                        }
                    } else {
                        let result = rx.changed().await;
                        if let Err(err) = result {
                            error!("Error listening on search_rx: {err:?}");
                            stats
                                .update(|stats| {
                                    stats.autograbber_result.insert(i, Err(err.into()));
                                })
                                .await;
                        }
                    }
                    {
                        stats
                            .update(|stats| {
                                stats
                                    .autograbber_run_at
                                    .insert(i, OffsetDateTime::now_utc());
                                stats.autograbber_result.remove(&i);
                            })
                            .await;
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
                        stats
                            .update(|stats| {
                                stats.autograbber_result.insert(i, result);
                            })
                            .await;
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
                                    stats.update(|stats| {
                                        stats.autograbber_result.insert(i, Err(err.into()));
                                    }).await;
                                }
                            },
                        }
                    } else {
                        let result = rx.changed().await;
                        if let Err(err) = result {
                            error!("Error listening on search_rx for snatchlist: {err:?}");
                            stats
                                .update(|stats| {
                                    stats.autograbber_result.insert(i, Err(err.into()));
                                })
                                .await;
                        }
                    }
                    {
                        stats
                            .update(|stats| {
                                stats
                                    .autograbber_run_at
                                    .insert(i, OffsetDateTime::now_utc());
                                stats.autograbber_result.remove(&i);
                            })
                            .await;
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
                        stats
                            .update(|stats| {
                                stats.autograbber_result.insert(i, result);
                            })
                            .await;
                    }
                }
            });
        }

        for (i, list) in get_lists(&config).into_iter().enumerate() {
            let config = config.clone();
            let db = db.clone();
            let mam = mam.clone();
            let downloader_tx = downloader_tx.clone();
            let (tx, mut rx) = watch::channel(());
            import_tx.insert(i, tx);
            import_rx.insert(i, rx.clone());
            let stats = stats.clone();
            let list = Arc::new(list);
            tokio::spawn(async move {
                loop {
                    let interval = list.search_interval().unwrap_or(config.import_interval);
                    if interval > 0 {
                        select! {
                            () = sleep(Duration::from_secs(60 * interval)) => {},
                            result = rx.changed() => {
                                if let Err(err) = result {
                                    error!("Error listening on import_rx: {err:?}");
                                    stats.update(|stats| {
                                        stats.import_result.insert(i, Err(err.into()));
                                    }).await;
                                }
                            },
                        }
                    } else {
                        let result = rx.changed().await;
                        if let Err(err) = result {
                            error!("Error listening on import_rx: {err:?}");
                            stats
                                .update(|stats| {
                                    stats.import_result.insert(i, Err(err.into()));
                                })
                                .await;
                        }
                    }
                    {
                        stats
                            .update(|stats| {
                                stats.import_run_at.insert(i, OffsetDateTime::now_utc());
                                stats.import_result.remove(&i);
                            })
                            .await;
                    }
                    let result = run_list_import(
                        config.clone(),
                        db.clone(),
                        mam.clone(),
                        list.clone(),
                        i,
                        downloader_tx.clone(),
                    )
                    .await
                    .context("import");
                    if let Err(err) = &result {
                        error!("Error running import: {err:?}");
                    }
                    {
                        stats
                            .update(|stats| {
                                stats.import_result.insert(i, result);
                            })
                            .await;
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
                let mut linker_rx = torrent_linker_rx.clone();
                tokio::spawn(async move {
                    loop {
                        select! {
                            () = sleep(Duration::from_secs(60 * config.link_interval)) => {},
                            result = linker_rx.changed() => {
                                if let Err(err) = result {
                                    error!("Error listening on link_rx: {err:?}");
                                    stats
                                        .update(|stats| {
                                            stats.torrent_linker_run_at = Some(OffsetDateTime::now_utc());
                                            stats.torrent_linker_result = Some(Err(err.into()));
                                        }).await;
                                }
                            },
                        }
                        {
                            stats
                                .update(|stats| {
                                    stats.torrent_linker_run_at = Some(OffsetDateTime::now_utc());
                                    stats.torrent_linker_result = None;
                                })
                                .await;
                        }
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
                                stats
                                    .update(|stats| {
                                        stats.torrent_linker_run_at =
                                            Some(OffsetDateTime::now_utc());
                                        stats.torrent_linker_result =
                                            Some(Err(anyhow::Error::msg(format!(
                                                "Error logging in to qbit {}: {err}",
                                                qbit_conf.url,
                                            ))));
                                    })
                                    .await;
                                continue;
                            }
                        };
                        let result = link_torrents_to_library(
                            config.clone(),
                            db.clone(),
                            (&qbit_conf, &qbit),
                            &mam,
                        )
                        .await
                        .context("link_torrents_to_library");
                        if let Err(err) = &result {
                            error!("Error running linker: {err:?}");
                        }
                        {
                            stats
                                .update(|stats| {
                                    stats.torrent_linker_result = Some(result);
                                    stats.cleaner_run_at = Some(OffsetDateTime::now_utc());
                                    stats.cleaner_result = None;
                                })
                                .await;
                        }
                        let result = run_library_cleaner(config.clone(), db.clone())
                            .await
                            .context("library_cleaner");
                        if let Err(err) = &result {
                            error!("Error running library_cleaner: {err:?}");
                        }
                        {
                            stats
                                .update(|stats| {
                                    stats.cleaner_result = Some(result);
                                })
                                .await;
                        }
                    }
                });
            }
        }
    }
    {
        let config = config.clone();
        let db = db.clone();
        let stats = stats.clone();
        let mut linker_rx = folder_linker_rx.clone();
        tokio::spawn(async move {
            loop {
                select! {
                    () = sleep(Duration::from_secs(60 * config.link_interval)) => {},
                    result = linker_rx.changed() => {
                        if let Err(err) = result {
                            error!("Error listening on link_rx: {err:?}");
                            stats
                                .update(|stats| {
                                    stats.folder_linker_run_at = Some(OffsetDateTime::now_utc());
                                    stats.folder_linker_result = Some(Err(err.into()));
                                }).await;
                        }
                    },
                }
                {
                    stats
                        .update(|stats| {
                            stats.folder_linker_run_at = Some(OffsetDateTime::now_utc());
                            stats.folder_linker_result = None;
                        })
                        .await;
                }
                let result = link_folders_to_library(config.clone(), db.clone())
                    .await
                    .context("link_torrents_to_library");
                if let Err(err) = &result {
                    error!("Error running linker: {err:?}");
                }
                {
                    stats
                        .update(|stats| {
                            stats.folder_linker_result = Some(result);
                            stats.cleaner_run_at = Some(OffsetDateTime::now_utc());
                            stats.cleaner_result = None;
                        })
                        .await;
                }
                let result = run_library_cleaner(config.clone(), db.clone())
                    .await
                    .context("library_cleaner");
                if let Err(err) = &result {
                    error!("Error running library_cleaner: {err:?}");
                }
                {
                    stats
                        .update(|stats| {
                            stats.cleaner_result = Some(result);
                        })
                        .await;
                }
            }
        });
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
                        stats
                            .update(|stats| {
                                stats.audiobookshelf_result = Some(Err(err.into()));
                            })
                            .await;
                        }
                    },
                }
                {
                    stats
                        .update(|stats| {
                            stats.audiobookshelf_run_at = Some(OffsetDateTime::now_utc());
                            stats.audiobookshelf_result = None;
                        })
                        .await;
                }
                let result = match_torrents_to_abs(&config, db.clone())
                    .await
                    .context("audiobookshelf_matcher");
                if let Err(err) = &result {
                    error!("Error running audiobookshelf matcher: {err:?}");
                }
                {
                    stats
                        .update(|stats| {
                            stats.audiobookshelf_result = Some(result);
                        })
                        .await;
                }
            }
        });
    }

    let triggers = Triggers {
        search_tx,
        import_tx,
        torrent_linker_tx,
        folder_linker_tx,
        downloader_tx,
        audiobookshelf_tx,
    };
    #[cfg(target_family = "windows")]
    let web_port = config.web_port;
    let context = Context {
        config: Arc::new(Mutex::new(config)),
        db,
        mam: Arc::new(mam),
        stats,
        triggers,
    };

    let result = start_webserver(context).await;

    #[cfg(target_family = "windows")]
    if let Err(err) = &result {
        windows::error_window::ErrorWindow::create_and_run(
            "MLM Webserver Error".to_string(),
            format!(
                "{err}\r\n\r\nThis usually mean that your port is in use.\r\nConfigured port: {}",
                web_port
            ),
            Some(config_file.clone()),
        )
        .unwrap();
        return Ok(());
    }
    result?;

    Ok(())
}
