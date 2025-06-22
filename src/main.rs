mod config;
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

    let mam = MaM::new(&config)?;
    let qbit = qbit::Api::login(
        &config.qbittorrent.url,
        &config.qbittorrent.username,
        &config.qbittorrent.password,
    )
    .await
    .map_err(QbitError)?;

    link_torrents_to_library(&config, qbit, mam)
        .await
        .context("link_torrents_to_library")?;

    Ok(())
}
