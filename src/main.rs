mod autograbber;
mod config;
mod data;
mod linker;
mod mam;
mod mam_enums;
mod qbittorrent;

use anyhow::{Context, Result};
use autograbber::{autograb, run_autograbbers};
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};

use crate::{config::Config, linker::link_torrents_to_library, mam::MaM, qbittorrent::QbitError};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let config: Config = Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::prefixed("MLM_"))
        .extract()?;

    println!("config: {config:#?}");

    let db = native_db::Builder::new().create(&data::MODELS, "data.db")?;

    let mam = MaM::new(&config, &db).await?;

    let mut qbits = vec![];
    for qbit_conf in &config.qbittorrent {
        qbits.push((
            qbit_conf,
            qbit::Api::login(&qbit_conf.url, &qbit_conf.username, &qbit_conf.password)
                .await
                .map_err(QbitError)?,
        ));
    }

    run_autograbbers(&config, &db, &qbits[0].1, &mam).await?;

    // for qbit in qbits {
    //     link_torrents_to_library(&config, &db, qbit, &mam)
    //         .await
    //         .context("link_torrents_to_library")?;
    // }

    Ok(())
}
