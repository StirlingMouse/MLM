mod autograbber;
mod cleaner;
mod config;
mod data;
mod data_impl;
mod exporter;
mod linker;
mod mam;
mod mam_enums;
mod qbittorrent;
mod web;

use std::{env, sync::Arc, time::Duration};

use anyhow::Result;
use autograbber::run_autograbbers;
use axum::{Router, routing::get};
use cleaner::run_library_cleaner;
use exporter::export_db;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use tokio::time::sleep;
use web::start_webserver;

use crate::{config::Config, linker::link_torrents_to_library, mam::MaM, qbittorrent::QbitError};

#[tokio::main]
// #[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    let config_file = env::var("CONFIG_FILE").unwrap_or("config.toml".to_owned());
    let database_file = env::var("DB_FILE").unwrap_or("data.db".to_owned());
    let config: Config = Figment::new()
        .merge(Toml::file(config_file))
        .merge(Env::prefixed("MLM_"))
        .extract()?;
    let config = Arc::new(config);

    println!("config: {config:#?}");

    let db = native_db::Builder::new().create(&data::MODELS, database_file)?;
    data::migrate(&db)?;
    // export_db(&db)?;
    // return Ok(());
    let db = Arc::new(db);

    let mam = MaM::new(&config, db.clone()).await?;
    let mam = Arc::new(mam);

    if let Some(qbit_conf) = config.qbittorrent.first() {
        let config = config.clone();
        let db = db.clone();
        let mam = mam.clone();
        let qbit = qbit::Api::login(&qbit_conf.url, &qbit_conf.username, &qbit_conf.password)
            .await
            .map_err(QbitError)?;
        tokio::spawn(async move {
            loop {
                if let Err(err) =
                    run_autograbbers(config.clone(), db.clone(), &qbit, mam.clone()).await
                {
                    eprintln!("Error running autograbbers: {err}");
                }
                sleep(Duration::from_secs(60 * config.search_interval)).await;
            }
        });
    }

    {
        for qbit_conf in config.qbittorrent.clone() {
            let config = config.clone();
            let db = db.clone();
            let mam = mam.clone();
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
                        eprintln!("Error logging in to qbit {}: {err}", qbit_conf.url);
                        return;
                    }
                };
                loop {
                    if let Err(err) = link_torrents_to_library(
                        config.clone(),
                        db.clone(),
                        (&qbit_conf, &qbit),
                        mam.clone(),
                    )
                    .await
                    // .context("link_torrents_to_library")
                    {
                        eprintln!("Error running linker: {err}");
                    }
                    if let Err(err) = run_library_cleaner(config.clone(), db.clone()).await {
                        eprintln!("Error running library_cleaner: {err}");
                    }
                    sleep(Duration::from_secs(60 * config.link_interval)).await;
                }
            });
        }
    }

    start_webserver(config, db).await?;

    Ok(())
}
