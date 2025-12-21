use std::{fs::File, path::PathBuf};

use anyhow::Result;
use axum::{
    Json,
    extract::{Query, State},
};
use axum_extra::extract::Form;
use mlm_mam::search::{MaMTorrent, SearchFields};
use serde::{Deserialize, Serialize};
use tokio::fs::create_dir_all;

use crate::{
    autograbber::{mark_removed_torrents, search_torrents, select_torrents},
    config::{Cost, TorrentSearch},
    stats::Context,
    web::{AppError, MaMState},
};

pub async fn search_api(
    State(mam): State<MaMState>,
    Query(query): Query<SearchApiQuery>,
) -> std::result::Result<Json<SearchApiResponse>, AppError> {
    let Ok(mam) = mam.as_ref() else {
        return Err(anyhow::Error::msg("mam_id error").into());
    };
    let search: TorrentSearch = toml::from_str(&query.toml)?;
    let torrents = search_torrents(
        &search,
        SearchFields {
            description: true,
            isbn: true,
            media_info: true,
            ..Default::default()
        },
        mam,
    )
    .await?
    .collect::<Vec<_>>();

    Ok::<_, AppError>(Json(SearchApiResponse {
        torrents: Some(torrents),
        ..Default::default()
    }))
}

pub async fn search_api_post(
    State(context): State<Context>,
    Form(form): Form<SearchApiForm>,
) -> Result<Json<SearchApiResponse>, AppError> {
    let config = context.config().await;
    let mam = context.mam()?;
    let search: TorrentSearch = toml::from_str(&form.toml)?;
    let torrents = search_torrents(
        &search,
        SearchFields {
            description: true,
            isbn: true,
            media_info: true,
            dl_link: form.add
                && search.cost != Cost::MetadataOnly
                && search.cost != Cost::MetadataOnlyAdd,
            ..Default::default()
        },
        &mam,
    )
    .await?
    .collect::<Vec<_>>();
    if form.write_json {
        for torrent in &torrents {
            let id_str = torrent.id.to_string();
            let first = id_str.get(0..1).unwrap_or_default();
            let second = id_str.get(1..2).unwrap_or_default();
            let third = id_str.get(3..4).unwrap_or_default();
            let path = PathBuf::from("/data/torrents")
                .join(first)
                .join(second)
                .join(third);
            create_dir_all(&path).await.unwrap();
            let file_path = path.join(format!("{}.json", torrent.id));
            let file = File::create(file_path).unwrap();
            serde_json::to_writer(file, torrent).unwrap();
        }
    }

    if form.mark_removed {
        mark_removed_torrents(&context.db, &mam, &torrents).await?;
    }

    if form.add {
        select_torrents(
            &config,
            &context.db,
            &mam,
            torrents.into_iter(),
            &search.filter,
            search.cost,
            search.unsat_buffer,
            search.wedge_buffer,
            search.category.clone(),
            search.dry_run,
            u64::MAX,
            None,
        )
        .await?;
        return Ok::<_, AppError>(Json(SearchApiResponse {
            added: Some(true),
            ..Default::default()
        }));
    }

    Ok::<_, AppError>(Json(SearchApiResponse {
        torrents: Some(torrents),
        ..Default::default()
    }))
}

#[derive(Debug, Deserialize)]
pub struct SearchApiForm {
    toml: String,
    #[serde(default)]
    add: bool,
    #[serde(default)]
    mark_removed: bool,
    #[serde(default)]
    write_json: bool,
}

#[derive(Deserialize)]
pub struct SearchApiQuery {
    toml: String,
}

#[derive(Default, Serialize)]
pub struct SearchApiResponse {
    torrents: Option<Vec<MaMTorrent>>,
    added: Option<bool>,
}
