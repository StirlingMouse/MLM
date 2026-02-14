use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use mlm_mam::api::MaM;
use qbit;
use time::OffsetDateTime;
use tokio::{
    select,
    sync::{Mutex, watch},
    time::sleep,
};
use tracing::error;

use crate::{
    audiobookshelf::match_torrents_to_abs,
    autograbber::run_autograbber,
    cleaner::run_library_cleaner,
    config::Config,
    linker::{folder::link_folders_to_library, torrent::link_torrents_to_library},
    lists::{get_lists, run_list_import},
    metadata::MetadataService,
    snatchlist::run_snatchlist_search,
    stats::{Context, Stats, Triggers},
    torrent_downloader::grab_selected_torrents,
};

pub fn spawn_tasks(
    config: Arc<Config>,
    db: Arc<mlm_db::Database<'static>>,
    mam: Arc<Result<Arc<MaM<'static>>>>,
    stats: Stats,
    metadata: Arc<MetadataService>,
) -> Context {
    let (mut search_tx, mut search_rx) = (BTreeMap::new(), BTreeMap::new());
    let (mut import_tx, mut import_rx) = (BTreeMap::new(), BTreeMap::new());
    let (torrent_linker_tx, torrent_linker_rx) = watch::channel(());
    let (folder_linker_tx, folder_linker_rx) = watch::channel(());
    let (downloader_tx, mut downloader_rx) = watch::channel(());
    let (audiobookshelf_tx, mut audiobookshelf_rx) = watch::channel(());

    for (i, _) in config.autograbs.iter().enumerate() {
        let (tx, rx) = watch::channel(());
        search_tx.insert(i, tx);
        search_rx.insert(i, rx);
    }
    for (i, _) in config.snatchlist.iter().enumerate() {
        let (tx, rx) = watch::channel(());
        let idx = i + config.autograbs.len();
        search_tx.insert(idx, tx);
        search_rx.insert(idx, rx);
    }
    for (i, _) in get_lists(&config).iter().enumerate() {
        let (tx, rx) = watch::channel(());
        import_tx.insert(i, tx);
        import_rx.insert(i, rx);
    }

    // Downloader task
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
                                        stats.downloader_run_at = Some(OffsetDateTime::now_utc());
                                        stats.downloader_result = Some(Err(err.into()));
                                    })
                                    .await;
                            }
                        };
                    }
                    let Some(qbit_api) = &qbit else {
                        continue;
                    };
                    let Ok(mam_api) = mam.as_ref() else {
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
                        grab_selected_torrents(&config, &db, qbit_api, &qbit_conf.url, mam_api)
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

    // Autograbber tasks
    for (i, grab) in config.autograbs.iter().enumerate() {
        let config = config.clone();
        let db = db.clone();
        let mam = mam.clone();
        let downloader_tx = downloader_tx.clone();
        let mut rx = search_rx.remove(&i).unwrap();
        let stats = stats.clone();
        let grab = Arc::new(grab.clone());
        tokio::spawn(async move {
            loop {
                let interval = grab.search_interval.unwrap_or(config.search_interval);
                if interval > 0 {
                    select! {
                        _ = sleep(Duration::from_secs(60 * interval)) => {},
                        result = rx.changed() => {
                            if let Err(err) = result {
                                error!("Error listening on search_rx: {err:?}");
                                stats.update(|stats| {
                                    stats.autograbber_result.insert(i, Err(err.into()));
                                }).await;
                                break;
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
                        break;
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
                let Ok(mam_api) = mam.as_ref() else {
                    continue;
                };
                let result = run_autograbber(
                    config.clone(),
                    db.clone(),
                    mam_api.clone(),
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

    // Snatchlist tasks
    for (i, grab) in config.snatchlist.iter().enumerate() {
        let idx = i + config.autograbs.len();
        let config = config.clone();
        let db = db.clone();
        let mam = mam.clone();
        let mut rx = search_rx.remove(&idx).unwrap();
        let stats = stats.clone();
        let grab = Arc::new(grab.clone());
        tokio::spawn(async move {
            loop {
                let interval = grab.search_interval.unwrap_or(config.search_interval);
                if interval > 0 {
                    select! {
                        _ = sleep(Duration::from_secs(60 * interval)) => {},
                        result = rx.changed() => {
                            if let Err(err) = result {
                                error!("Error listening on search_rx for snatchlist: {err:?}");
                                stats.update(|stats| {
                                    stats.autograbber_result.insert(idx, Err(err.into()));
                                }).await;
                                break;
                            }
                        },
                    }
                } else {
                    let result = rx.changed().await;
                    if let Err(err) = result {
                        error!("Error listening on search_rx for snatchlist: {err:?}");
                        stats
                            .update(|stats| {
                                stats.autograbber_result.insert(idx, Err(err.into()));
                            })
                            .await;
                        break;
                    }
                }
                {
                    stats
                        .update(|stats| {
                            stats
                                .autograbber_run_at
                                .insert(idx, OffsetDateTime::now_utc());
                            stats.autograbber_result.remove(&idx);
                        })
                        .await;
                }
                let Ok(mam_api) = mam.as_ref() else {
                    continue;
                };
                let result = run_snatchlist_search(
                    config.clone(),
                    db.clone(),
                    mam_api.clone(),
                    idx,
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
                            stats.autograbber_result.insert(idx, result);
                        })
                        .await;
                }
            }
        });
    }

    // List import tasks
    for (i, list) in get_lists(&config).into_iter().enumerate() {
        let config = config.clone();
        let db = db.clone();
        let mam = mam.clone();
        let downloader_tx = downloader_tx.clone();
        let mut rx = import_rx.remove(&i).unwrap();
        let stats = stats.clone();
        let list = Arc::new(list);
        tokio::spawn(async move {
            loop {
                let interval = list.search_interval().unwrap_or(config.import_interval);
                if interval > 0 {
                    select! {
                        _ = sleep(Duration::from_secs(60 * interval)) => {},
                        result = rx.changed() => {
                            if let Err(err) = result {
                                error!("Error listening on import_rx: {err:?}");
                                stats.update(|stats| {
                                    stats.import_result.insert(i, Err(err.into()));
                                }).await;
                                break;
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
                        break;
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
                let Ok(mam_api) = mam.as_ref() else {
                    continue;
                };
                let result = run_list_import(
                    config.clone(),
                    db.clone(),
                    mam_api.clone(),
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

    // Torrent linker tasks
    for qbit_conf in config.qbittorrent.clone() {
        let config = config.clone();
        let db = db.clone();
        let mam = mam.clone();
        let stats = stats.clone();
        let mut rx = torrent_linker_rx.clone();
        tokio::spawn(async move {
            loop {
                select! {
                    _ = sleep(Duration::from_secs(60 * config.link_interval)) => {},
                    result = rx.changed() => {
                        if let Err(err) = result {
                            error!("Error listening on link_rx: {err:?}");
                            stats
                                .update(|stats| {
                                    stats.torrent_linker_run_at = Some(OffsetDateTime::now_utc());
                                    stats.torrent_linker_result = Some(Err(err.into()));
                                }).await;
                            break;
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
                                stats.torrent_linker_run_at = Some(OffsetDateTime::now_utc());
                                stats.torrent_linker_result = Some(Err(anyhow::Error::msg(
                                    format!("Error logging in to qbit {}: {err}", qbit_conf.url,),
                                )));
                            })
                            .await;
                        continue;
                    }
                };
                let Ok(mam_api) = mam.as_ref() else {
                    continue;
                };
                let result = link_torrents_to_library(
                    config.clone(),
                    db.clone(),
                    (&qbit_conf, &qbit),
                    mam_api,
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

    // Folder linker task
    {
        let config = config.clone();
        let db = db.clone();
        let stats = stats.clone();
        let mut rx = folder_linker_rx.clone();
        tokio::spawn(async move {
            loop {
                select! {
                    _ = sleep(Duration::from_secs(60 * config.link_interval)) => {},
                    result = rx.changed() => {
                        if let Err(err) = result {
                            error!("Error listening on link_rx: {err:?}");
                            stats
                                .update(|stats| {
                                    stats.folder_linker_run_at = Some(OffsetDateTime::now_utc());
                                    stats.folder_linker_result = Some(Err(err.into()));
                                }).await;
                            break;
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

    // Audiobookshelf task
    if let Some(abs_config) = &config.audiobookshelf {
        let abs_config = abs_config.clone();
        let db = db.clone();
        let stats = stats.clone();
        tokio::spawn(async move {
            loop {
                select! {
                    _ = sleep(Duration::from_secs(60 * abs_config.interval)) => {},
                    result = audiobookshelf_rx.changed() => {
                        if let Err(err) = result {
                            error!("Error listening on audiobookshelf_rx: {err:?}");
                        stats
                            .update(|stats| {
                                stats.audiobookshelf_result = Some(Err(err.into()));
                            })
                            .await;
                        break;
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
                let result = match_torrents_to_abs(&abs_config, db.clone())
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

    Context {
        config: Arc::new(Mutex::new(config)),
        db,
        mam,
        stats,
        metadata,
        triggers: Triggers {
            search_tx,
            import_tx,
            torrent_linker_tx,
            folder_linker_tx,
            downloader_tx,
            audiobookshelf_tx,
        },
    }
}
