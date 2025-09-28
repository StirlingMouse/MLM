use std::fmt::{Debug, Display};

use anyhow::Result;
use qbit::{
    models::{TorrentInfo, Tracker},
    parameters::TorrentListParams,
};

use crate::config::Config;

#[derive(Debug)]
#[allow(unused)]
pub struct QbitError(pub qbit::Error);

impl std::error::Error for QbitError {}
impl Display for QbitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}
impl From<qbit::Error> for QbitError {
    fn from(value: qbit::Error) -> Self {
        QbitError(value)
    }
}

pub async fn get_torrent(config: &Config, hash: &str) -> Result<Option<(TorrentInfo, qbit::Api)>> {
    for qbit_conf in config.qbittorrent.iter() {
        let Ok(qbit) = qbit::Api::login(&qbit_conf.url, &qbit_conf.username, &qbit_conf.password)
            .await
            .map_err(QbitError)
        else {
            continue;
        };
        let Some(torrent) = qbit
            .torrents(TorrentListParams {
                hashes: Some(vec![hash.to_string()]),
                ..TorrentListParams::default()
            })
            .await
            .map_err(QbitError)?
            .into_iter()
            .next()
        else {
            continue;
        };
        return Ok(Some((torrent, qbit)));
    }
    Ok(None)
}
