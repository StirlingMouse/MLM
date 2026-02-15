use axum::{
    Json,
    extract::{Path, State},
};
use mlm_db::{Torrent, TorrentKey};
use serde_json::json;

use crate::AppError;
use mlm_core::{
    Context, ContextExt,
    qbittorrent::{self},
};

pub async fn torrent_api(
    State(context): State<Context>,
    Path(id_or_mam_id): Path<String>,
) -> std::result::Result<Json<serde_json::Value>, AppError> {
    if let Ok(id) = id_or_mam_id.parse() {
        torrent_api_mam_id(State(context), Path(id)).await
    } else {
        torrent_api_id(State(context), Path(id_or_mam_id)).await
    }
}

async fn torrent_api_mam_id(
    State(context): State<Context>,
    Path(mam_id): Path<u64>,
) -> std::result::Result<Json<serde_json::Value>, AppError> {
    if let Some(torrent) = context
        .db()
        .r_transaction()?
        .get()
        .secondary::<Torrent>(TorrentKey::mam_id, mam_id)?
    {
        return torrent_api_id(State(context), Path(torrent.id)).await;
    };

    let mam = context.mam()?;
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
    State(context): State<Context>,
    Path(id): Path<String>,
) -> std::result::Result<Json<serde_json::Value>, AppError> {
    let config = context.config().await;
    let Some(torrent) = context.db().r_transaction()?.get().primary::<Torrent>(id)? else {
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
