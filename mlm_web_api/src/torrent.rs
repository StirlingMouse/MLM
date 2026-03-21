use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    response::Response,
};
use mlm_db::{Torrent, TorrentKey, ids};
use serde_json::json;

use crate::error::AppError;
use mlm_core::{
    Context, ContextExt,
    qbittorrent::{self},
};

pub async fn torrent_api(
    State(context): State<Context>,
    Path(id_or_mam_id): Path<String>,
) -> std::result::Result<Json<serde_json::Value>, AppError> {
    if let Some(torrent) = context
        .db()
        .r_transaction()?
        .get()
        .primary::<Torrent>(id_or_mam_id.clone())?
    {
        if torrent.id_is_hash {
            return torrent_api_id(State(context), Path(torrent.id)).await;
        }
        if let Some(mam_id) = torrent.mam_id {
            return torrent_api_mam_id(State(context), Path(mam_id)).await;
        }
        return torrent_api_id(State(context), Path(torrent.id)).await;
    }

    let mam_id = id_or_mam_id.parse().map_err(|_| AppError::NotFound)?;
    torrent_api_mam_id(State(context), Path(mam_id)).await
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

    Ok(Json(json!({
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

    Ok(Json(json!({
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

pub async fn torrent_cover_redirect(
    State(context): State<Context>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let config = context.config().await;
    let abs_cfg = config.audiobookshelf.as_ref().ok_or(AppError::NotFound)?;

    let torrent = context
        .db()
        .r_transaction()?
        .get()
        .primary::<Torrent>(id)?
        .ok_or(AppError::NotFound)?;

    let abs_id = torrent.meta.ids.get(ids::ABS).ok_or(AppError::NotFound)?;

    let cover_url = format!("{}/api/items/{}/cover", abs_cfg.url, abs_id);

    // Return 302 redirect to ABS cover URL
    let response = Response::builder()
        .status(302)
        .header(axum::http::header::LOCATION, cover_url)
        .body(Body::empty())
        .map_err(|e| AppError::Generic(anyhow::anyhow!("Failed to build response: {}", e)))?;
    Ok(response)
}
