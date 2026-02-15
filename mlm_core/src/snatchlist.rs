use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, bail};
use itertools::Itertools as _;
use mlm_db::{
    DatabaseExt, Event, EventType, MetadataSource, Timestamp, Torrent, TorrentKey, TorrentMeta,
    VipStatus,
};
use mlm_mam::{api::MaM, meta::MetaError, user_torrent::UserDetailsTorrent};
use mlm_parse::normalize_title;
use native_db::{Database, db_type, transaction::RwTransaction};
use time::UtcDateTime;
use tokio::{sync::MutexGuard, time::sleep};
use tracing::{Level, debug, enabled, info, instrument, trace, warn};
use uuid::Uuid;

use crate::config::{Config, Cost, SnatchlistSearch, TorrentFilter};
use crate::logging::write_event;

#[instrument(skip_all)]
pub async fn run_snatchlist_search(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    mam: Arc<MaM<'_>>,
    index: usize,
    snatchlist_config: Arc<SnatchlistSearch>,
    events: &crate::stats::Events,
) -> Result<()> {
    if !snatchlist_config.filter.edition.languages.is_empty() {
        bail!("Language filtering is not supported in snatchlist searches");
    }
    if snatchlist_config.filter.uploaded_after.is_some()
        || snatchlist_config.filter.uploaded_before.is_some()
    {
        bail!("Uploaded date filtering is not supported in snatchlist searches");
    }
    if snatchlist_config.cost != Cost::MetadataOnly
        && snatchlist_config.cost != Cost::MetadataOnlyAdd
    {
        bail!("Only metadata costs are supported in snatchlist searches");
    }

    let name = snatchlist_config
        .filter
        .name
        .clone()
        .unwrap_or_else(|| index.to_string());
    debug!("snatchlist {}", name);

    search_and_update_torrents(&config, &db, &snatchlist_config, &mam, events)
        .await
        .context("search_torrents")?;

    Ok(())
}

#[instrument(skip_all)]
async fn search_and_update_torrents(
    config: &Config,
    db: &Database<'_>,
    torrent_search: &SnatchlistSearch,
    mam: &MaM<'_>,
    events: &crate::stats::Events,
) -> Result<()> {
    let max_pages = torrent_search.max_pages.unwrap_or(100);
    let now = UtcDateTime::now();

    for page in 0.. {
        let page_results = mam
            .snatchlist(torrent_search.kind, page, now)
            .await
            .context("search")?;

        let row_count = page_results.rows.len();
        debug!(
            "result: rows: {}, success: {}",
            row_count, page_results.success,
        );

        if page_results.rows.is_empty() {
            break;
        }

        if enabled!(Level::TRACE) {
            trace!(
                "torrents in result: {:?}",
                page_results.rows.iter().map(|t| t.id).collect::<Vec<_>>()
            )
        }
        let torrents = page_results
            .rows
            .into_iter()
            .filter(|t| torrent_search.filter.matches_user(t));

        update_torrents(
            config,
            db,
            torrents,
            &torrent_search.filter,
            torrent_search.cost,
            torrent_search.dry_run,
            events,
        )
        .await
        .context("update_torrents")?;

        if page >= max_pages as u64 || row_count < 250 || row_count > 250 {
            break;
        }
        sleep(Duration::from_millis(400)).await;
    }

    Ok(())
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
async fn update_torrents<T: Iterator<Item = UserDetailsTorrent>>(
    config: &Config,
    db: &Database<'_>,
    torrents: T,
    grabber: &TorrentFilter,
    cost: Cost,
    dry_run: bool,
    events: &crate::stats::Events,
) -> Result<()> {
    'torrent: for torrent in torrents {
        if config.ignore_torrents.contains(&torrent.id) {
            trace!("Torrent {} is ignored", torrent.id);
            continue;
        }

        let meta = match torrent.as_meta() {
            Ok(it) => it,
            Err(err) => match err {
                MetaError::UnknownMediaType(_) => {
                    warn!("{err} for torrent {} {}", torrent.id, torrent.title);
                    continue;
                }
                _ => return Err(err.into()),
            },
        };
        if grabber.matches_meta(&meta).is_ok_and(|matches| !matches) {
            continue;
        }
        let rw_opt = if dry_run {
            None
        } else {
            Some(db.rw_async().await?)
        };
        if let Some((_, rw)) = &rw_opt {
            let old_library = rw
                .get()
                .secondary::<Torrent>(TorrentKey::mam_id, meta.mam_id())?;
            if let Some(old) = old_library {
                if old.meta != meta {
                    update_torrent_meta(
                        db,
                        rw_opt.unwrap(),
                        Some(&torrent),
                        old,
                        meta,
                        cost == Cost::MetadataOnlyAdd,
                        events,
                    )
                    .await?;
                }
                trace!("Torrent {} is already in library", torrent.id);
                continue 'torrent;
            }
        }
        if cost == Cost::MetadataOnlyAdd {
            let mam_id = torrent.id;
            add_metadata_only_torrent(rw_opt.unwrap(), torrent, meta)
                .await
                .or_else(|err| {
                    let err = err.downcast::<db_type::Error>()?;
                    if let db_type::Error::DuplicateKey { .. } = err {
                        warn!("Got dup key when adding torrent {}", mam_id);
                        Result::<(), anyhow::Error>::Ok(())
                    } else {
                        Err(err.into())
                    }
                })?;
            continue 'torrent;
        }
        if cost != Cost::MetadataOnly {
            warn!(
                "Ignoring cost {:?} for snatchlist torrent, only metadata updates supported",
                cost
            );
        }
        continue 'torrent;
    }
    Ok(())
}

#[instrument(skip_all)]
async fn add_metadata_only_torrent(
    (_guard, rw): (MutexGuard<'_, ()>, RwTransaction<'_>),
    torrent: UserDetailsTorrent,
    meta: TorrentMeta,
) -> Result<()> {
    info!("Adding metadata only torrent \"{}\"", meta.title);
    let id = Uuid::new_v4().to_string();

    let mam_id = torrent.id;
    {
        rw.insert(Torrent {
            id,
            id_is_hash: false,
            mam_id: Some(mam_id),
            library_path: None,
            library_files: Default::default(),
            linker: if torrent.uploader_name.is_empty() {
                None
            } else {
                Some(torrent.uploader_name)
            },
            category: None,
            selected_audio_format: None,
            selected_ebook_format: None,
            title_search: normalize_title(&meta.title),
            meta,
            created_at: Timestamp::now(),
            replaced_with: None,
            library_mismatch: None,
            client_status: None,
        })?;
        rw.commit()?;
    }

    Ok(())
}

async fn update_torrent_meta(
    db: &Database<'_>,
    (guard, rw): (MutexGuard<'_, ()>, RwTransaction<'_>),
    mam_torrent: Option<&UserDetailsTorrent>,
    mut torrent: Torrent,
    mut meta: TorrentMeta,
    linker_is_owner: bool,
    events: &crate::stats::Events,
) -> Result<()> {
    // These are missing in user details torrent response, so keep the old values
    meta.ids = torrent.meta.ids.clone();
    meta.media_type = torrent.meta.media_type;
    meta.main_cat = torrent.meta.main_cat;
    meta.language = torrent.meta.language;
    meta.tags = torrent.meta.tags.clone();
    meta.description = torrent.meta.description.clone();
    meta.num_files = torrent.meta.num_files;
    meta.uploaded_at = torrent.meta.uploaded_at;

    if torrent.meta.source != MetadataSource::Mam {
        // Update VIP status still
        if torrent.meta.vip_status != meta.vip_status {
            torrent.meta.vip_status = meta.vip_status;
            rw.upsert(torrent.clone())?;
            rw.commit()?;
        }
        return Ok(());
    }

    // Check expiring VIP
    if torrent.meta.vip_status != meta.vip_status
        && torrent
            .meta
            .vip_status
            .as_ref()
            .is_some_and(|s| !s.is_vip())
        && meta.vip_status == Some(VipStatus::NotVip)
    {
        torrent.meta.vip_status = meta.vip_status.clone();
        // If expiring VIP was the only change, just silently update the database
        if torrent.meta == meta {
            rw.upsert(torrent.clone())?;
            rw.commit()?;
            return Ok(());
        }
    }

    if linker_is_owner && torrent.linker.is_none() {
        if let Some(mam_torrent) = mam_torrent {
            torrent.linker = Some(mam_torrent.uploader_name.clone());
        }
    } else if meta == torrent.meta {
        return Ok(());
    }

    let id = torrent.id.clone();
    let diff = torrent.meta.diff(&meta);
    debug!(
        "Updating meta for torrent {}, diff:\n{}",
        id,
        diff.iter()
            .map(|field| format!("  {}: {} → {}", field.field, field.from, field.to))
            .join("\n")
    );
    torrent.meta = meta.clone();
    torrent.title_search = normalize_title(&meta.title);
    rw.upsert(torrent.clone())?;
    rw.commit()?;
    drop(guard);

    if !diff.is_empty() {
        let mam_id = mam_torrent.map(|m| m.id);
        write_event(
            db,
            events,
            Event::new(
                Some(id),
                mam_id,
                EventType::Updated {
                    fields: diff,
                    source: (meta.source.clone(), String::new()),
                },
            ),
        )
        .await;
    }
    Ok(())
}
