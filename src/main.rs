mod autograbber;
mod config;
mod data;
mod linker;
mod mam;
mod mam_enums;
mod qbittorrent;

use anyhow::{Context, Result};
use autograbber::autograb;
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

    println!("config: {config:#?}");

    let db = native_db::Builder::new().create(&data::MODELS, "data.db")?;

    let mam = MaM::new(&config, &db).await?;
    let user_info = mam.user_info().await?;
    let max_torrents = user_info
        .classname
        .unsats()
        .saturating_sub(user_info.unsat.count as u8);
    println!("user_info: {user_info:#?}; max_torrents: {max_torrents}");

    let mut snatched_torrents = 0;
    for autograb_config in &config.autograbs {
        let max_torrents = max_torrents
            .saturating_sub(autograb_config.unsat_buffer.unwrap_or(config.unsat_buffer))
            .saturating_sub(snatched_torrents);
        if max_torrents > 0 {
            snatched_torrents += autograb(autograb_config, &mam, max_torrents).await?;
        }
    }
    // let search = mauser_infom.user_info().await?;
    //
    // let mut qbits = vec![];
    // for qbit_conf in &config.qbittorrent {
    //     qbits.push((
    //         qbit_conf,
    //         qbit::Api::login(&qbit_conf.url, &qbit_conf.username, &qbit_conf.password)
    //             .await
    //             .map_err(QbitError)?,
    //     ));
    // }
    //
    // for qbit in qbits {
    //     link_torrents_to_library(&config, &db, qbit, &mam)
    //         .await
    //         .context("link_torrents_to_library")?;
    // }

    Ok(())
}
