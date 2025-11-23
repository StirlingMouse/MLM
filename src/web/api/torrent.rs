use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use native_db::Database;
use serde_json::json;

use crate::{
    audiobookshelf::Abs,
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
        torrent_api_id(State((config, db, mam)), Path(id_or_mam_id)).await
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
        return torrent_api_id(State((config, db, mam)), Path(torrent.id)).await;
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
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    Path(id): Path<String>,
) -> std::result::Result<Json<serde_json::Value>, AppError> {
    let abs = config.audiobookshelf.as_ref().map(Abs::new);
    let Some(mut torrent) = db.r_transaction()?.get().primary::<Torrent>(id)? else {
        return Err(AppError::NotFound);
    };
    // let replacement_torrent = torrent
    //     .replaced_with
    //     .as_ref()
    //     .map(|(id, _)| db.r_transaction()?.get().primary::<Torrent>(id.to_string()))
    //     .transpose()?
    //     .flatten();
    //
    // if replacement_torrent.is_none() && torrent.replaced_with.is_some() {
    //     let rw = db.rw_transaction()?;
    //     torrent.replaced_with = None;
    //     rw.upsert(torrent.clone())?;
    //     rw.commit()?;
    // }
    // let book = match abs {
    //     Some(abs) => abs?.get_book(&torrent).await?,
    //     None => None,
    // };

    // let events = db
    //     .r_transaction()?
    //     .scan()
    //     .secondary::<Event>(EventKey::created_at)?;
    // let events = events.all()?.rev();
    // let events = events
    //     .filter(|t| {
    //         let Ok(t) = t else {
    //             return true;
    //         };
    //         t.hash.as_deref() == Some(&torrent.hash)
    //     })
    //     .collect::<Result<Vec<_>, _>>()?;

    // let Ok(mam) = mam.as_ref() else {
    //     return Err(anyhow::Error::msg("mam_id error").into());
    // };
    // let mam_torrent = mam.get_torrent_info(&torrent.hash).await?;
    // let mam_meta = mam_torrent.as_ref().map(|t| t.as_meta()).transpose()?;

    // let mut qbit_data = None;
    // let mut wanted_path = None;
    let mut qbit_torrent = None;
    let mut qbit_files = vec![];
    if torrent.id_is_hash
        && let Some((qbit_torrent_, qbit, _)) =
            qbittorrent::get_torrent(&config, &torrent.id).await?
    {
        qbit_torrent = Some(qbit_torrent_);
        // let trackers = qbit.trackers(&torrent.hash).await?;
        // let mut categories = qbit.categories().await?.into_values().collect_vec();
        // categories.sort_by(|a, b| a.name.cmp(&b.name));
        // let tags = qbit.tags().await?;
        //
        // wanted_path = find_library(&config, &qbit_torrent).and_then(|library| {
        //     library_dir(
        //         config.exclude_narrator_in_library_dir,
        //         library,
        //         &torrent.meta,
        //     )
        // });

        // qbit_data = Some(QbitData {
        //     torrent_tags: qbit_torrent.tags.split(", ").map(str::to_string).collect(),
        //     torrent: qbit_torrent,
        //     trackers,
        //     categories,
        //     tags,
        // });

        qbit_files = qbit.files(&torrent.id, None).await?;
    }

    Ok::<_, AppError>(Json(json!({
        "abs_url": config
            .audiobookshelf
            .as_ref()
            .map(|abs| abs.url.clone())
            .unwrap_or_default(),
        "torrent": torrent,
        // "replacement_torrent": replacement_torrent,
        // "events": events,
        // "book": book,
        // "mam_torrent": mam_torrent,
        // "mam_meta": mam_meta,
        // "qbit_data": qbit_data,
        // "wanted_path": wanted_path,
        "qbit_torrent": qbit_torrent,
        "qbit_files": qbit_files,
    })))
}
