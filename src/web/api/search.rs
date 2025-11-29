use std::sync::Arc;

use anyhow::Result;
use axum::{
    Json,
    extract::{Query, State},
};
use axum_extra::extract::Form;
use native_db::Database;
use serde::{Deserialize, Serialize};

use crate::{
    autograbber::{search_torrents, select_torrents},
    config::{Config, TorrentSearch},
    mam::search::MaMTorrent,
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
    let torrents = search_torrents(&search, mam).await?.collect::<Vec<_>>();

    Ok::<_, AppError>(Json(SearchApiResponse {
        torrents: Some(torrents),
        ..Default::default()
    }))
}

pub async fn search_api_post(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    Form(form): Form<SearchApiForm>,
) -> Result<Json<SearchApiResponse>, AppError> {
    let Ok(mam) = mam.as_ref() else {
        return Err(anyhow::Error::msg("mam_id error").into());
    };
    let search: TorrentSearch = toml::from_str(&form.toml)?;
    let torrents = search_torrents(&search, mam).await?.collect::<Vec<_>>();
    if form.add {
        println!("adding");
        select_torrents(
            &config,
            &db,
            mam,
            torrents.into_iter(),
            &search.filter,
            search.cost,
            search.unsat_buffer,
            search.category.clone(),
            search.dry_run,
            u64::MAX,
            None,
        )
        .await?;
        println!("added");
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
