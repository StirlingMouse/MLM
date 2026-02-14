use std::time::Duration;

use anyhow::{Result, anyhow};
use bytes::Bytes;
use lava_torrent::torrent::v1::Torrent;
use mlm_db::{
    DatabaseExt as _, ErroredTorrentId, Event, EventType, SelectedTorrent, Size, Timestamp,
    TorrentCost,
};
use mlm_mam::api::{MaM, RateLimitError, WedgeBuyError};
use native_db::Database;
use qbit::{
    models::Torrent as QbitTorrent,
    parameters::{AddTorrent, AddTorrentType, TorrentFile, TorrentListParams, TorrentState},
};
use tokio::time::sleep;
use tracing::{debug, info, instrument, trace, warn};

use crate::{
    config::Config,
    logging::{TorrentMetaError, update_errored_torrent, write_event},
    qbittorrent::add_torrent_with_category,
};

#[instrument(skip_all)]
pub async fn grab_selected_torrents(
    config: &Config,
    db: &Database<'_>,
    qbit: &qbit::Api,
    qbit_url: &str,
    mam: &MaM<'_>,
) -> Result<()> {
    let selected_torrents = {
        let r = db.r_transaction()?;
        r.scan()
            .primary::<SelectedTorrent>()?
            .all()?
            .filter(|t| t.as_ref().is_ok_and(|t| t.removed_at.is_none()))
            .collect::<Result<Vec<_>, native_db::db_type::Error>>()
    }?;
    if selected_torrents.is_empty() {
        trace!("no selected torrents");
        return Ok(());
    }

    let user_info = mam.user_info().await?;
    let max_torrents = user_info.unsat.limit.saturating_sub(user_info.unsat.count);

    let downloading_size: f64 = selected_torrents
        .iter()
        .filter(|t| t.started_at.is_some())
        .map(|t| t.meta.size.bytes() as f64)
        .sum();

    let mut remaining_buffer =
        (user_info.uploaded_bytes - user_info.downloaded_bytes - downloading_size)
            / config.min_ratio;
    debug!(
        "downloader, unsats: {:#?}; max_torrents: {max_torrents}; buffer: {}",
        user_info.unsat,
        Size::from_bytes(remaining_buffer as u64)
    );

    let mut snatched_torrents = 0;
    for torrent in selected_torrents
        .into_iter()
        .filter(|t| t.started_at.is_none())
    {
        let max_torrents = max_torrents
            .saturating_sub(torrent.unsat_buffer.unwrap_or(config.unsat_buffer))
            .saturating_sub(snatched_torrents);
        if max_torrents == 0 {
            continue;
        }
        let buffer_after = remaining_buffer - torrent.meta.size.bytes() as f64;
        if buffer_after <= 0.0 {
            continue;
        }

        let result = grab_torrent(config, db, qbit, qbit_url, mam, torrent.clone())
            .await
            .map_err(|err| anyhow::Error::new(TorrentMetaError(torrent.meta.clone(), err)));

        if result.is_ok() {
            snatched_torrents += 1;
            remaining_buffer = buffer_after;
        }

        update_errored_torrent(
            db,
            ErroredTorrentId::Grabber(torrent.mam_id),
            torrent.meta.title,
            result,
        )
        .await;

        sleep(Duration::from_millis(1000)).await;
    }
    Ok(())
}

#[instrument(skip_all)]
async fn grab_torrent(
    config: &Config,
    db: &Database<'_>,
    qbit: &qbit::Api,
    qbit_url: &str,
    mam: &MaM<'_>,
    torrent: SelectedTorrent,
) -> Result<()> {
    info!(
        "Grabbing torrent \"{}\", with category {:?} and tags {:?}",
        torrent.meta.title, torrent.category, torrent.tags,
    );

    let user_info = mam.user_info().await?;
    let torrent_file_bytes = get_mam_torrent_file(mam, &torrent.dl_link).await?;
    let torrent_file = Torrent::read_from_bytes(torrent_file_bytes.clone())?;
    let hash = torrent_file.info_hash();

    if let Some(qbit_torrent) = get_existing_qbit_torrent(config, qbit, qbit_url, &hash).await {
        let is_completed = matches!(
            qbit_torrent.state,
            TorrentState::Uploading
                | TorrentState::StoppedUploading
                | TorrentState::QueuedUploading
                | TorrentState::StalledUploading
                | TorrentState::CheckingUploading
                | TorrentState::ForcedUploading
        );

        let (_guard, rw) = db.rw_async().await?;

        if is_completed {
            if rw.get().primary::<mlm_db::Torrent>(hash.clone())?.is_none() {
                rw.upsert(mlm_db::Torrent {
                    id: hash.clone(),
                    id_is_hash: true,
                    mam_id: torrent.meta.mam_id(),
                    library_path: None,
                    library_files: Default::default(),
                    linker: None,
                    category: torrent.category.clone(),
                    selected_audio_format: None,
                    selected_ebook_format: None,
                    title_search: torrent.title_search.clone(),
                    meta: torrent.meta.clone(),
                    created_at: Timestamp::now(),
                    replaced_with: None,
                    library_mismatch: None,
                    client_status: None,
                })?;
            }
            rw.remove(torrent)?;
        } else {
            let mut t = torrent;
            t.started_at = Some(Timestamp::now());
            rw.upsert(t)?;
        }
        rw.commit()?;

        return Ok(());
    }

    let library_existing = {
        let r = db.r_transaction()?;
        r.get().primary::<mlm_db::Torrent>(hash.clone())?
    };

    if let Some(existing) = library_existing
        && existing.library_path.is_some()
    {
        let (_guard, rw) = db.rw_async().await?;
        rw.remove(torrent)?;
        rw.commit()?;
        return Ok(());
    }

    let wedge_buffer = torrent.wedge_buffer.unwrap_or(config.wedge_buffer);
    let mut wedged = false;
    if torrent.cost == TorrentCost::UseWedge || torrent.cost == TorrentCost::TryWedge {
        if user_info.wedges <= wedge_buffer {
            return Err(anyhow::Error::msg(format!(
                "Fewer wedges ({}) than wedge_buffer ({})",
                user_info.wedges, wedge_buffer
            )));
        }
        info!("Using wedge on torrent \"{}\"", torrent.meta.title);
        match mam.wedge_torrent(torrent.mam_id).await {
            Ok(_) => {
                wedged = true;
                if let Some((_, user_info)) = mam.user.lock().await.as_mut() {
                    user_info.wedges = user_info.wedges.saturating_sub(1);
                }
            }
            Err(err) => {
                warn!(
                    "Failed applying wedge for torrent {}: {}",
                    torrent.mam_id, err
                );
                match err.downcast::<WedgeBuyError>() {
                    Ok(
                        WedgeBuyError::IsVip
                        | WedgeBuyError::IsGlobalFreeleech
                        | WedgeBuyError::IsPersonalFreeleech,
                    ) => {}
                    _ => {
                        if torrent.cost == TorrentCost::UseWedge {
                            return Err(anyhow!("Failed to apply wedge for torrent"));
                        }
                    }
                }
            }
        }
    } else if torrent.cost != TorrentCost::Ratio {
        let Some(torrent_info) = mam.get_torrent_info(&hash).await? else {
            return Err(anyhow!("Could not get torrent from MaM"));
        };
        if !torrent_info.is_free() {
            return Err(anyhow!(
                "Torrent is no longer free, expected: {:?}",
                torrent.cost
            ));
        }
    }

    mam.add_unsats(1).await;
    add_torrent_with_category(
        qbit,
        qbit_url,
        AddTorrent {
            torrents: AddTorrentType::Files(vec![TorrentFile {
                filename: format!("{}.torrent", torrent.mam_id),
                data: torrent_file_bytes.iter().copied().collect(),
            }]),
            stopped: config.add_torrents_stopped,
            category: torrent.category.clone(),
            tags: if torrent.tags.is_empty() {
                None
            } else {
                Some(torrent.tags.clone())
            },
            ..Default::default()
        },
    )
    .await?;

    let mam_id = torrent.mam_id;
    let cost = Some(torrent.cost);
    let grabber = torrent.grabber.clone();
    {
        let (_guard, rw) = db.rw_async().await?;
        rw.upsert(mlm_db::Torrent {
            id: hash.clone(),
            id_is_hash: true,
            mam_id: torrent.meta.mam_id(),
            library_path: None,
            library_files: Default::default(),
            linker: None,
            category: torrent.category.clone(),
            selected_audio_format: None,
            selected_ebook_format: None,
            title_search: torrent.title_search.clone(),
            meta: torrent.meta.clone(),
            created_at: Timestamp::now(),
            replaced_with: None,
            library_mismatch: None,
            client_status: None,
        })?;
        let mut torrent = torrent;
        torrent.hash = Some(hash.clone());
        torrent.started_at = Some(Timestamp::now());
        rw.upsert(torrent)?;
        rw.commit()?;
    }

    write_event(
        db,
        Event::new(
            Some(hash),
            Some(mam_id),
            EventType::Grabbed {
                grabber,
                cost,
                wedged,
            },
        ),
    )
    .await;

    Ok(())
}

async fn get_existing_qbit_torrent(
    config: &Config,
    qbit: &qbit::Api,
    qbit_url: &str,
    hash: &str,
) -> Option<QbitTorrent> {
    if let Ok(Some(qbit_torrent)) = qbit
        .torrents(Some(TorrentListParams {
            hashes: Some(vec![hash.to_string()]),
            ..TorrentListParams::default()
        }))
        .await
        .map(|t| t.into_iter().next())
    {
        return Some(qbit_torrent);
    }

    for qbit_conf in config.qbittorrent.iter().filter(|q| q.url != qbit_url) {
        let Ok(qbit) = qbit::Api::new_login_username_password(
            &qbit_conf.url,
            &qbit_conf.username,
            &qbit_conf.password,
        )
        .await
        else {
            continue;
        };

        if let Ok(Some(qbit_torrent)) = qbit
            .torrents(Some(TorrentListParams {
                hashes: Some(vec![hash.to_string()]),
                ..TorrentListParams::default()
            }))
            .await
            .map(|t| t.into_iter().next())
        {
            return Some(qbit_torrent);
        }
    }

    None
}

pub(crate) async fn get_mam_torrent_file(mam: &MaM<'_>, dl_link: &str) -> Result<Bytes> {
    loop {
        let result = mam.get_torrent_file(dl_link).await;

        match result {
            Ok(v) => return Ok(v),
            Err(e) => match e.downcast::<RateLimitError>() {
                Ok(_) => {
                    sleep(Duration::from_millis(30_000)).await;
                }
                Err(e) => return Err(e),
            },
        };
    }
}
