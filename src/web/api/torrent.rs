use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use native_db::Database;
use serde_json::json;

use crate::{
    config::Config,
    data::{Torrent, TorrentKey},
    qbittorrent::{self},
    web::{AppError, MaMState},
};

pub async fn torrent_api(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    Path(id_or_mam_id): Path<String>,
) -> std::result::Result<Json<serde_json::Value>, AppError> {
    if let Ok(id) = id_or_mam_id.parse() {
        torrent_api_mam_id(State((config, db, mam)), Path(id)).await
    } else {
        torrent_api_id(State((config, db)), Path(id_or_mam_id)).await
    }
}

async fn torrent_api_mam_id(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    Path(mam_id): Path<u64>,
) -> std::result::Result<Json<serde_json::Value>, AppError> {
    if let Some(torrent) = db
        .r_transaction()?
        .get()
        .secondary::<Torrent>(TorrentKey::mam_id, mam_id)?
    {
        return torrent_api_id(State((config, db)), Path(torrent.id)).await;
    };

    let Ok(mam) = mam.as_ref() else {
        return Err(anyhow::Error::msg("mam_id error").into());
    };
    let Some(mam_torrent) = mam.get_torrent_info_by_id(mam_id).await? else {
        return Err(AppError::NotFound);
    };
    let meta = mam_torrent.as_meta()?;

    Ok::<_, AppError>(Json(json!({
        "mam_torrent": mam_torrent,
        "meta": meta,
    })))
}

async fn torrent_api_id(
    State((config, db)): State<(Arc<Config>, Arc<Database<'static>>)>,
    Path(id): Path<String>,
) -> std::result::Result<Json<serde_json::Value>, AppError> {
    let Some(torrent) = db.r_transaction()?.get().primary::<Torrent>(id)? else {
        return Err(AppError::NotFound);
    };
    let mut qbit_torrent = None;
    let mut qbit_files = vec![];
    if torrent.id_is_hash
        && let Some((qbit_torrent_, qbit, _)) =
            qbittorrent::get_torrent(&config, &torrent.id).await?
    {
        qbit_torrent = Some(qbit_torrent_);
        qbit_files = qbit.files(&torrent.id, None).await?;
    }

    Ok::<_, AppError>(Json(json!({
        "abs_url": config
            .audiobookshelf
            .as_ref()
            .map(|abs| abs.url.clone())
            .unwrap_or_default(),
        "torrent": torrent,
        "qbit_torrent": qbit_torrent,
        "qbit_files": qbit_files,
    })))
}
