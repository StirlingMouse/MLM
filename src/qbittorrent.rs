use anyhow::Result;
use qbit::{models::Torrent, parameters::TorrentListParams};

use crate::config::Config;

pub async fn get_torrent(config: &Config, hash: &str) -> Result<Option<(Torrent, qbit::Api)>> {
    for qbit_conf in config.qbittorrent.iter() {
        let Ok(qbit) = qbit::Api::new_login_username_password(
            &qbit_conf.url,
            &qbit_conf.username,
            &qbit_conf.password,
        )
        .await
        else {
            continue;
        };
        let Some(torrent) = qbit
            .torrents(Some(TorrentListParams {
                hashes: Some(vec![hash.to_string()]),
                ..TorrentListParams::default()
            }))
            .await?
            .into_iter()
            .next()
        else {
            continue;
        };
        return Ok(Some((torrent, qbit)));
    }
    Ok(None)
}
