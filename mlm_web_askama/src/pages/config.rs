#![allow(dead_code)]

use std::{ops::Deref, sync::Arc};

use anyhow::Result;
use askama::Template;
use axum::{
    extract::{OriginalUri, Query, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use mlm_db::{DatabaseExt as _, Torrent};
use serde::Deserialize;
use tracing::{info, warn};

use crate::{AppError, Page, filter, yaml_items, yaml_nums};
use mlm_core::config::Library;
use mlm_core::{
    Config, Context, ContextExt, autograbber::update_torrent_meta,
    qbittorrent::ensure_category_exists,
};

pub async fn config_page(
    State(context): State<Context>,
    Query(query): Query<ConfigPageQuery>,
) -> std::result::Result<Html<String>, AppError> {
    let config = context.config().await;
    let template = ConfigPageTemplate {
        config,
        show_apply_tags: query.show_apply_tags.unwrap_or_default(),
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn config_page_post(
    State(context): State<Context>,
    uri: OriginalUri,
    Form(form): Form<ConfigPageForm>,
) -> Result<Redirect, AppError> {
    let config = context.config().await;
    match form.action.as_str() {
        "apply" => {
            let tag_filter = form
                .tag_filter
                .ok_or(anyhow::Error::msg("apply requires tag_filter"))?;
            let tag_filter = config
                .tags
                .get(tag_filter)
                .ok_or(anyhow::Error::msg("invalid tag_filter"))?;
            let qbit_conf = config
                .qbittorrent
                .get(form.qbit_index.unwrap_or_default())
                .ok_or(anyhow::Error::msg("requires a qbit config"))?;
            let qbit = qbit::Api::new_login_username_password(
                &qbit_conf.url,
                &qbit_conf.username,
                &qbit_conf.password,
            )
            .await?;
            let torrents = context.db().r_transaction()?.scan().primary::<Torrent>()?;
            for torrent in torrents.all()? {
                let torrent = torrent?;
                match tag_filter.filter.matches_lib(&torrent) {
                    Ok(matches) => {
                        if !matches {
                            continue;
                        }
                    }
                    Err(err) => {
                        let Some(mam_id) = torrent.mam_id else {
                            continue;
                        };
                        info!("need to ask mam due to: {err}");
                        let mam = context.mam()?;
                        let Some(mam_torrent) = mam.get_torrent_info_by_id(mam_id).await? else {
                            warn!("could not get torrent from mam");
                            continue;
                        };
                        let new_meta = mam_torrent.as_meta()?;
                        if new_meta != torrent.meta {
                            update_torrent_meta(
                                &config,
                                context.db(),
                                context.db().rw_async().await?,
                                Some(&mam_torrent),
                                torrent.clone(),
                                new_meta,
                                false,
                                false,
                                &context.events,
                            )
                            .await?;
                        }
                        if !tag_filter.filter.matches(&mam_torrent) {
                            continue;
                        }
                    }
                };
                if let Some(category) = &tag_filter.category {
                    ensure_category_exists(&qbit, &qbit_conf.url, category).await?;
                    qbit.set_category(Some(vec![torrent.id.as_str()]), category)
                        .await?;
                    info!(
                        "set category {} on torrent {}",
                        category, torrent.meta.title
                    );
                }

                if !tag_filter.tags.is_empty() {
                    qbit.add_tags(
                        Some(vec![torrent.id.as_str()]),
                        tag_filter.tags.iter().map(Deref::deref).collect(),
                    )
                    .await?;
                    info!(
                        "set tags {:?} on torrent {}",
                        tag_filter.tags, torrent.meta.title
                    );
                }
            }
        }
        action => {
            eprintln!("unknown action: {action}");
        }
    }

    Ok(Redirect::to(&uri.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct ConfigPageQuery {
    show_apply_tags: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigPageForm {
    action: String,
    qbit_index: Option<usize>,
    tag_filter: Option<usize>,
}

#[derive(Template)]
#[template(path = "pages/config.html")]
struct ConfigPageTemplate {
    config: Arc<Config>,
    show_apply_tags: bool,
}

impl Page for ConfigPageTemplate {}
