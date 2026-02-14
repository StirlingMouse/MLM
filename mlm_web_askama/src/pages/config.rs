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

use mlm_core::{
    autograbber::update_torrent_meta,
    config::{Config, Library},
    qbittorrent::ensure_category_exists,
    stats::Context,
};
use crate::{AppError, Page, filter, yaml_items, yaml_nums};

pub async fn config_page(
    State(config): State<Arc<Config>>,
    Query(query): Query<ConfigPageQuery>,
) -> std::result::Result<Html<String>, AppError> {
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
            use tokio::sync::Semaphore;

            const BATCH_SIZE: usize = 100;
            const MAX_CONCURRENT_MAM_REQUESTS: usize = 5;

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
            let torrents = context.db.r_transaction()?.scan().primary::<Torrent>()?;
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
                                &context.db,
                                context.db.rw_async().await?,
                                Some(&mam_torrent),
                                torrent.clone(),
                                new_meta,
                                false,
                                false,
                            )
                            .await?;
                        }
                        Err(err) => {
                            let Some(mam_id) = torrent.mam_id else {
                                continue;
                            };
                            info!("need to ask mam due to: {err}");

                            // Acquire semaphore permit to limit concurrent MAM requests
                            let permit = mam_semaphore
                                .clone()
                                .acquire_owned()
                                .await
                                .map_err(anyhow::Error::new)?;

                            let mam = context.mam()?;
                            let Some(mam_torrent) = mam.get_torrent_info_by_id(mam_id).await?
                            else {
                                warn!("could not get torrent from mam");
                                continue;
                            };

                            // Drop permit after MAM call completes
                            drop(permit);

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
                            matched_torrents.push(torrent);
                        }
                    };
                }

                // Second pass: Batch qBittorrent API calls
                if !matched_torrents.is_empty() {
                    let matched_count = matched_torrents.len();
                    info!(
                        "Applying category/tags to {} torrents in batch",
                        matched_count
                    );

                    // Collect all hashes for batch API calls
                    let hashes: Vec<&str> =
                        matched_torrents.iter().map(|t| t.id.as_str()).collect();

                    // Batch set category if specified
                    if let Some(category) = &tag_filter.category {
                        ensure_category_exists(&qbit, &qbit_conf.url, category).await?;
                        qbit.set_category(Some(hashes.clone()), category).await?;
                        info!("Set category '{}' for {} torrents", category, matched_count);
                    }

                    // Batch add tags if specified
                    if !tag_filter.tags.is_empty() {
                        let tags: Vec<&str> = tag_filter.tags.iter().map(Deref::deref).collect();
                        qbit.add_tags(Some(hashes), tags).await?;
                        info!(
                            "Added tags {:?} to {} torrents",
                            tag_filter.tags, matched_count
                        );
                    }

                    processed_count += matched_count;
                }

                info!(
                    "Completed batch {}/{}, total processed: {}/{}",
                    batch_idx + 1,
                    total_torrents.div_ceil(BATCH_SIZE),
                    processed_count,
                    total_torrents
                );
            }

            info!(
                "Tag filter application complete: {} torrents processed",
                processed_count
            );
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
