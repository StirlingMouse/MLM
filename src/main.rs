mod autograbber;
mod config;
mod data;
mod linker;
mod mam;
mod mam_enums;
mod qbittorrent;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use autograbber::run_autograbbers;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use tokio::time::sleep;

use crate::{config::Config, linker::link_torrents_to_library, mam::MaM, qbittorrent::QbitError};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let config: Config = Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::prefixed("MLM_"))
        .extract()?;
    let config = Arc::new(config);

    println!("config: {config:#?}");

    let db = native_db::Builder::new().create(&data::MODELS, "data.db")?;
    let db = Arc::new(db);

    let mam = MaM::new(&config, db.clone()).await?;
    let mam = Arc::new(mam);

    let mut qbits = vec![];
    {
        let config = config.clone();
        for qbit_conf in config.qbittorrent.iter().cloned() {
            let qbit = qbit::Api::login(&qbit_conf.url, &qbit_conf.username, &qbit_conf.password)
                .await
                .map_err(QbitError)?;
            qbits.push((qbit_conf, qbit));
        }
    }
    let qbits = Arc::new(qbits);

    {
        let config = config.clone();
        let db = db.clone();
        let mam = mam.clone();
        let qbits = qbits.clone();
        tokio::spawn(async move {
            loop {
                if let Err(err) =
                    run_autograbbers(config.clone(), db.clone(), &qbits[0].1, mam.clone()).await
                {
                    eprintln!("Error running autograbbers: {err}");
                }
                sleep(Duration::from_secs(60 * 20)).await;
            }
        });
    }

    {
        let config = config.clone();
        let db = db.clone();
        let mam = mam.clone();
        let qbits = qbits.clone();
        // tokio::spawn(async move {
        loop {
            for (qbit_conf, qbit) in qbits.iter() {
                if let Err(err) = link_torrents_to_library(
                    config.clone(),
                    db.clone(),
                    (qbit_conf, qbit),
                    mam.clone(),
                )
                .await
                // .context("link_torrents_to_library")
                {
                    eprintln!("Error running linker: {err}");
                }
                sleep(Duration::from_secs(60 * 10)).await;
            }
        }
        // });
    }

    // Ok(())
}
