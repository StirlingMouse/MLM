mod config;
mod data;
mod linker;
mod mam;
mod qbittorrent;

use anyhow::{Context, Result};
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};

use crate::{config::Config, linker::link_torrents_to_library, mam::MaM, qbittorrent::QbitError};

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config = Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::prefixed("MLM_"))
        .extract()?;

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

    for qbit in qbits {
        link_torrents_to_library(&config, &db, qbit, &mam)
            .await
            .context("link_torrents_to_library")?;
    }

    Ok(())
}
