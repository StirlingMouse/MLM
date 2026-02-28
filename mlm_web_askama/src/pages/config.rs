// TODO: Remove this temporary allow once the Askama config page is fully retired
// after the Dioxus port has stabilized.
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
use tokio::sync::Semaphore;
use tracing::{info, warn};

use crate::{AppError, Page, filter, yaml_items, yaml_nums};
use mlm_core::{
    Context, ContextExt,
    autograbber::update_torrent_meta,
    config::{Config, Library},
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

            // Collect all torrents first
            let torrents: Vec<Torrent> = context
                .db()
                .r_transaction()?
                .scan()
                .primary::<Torrent>()?
                .all()?
                .collect::<Result<Vec<_>, _>>()?;

            let total_torrents = torrents.len();
            info!("Processing {} torrents for tag filter", total_torrents);

            // Create semaphore to limit concurrent MAM API calls
            let mam_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_MAM_REQUESTS));
            let mut processed_count = 0;

            // Process torrents in batches
            for (batch_idx, batch) in torrents.chunks(BATCH_SIZE).enumerate() {
                let batch_start = batch_idx * BATCH_SIZE;
                let batch_end = (batch_start + batch.len()).min(total_torrents);
                info!(
                    "Processing batch {}/{} (torrents {}-{})",
                    batch_idx + 1,
                    total_torrents.div_ceil(BATCH_SIZE),
                    batch_start + 1,
                    batch_end
                );

                // First pass: Process MAM API calls with concurrency limiting
                // Collect torrents that match the filter
                let mut matched_torrents: Vec<&Torrent> = Vec::with_capacity(batch.len());

                for torrent in batch {
                    match tag_filter.filter.matches_lib(torrent) {
                        Ok(matches) => {
                            if matches {
                                matched_torrents.push(torrent);
                            }
                        }
                        Err(err) => {
                            let Some(mam_id) = torrent.mam_id else {
                                continue;
                            };
                            info!("need to ask mam due to: {err}");

                            let permit = mam_semaphore
                                .clone()
                                .acquire_owned()
                                .await
                                .map_err(anyhow::Error::new)?;

                            let mam = context.mam()?;
                            let Some(mam_torrent) = mam.get_torrent_info_by_id(mam_id).await?
                            else {
                                drop(permit);
                                warn!("could not get torrent from mam");
                                continue;
                            };

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

                            if tag_filter.filter.matches(&mam_torrent) {
                                matched_torrents.push(torrent);
                            }
                        }
                    }
                }

                if !matched_torrents.is_empty() {
                    let matched_count = matched_torrents.len();
                    info!(
                        "Applying category/tags to {} torrents in batch",
                        matched_count
                    );

                    let hashes: Vec<&str> =
                        matched_torrents.iter().map(|t| t.id.as_str()).collect();

                    if let Some(category) = &tag_filter.category {
                        ensure_category_exists(&qbit, &qbit_conf.url, category).await?;
                        qbit.set_category(Some(hashes.clone()), category).await?;
                        info!("Set category '{}' for {} torrents", category, matched_count);
                    }

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
