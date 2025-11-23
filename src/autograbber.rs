use std::{
    fs::File,
    io::{BufWriter, Write as _},
    ops::RangeInclusive,
    sync::Arc,
    time::Duration,
};

use crate::{
    audiobookshelf::{self as abs, Abs},
    config::{Config, Cost, SortBy, TorrentFilter, TorrentSearch, Type},
    data::{
        self, DuplicateTorrent, ErroredTorrentId, Event, EventType, MetadataSource,
        SelectedTorrent, Size, Timestamp, TorrentCost, TorrentKey, TorrentMeta, VipStatus,
    },
    logging::{TorrentMetaError, update_errored_torrent, write_event},
    mam::{
        api::{MaM, RateLimitError, WedgeBuyError},
        enums::{SearchKind, SearchTarget},
        meta::{MetaError, normalize_title},
        search::{MaMTorrent, SearchQuery, SearchResult, Tor},
        serde::DATE_FORMAT,
        user_data::UserResponse,
    },
};
use anyhow::{Context, Error, Result};
use bytes::Bytes;
use itertools::Itertools as _;
use lava_torrent::torrent::v1::Torrent;
use native_db::{Database, db_type, transaction::RwTransaction};
use qbit::parameters::{AddTorrent, AddTorrentType, TorrentFile};
use tokio::{fs, sync::watch::Sender, time::sleep};
use tracing::{Level, debug, enabled, error, info, instrument, trace, warn};
use uuid::Uuid;

#[instrument(skip_all)]
pub async fn run_autograbber(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    mam: Arc<MaM<'_>>,
    autograb_trigger: Sender<()>,
    index: usize,
    autograb_config: Arc<TorrentSearch>,
) -> Result<()> {
    let user_info = mam.user_info().await?;
    let max_torrents = user_info.unsat.limit.saturating_sub(user_info.unsat.count);
    let name = autograb_config
        .filter
        .name
        .clone()
        .unwrap_or_else(|| index.to_string());
    debug!(
        "autograbber {}, unsats: {:#?}; max_torrents: {max_torrents}",
        name, user_info.unsat
    );

    let unsat_buffer = autograb_config.unsat_buffer.unwrap_or(config.unsat_buffer);
    let mut max_torrents = max_torrents.saturating_sub(unsat_buffer);

    if max_torrents > 0
        && let Some(max_active_downloads) = autograb_config.max_active_downloads
    {
        let r = db.r_transaction()?;
        let downloading_torrents = r
            .scan()
            .primary::<SelectedTorrent>()?
            .all()?
            .filter(|t| t.as_ref().is_ok_and(|t| t.grabber.as_ref() == Some(&name)))
            .count() as u64;
        max_torrents = max_torrents.min(max_active_downloads.saturating_sub(downloading_torrents));
    }

    if max_torrents > 0
        || autograb_config.cost == Cost::MetadataOnly
        || autograb_config.cost == Cost::MetadataOnlyAdd
    {
        let selected_torrents =
            search_and_select_torrents(&config, &db, &autograb_config, &mam, max_torrents)
                .await
                .context("search_torrents")?;
        mam.add_unsats(selected_torrents).await;
    }

    autograb_trigger.send(())?;

    Ok(())
}

#[instrument(skip_all)]
pub async fn grab_selected_torrents(
    config: &Config,
    db: &Database<'_>,
    qbit: &qbit::Api,
    mam: &MaM<'_>,
) -> Result<()> {
    let selected_torrents = {
        let r = db.r_transaction()?;
        r.scan()
            .primary::<data::SelectedTorrent>()?
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

        let result = grab_torrent(config, db, qbit, mam, &user_info, torrent.clone())
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
        );

        sleep(Duration::from_millis(1000)).await;
    }
    Ok(())
}

#[instrument(skip_all)]
pub async fn search_and_select_torrents(
    config: &Config,
    db: &Database<'_>,
    torrent_search: &TorrentSearch,
    mam: &MaM<'_>,
    max_torrents: u64,
) -> Result<u64> {
    let torrents = search_torrents(torrent_search, mam).await?;

    select_torrents(
        config,
        db,
        mam,
        torrents,
        &torrent_search.filter,
        torrent_search.cost,
        torrent_search.unsat_buffer,
        torrent_search.category.clone(),
        torrent_search.dry_run,
        max_torrents,
        None,
    )
    .await
    .context("select_torrents")
}

#[instrument(skip_all)]
pub async fn search_torrents(
    torrent_search: &TorrentSearch,
    mam: &MaM<'_>,
) -> Result<impl Iterator<Item = MaMTorrent>> {
    let target = match torrent_search.kind {
        Type::Bookmarks => Some(SearchTarget::Bookmarks),
        Type::Mine => Some(SearchTarget::Mine),
        Type::Uploader(id) => Some(SearchTarget::Uploader(id)),
        _ => None,
    };
    let kind = match (torrent_search.kind, torrent_search.cost) {
        (Type::Freeleech, _) => Some(SearchKind::Freeleech),
        (_, Cost::Free) => Some(SearchKind::Free),
        _ => None,
    };
    let sort_type = torrent_search
        .sort_by
        .map(|sort_by| match sort_by {
            SortBy::LowSeeders => "seedersAsc",
            SortBy::LowSnatches => "snatchedAsc",
            SortBy::OldestFirst => "dateAsc",
            SortBy::Random => "random",
        })
        .unwrap_or(match torrent_search.kind {
            Type::New => "dateDesc",
            _ => "",
        });
    let (flags_is_hide, flags) = torrent_search.filter.flags.as_search_bitfield();
    let max_pages = torrent_search
        .max_pages
        .unwrap_or(match torrent_search.kind {
            Type::Bookmarks | Type::Freeleech | Type::Mine => 50,
            _ => 0,
        });

    let mut results: Option<SearchResult> = None;
    for page in 1.. {
        let mut page_results = mam
            .search(&SearchQuery {
                dl_link: true,
                perpage: 100,
                tor: Tor {
                    start_number: results.as_ref().map_or(0, |r| r.data.len() as u64),
                    target,
                    kind,
                    text: &torrent_search.query.clone().unwrap_or_default(),
                    srch_in: torrent_search.search_in.clone(),
                    main_cat: torrent_search.filter.categories.get_main_cats(),
                    cat: torrent_search.filter.categories.get_cats(),
                    browse_lang: torrent_search
                        .filter
                        .languages
                        .iter()
                        .map(|l| l.to_id())
                        .collect(),
                    browse_flags_hide_vs_show: if flags.is_empty() {
                        None
                    } else {
                        Some(if flags_is_hide { 0 } else { 1 })
                    },
                    browse_flags: flags.clone(),
                    start_date: torrent_search
                        .filter
                        .uploaded_after
                        .map_or_else(|| Ok(String::new()), |d| d.format(&DATE_FORMAT))?,
                    end_date: torrent_search
                        .filter
                        .uploaded_before
                        .map_or_else(|| Ok(String::new()), |d| d.format(&DATE_FORMAT))?,
                    min_size: torrent_search.filter.min_size.bytes(),
                    max_size: torrent_search.filter.max_size.bytes(),
                    unit: torrent_search
                        .filter
                        .min_size
                        .unit()
                        .max(torrent_search.filter.max_size.unit()),
                    min_seeders: torrent_search.filter.min_seeders,
                    max_seeders: torrent_search.filter.max_seeders,
                    min_leechers: torrent_search.filter.min_leechers,
                    max_leechers: torrent_search.filter.max_leechers,
                    min_snatched: torrent_search.filter.min_snatched,
                    max_snatched: torrent_search.filter.max_snatched,
                    sort_type,
                    ..Default::default()
                },

                ..Default::default()
            })
            .await
            .context("search")?;

        debug!(
            "result: perpage: {}, start: {}, data: {}, total: {}, found: {}",
            page_results.perpage,
            page_results.start,
            page_results.data.len(),
            page_results.total,
            page_results.found
        );

        if page_results.data.is_empty() {
            if results.is_none() {
                results = Some(page_results);
            }
            break;
        }

        if enabled!(Level::TRACE) {
            trace!(
                "torrents in result: {:?}",
                page_results.data.iter().map(|t| t.id).collect::<Vec<_>>()
            )
        }
        if let Some(results) = &mut results {
            results.data.append(&mut page_results.data);
        } else {
            results = Some(page_results);
        }

        let results = results.as_ref().unwrap();
        if page >= max_pages || results.data.len() >= results.found {
            break;
        }
        sleep(Duration::from_millis(400)).await;
    }

    let torrents = results
        .unwrap()
        .data
        .into_iter()
        .filter(|t| torrent_search.filter.matches(t));

    Ok(torrents)
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn select_torrents<T: Iterator<Item = MaMTorrent>>(
    config: &Config,
    db: &Database<'_>,
    mam: &MaM<'_>,
    torrents: T,
    grabber: &TorrentFilter,
    cost: Cost,
    unsat_buffer: Option<u64>,
    filter_category: Option<String>,
    dry_run: bool,
    max_torrents: u64,
    goodreads_id: Option<u64>,
) -> Result<u64> {
    let mut selected_torrents = 0;
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
        let rw_opt = if dry_run {
            None
        } else {
            Some(db.rw_transaction()?)
        };
        if let Some(rw) = &rw_opt
            && let Some(old_selected) = rw
                .get()
                .primary::<data::SelectedTorrent>(torrent.id)
                .ok()
                .flatten()
        {
            if let Some(unsat_buffer) = unsat_buffer
                && old_selected.unsat_buffer.is_none_or(|u| unsat_buffer < u)
            {
                let mut updated = old_selected.clone();
                updated.unsat_buffer = Some(unsat_buffer);
                if updated.meta != meta {
                    update_selected_torrent_meta(db, rw_opt.unwrap(), mam, updated, meta).await?;
                } else {
                    rw.update(old_selected, updated)?;
                    rw_opt.unwrap().commit()?;
                }
            } else if old_selected.meta != meta {
                update_selected_torrent_meta(db, rw_opt.unwrap(), mam, old_selected, meta).await?;
            }
            trace!("Torrent {} is already selected", torrent.id);
            continue;
        }
        let title_search = normalize_title(&torrent.title);
        let preferred_types = meta.media_type.preferred_types(config);
        let preference = preferred_types
            .iter()
            .position(|t| meta.filetypes.contains(t));
        if preference.is_none() {
            trace!(
                "Could not find any wanted formats in torrent {}",
                meta.mam_id
            );
            continue;
        }
        if let Some(rw) = &rw_opt {
            let old_library = rw
                .get()
                .secondary::<data::Torrent>(TorrentKey::mam_id, meta.mam_id)?;
            if let Some(old) = old_library {
                if old.meta != meta {
                    update_torrent_meta(config, db, rw_opt.unwrap(), &torrent, old, meta, false)
                        .await?;
                }
                trace!("Torrent {} is already in library", torrent.id);
                continue 'torrent;
            }
        }
        if cost == Cost::MetadataOnlyAdd {
            add_metadata_only_torrent(rw_opt.unwrap(), torrent, meta).await?;
            continue 'torrent;
        }
        if cost == Cost::MetadataOnly {
            continue 'torrent;
        }
        if let Some(rw) = &rw_opt {
            let old_selected = {
                rw.scan()
                    .secondary::<data::SelectedTorrent>(data::SelectedTorrentKey::title_search)?
                    .range::<RangeInclusive<&str>>(title_search.as_str()..=title_search.as_str())?
                    .collect::<Result<Vec<_>, native_db::db_type::Error>>()
            }?;
            for old in old_selected {
                if old.mam_id == meta.mam_id {
                    if old.meta != meta {
                        update_selected_torrent_meta(db, rw_opt.unwrap(), mam, old, meta).await?;
                    }
                    trace!("Torrent {} is already selected2", torrent.id);
                    continue 'torrent;
                }
                trace!(
                    "Checking old torrent {} with formats {:?}",
                    old.title_search, old.meta.filetypes
                );
                if meta.matches(&old.meta) {
                    let old_preference = preferred_types
                        .iter()
                        .position(|t| old.meta.filetypes.contains(t));
                    if old_preference <= preference {
                        if let Err(err) =
                            add_duplicate_torrent(rw, None, torrent.dl.clone(), title_search, meta)
                        {
                            error!("Error writing duplicate torrent: {err}");
                        }
                        rw_opt.unwrap().commit()?;
                        trace!(
                            "Skipping torrent {} as we have {} selected",
                            torrent.id, old.meta.mam_id
                        );
                        continue 'torrent;
                    } else {
                        if let Err(err) = add_duplicate_torrent(
                            rw,
                            None,
                            torrent.dl.clone(),
                            title_search.clone(),
                            old.meta.clone(),
                        ) {
                            error!("Error writing duplicate torrent: {err}");
                        }
                        info!(
                            "Unselecting torrent \"{}\" with formats {:?}",
                            old.meta.title, old.meta.filetypes
                        );
                        rw.remove(old)?;
                    }
                }
            }
        }
        if let Some(rw) = &rw_opt {
            let old_library = {
                rw.scan()
                    .secondary::<data::Torrent>(data::TorrentKey::title_search)?
                    .range::<RangeInclusive<&str>>(title_search.as_str()..=title_search.as_str())?
                    .collect::<Result<Vec<_>, native_db::db_type::Error>>()
            }?;
            for old in old_library {
                if old.meta.mam_id == meta.mam_id {
                    if old.meta != meta {
                        update_torrent_meta(
                            config,
                            db,
                            rw_opt.unwrap(),
                            &torrent,
                            old,
                            meta,
                            false,
                        )
                        .await?;
                    }
                    trace!("Torrent {} is already in library2", torrent.id);
                    continue 'torrent;
                }
                trace!(
                    "Checking old torrent {} with formats {:?}",
                    old.title_search, old.meta.filetypes
                );
                if meta.matches(&old.meta) {
                    let old_preference = preferred_types
                        .iter()
                        .position(|t| old.meta.filetypes.contains(t));
                    if old_preference <= preference {
                        if let Err(err) = add_duplicate_torrent(
                            rw,
                            Some(old.hash),
                            torrent.dl.clone(),
                            title_search,
                            meta,
                        ) {
                            error!("Error writing duplicate torrent: {err}");
                        }
                        rw_opt.unwrap().commit()?;
                        trace!(
                            "Skipping torrent {} as we have {} in libary",
                            torrent.id, old.meta.mam_id
                        );
                        continue 'torrent;
                    } else {
                        info!(
                            "Selecting replacement for library torrent \"{}\" with formats {:?}",
                            old.meta.title, old.meta.filetypes
                        );
                    }
                }
            }
        }
        let tags: Vec<_> = config
            .tags
            .iter()
            .filter(|t| t.filter.matches(&torrent))
            .collect();
        let category = filter_category
            .clone()
            .or_else(|| tags.iter().find_map(|t| t.category.clone()));
        let tags = tags.iter().flat_map(|t| t.tags.clone()).collect();
        let cost = if torrent.vip {
            TorrentCost::Vip
        } else if torrent.personal_freeleech {
            TorrentCost::PersonalFreeleech
        } else if torrent.free {
            TorrentCost::GlobalFreeleech
        } else if cost == Cost::Wedge {
            TorrentCost::UseWedge
        } else if cost == Cost::TryWedge {
            TorrentCost::TryWedge
        } else {
            TorrentCost::Ratio
        };
        info!(
            "Selecting torrent \"{}\" in format {}, cost: {:?}, with category {:?} and tags {:?}",
            torrent.title, torrent.filetype, cost, category, tags
        );
        if let Some(rw) = &rw_opt {
            selected_torrents += 1;
            rw.insert(data::SelectedTorrent {
                mam_id: torrent.id,
                goodreads_id,
                hash: None,
                dl_link: torrent
                    .dl
                    .clone()
                    .ok_or_else(|| Error::msg(format!("no dl field for torrent {}", torrent.id)))?,
                unsat_buffer,
                cost,
                category,
                tags,
                title_search,
                meta,
                grabber: grabber.name.clone(),
                created_at: Timestamp::now(),
                started_at: None,
                removed_at: None,
            })?;
            rw_opt.unwrap().commit()?;
            if selected_torrents >= max_torrents {
                break;
            }
        }
    }

    Ok(selected_torrents)
}

#[instrument(skip_all)]
async fn grab_torrent(
    config: &Config,
    db: &Database<'_>,
    qbit: &qbit::Api,
    mam: &MaM<'_>,
    user_info: &UserResponse,
    torrent: SelectedTorrent,
) -> Result<()> {
    info!(
        "Grabbing torrent \"{}\", with category {:?} and tags {:?}",
        torrent.meta.title, torrent.category, torrent.tags,
    );

    let torrent_file_bytes = get_mam_torrent_file(mam, &torrent.dl_link).await?;
    let torrent_file = Torrent::read_from_bytes(torrent_file_bytes.clone())?;
    let hash = torrent_file.info_hash();

    let mut wedged = false;
    if torrent.cost == TorrentCost::UseWedge || torrent.cost == TorrentCost::TryWedge {
        if user_info.wedges < config.wedge_buffer {
            return Err(anyhow::Error::msg(format!(
                "Fewer wedges ({}) than wedge_buffer ({})",
                user_info.wedges, config.wedge_buffer
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
                            return Err(anyhow::Error::msg("Failed to apply wedge for torrent"));
                        }
                    }
                }
            }
        }
    } else if torrent.cost != TorrentCost::Ratio {
        let Some(torrent_info) = mam.get_torrent_info(&hash).await? else {
            return Err(anyhow::Error::msg("Could not get torrent from MaM"));
        };
        if !torrent_info.is_free() {
            return Err(anyhow::Error::msg(format!(
                "Torrent is no longer free, expected: {:?}",
                torrent.cost
            )));
        }
    }

    qbit.add_torrent(AddTorrent {
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
    })
    .await?;

    let mam_id = torrent.mam_id;
    let cost = Some(torrent.cost);
    let grabber = torrent.grabber.clone();
    {
        let rw = db.rw_transaction()?;
        rw.insert(data::Torrent {
            hash: hash.clone(),
            mam_id: torrent.meta.mam_id,
            abs_id: None,
            goodreads_id: torrent.goodreads_id,
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
            request_matadata_update: false,
            library_mismatch: None,
            client_status: None,
        })
        .or_else(|err| {
            if let db_type::Error::DuplicateKey { .. } = err {
                warn!("Got dup key when adding torrent {:?}", torrent);
                Ok(())
            } else {
                Err(err)
            }
        })?;
        let mut torrent = torrent;
        torrent.hash = Some(hash.clone());
        torrent.started_at = Some(Timestamp::now());
        rw.upsert(torrent).map(|_| ()).or_else(|err| {
            if let db_type::Error::KeyNotFound { .. } = err {
                warn!("Got missing key when updating selected torrent");
                Ok(())
            } else {
                Err(err)
            }
        })?;
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
    );

    Ok(())
}

#[instrument(skip_all)]
async fn add_metadata_only_torrent(
    rw: RwTransaction<'_>,
    torrent: MaMTorrent,
    meta: TorrentMeta,
) -> Result<()> {
    info!("Adding metadata only torrent \"{}\"", meta.title,);
    // let Some(dl_link) = torrent.dl.as_ref() else {
    //     bail!("missing dl_link");
    // };
    //
    // let torrent_file_bytes = get_mam_torrent_file(mam, dl_link).await?;
    // let torrent_file = Torrent::read_from_bytes(torrent_file_bytes.clone())?;
    // let hash = torrent_file.info_hash();
    let hash = Uuid::new_v4().to_string();

    let mam_id = torrent.id;
    {
        rw.insert(data::Torrent {
            hash,
            mam_id,
            abs_id: None,
            goodreads_id: None,
            library_path: None,
            library_files: Default::default(),
            linker: None,
            category: None,
            selected_audio_format: None,
            selected_ebook_format: None,
            title_search: normalize_title(&meta.title),
            meta,
            created_at: Timestamp::now(),
            replaced_with: None,
            request_matadata_update: false,
            library_mismatch: None,
            client_status: None,
        })
        .or_else(|err| {
            if let db_type::Error::DuplicateKey { .. } = err {
                warn!("Got dup key when adding torrent {:?}", torrent);
                Ok(())
            } else {
                Err(err)
            }
        })?;
        rw.commit()?;
    }

    Ok(())
}

pub async fn update_torrent_meta(
    config: &Config,
    db: &Database<'_>,
    rw: RwTransaction<'_>,
    mam_torrent: &MaMTorrent,
    mut torrent: data::Torrent,
    meta: TorrentMeta,
    allow_non_mam: bool,
) -> Result<()> {
    if !allow_non_mam && torrent.meta.source != MetadataSource::Mam {
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

    let hash = torrent.hash.clone();
    let mam_id = meta.mam_id;
    let diff = torrent.meta.diff(&meta);
    debug!(
        "Updating meta for torrent {}, diff:\n{}",
        mam_id,
        diff.iter()
            .map(|field| format!("  {}: {} → {}", field.field, field.from, field.to))
            .join("\n")
    );
    torrent.meta = meta.clone();
    torrent.title_search = normalize_title(&meta.title);
    rw.upsert(torrent.clone())?;
    rw.commit()?;

    if let Some(library_path) = &torrent.library_path
        && let serde_json::Value::Object(new) = abs::create_metadata(mam_torrent, &meta)
    {
        let metadata_path = library_path.join("metadata.json");
        if metadata_path.exists() {
            let existing = fs::read_to_string(&metadata_path).await?;
            let mut existing: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&existing)?;
            for (key, value) in new {
                existing.insert(key, value);
            }
            let file = File::create(&metadata_path)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer(&mut writer, &serde_json::Value::Object(existing))?;
            writer.flush()?;
            debug!("updated ABS metadata file {}", torrent.meta.mam_id);
        }
        if let (Some(abs_id), Some(abs_config)) = (&torrent.abs_id, &config.audiobookshelf) {
            let abs = Abs::new(abs_config)?;
            match abs.update_book(abs_id, mam_torrent, &meta).await {
                Ok(_) => debug!("updated ABS via API {}", torrent.meta.mam_id),
                Err(err) => warn!("Failed updating book {} in abs: {err}", torrent.meta.mam_id),
            }
        }
    }

    write_event(
        db,
        Event::new(
            Some(hash),
            Some(mam_id),
            EventType::Updated { fields: diff },
        ),
    );
    Ok(())
}

async fn update_selected_torrent_meta(
    db: &Database<'_>,
    rw: RwTransaction<'_>,
    mam: &MaM<'_>,
    torrent: SelectedTorrent,
    meta: TorrentMeta,
) -> Result<()> {
    let mam_id = meta.mam_id;
    let diff = torrent.meta.diff(&meta);
    debug!(
        "Updating meta for selected torrent {}, diff:\n{}",
        mam_id,
        diff.iter()
            .map(|field| format!("  {}: {} -> {}", field.field, field.from, field.to))
            .join("\n")
    );
    let hash = get_mam_torrent_hash(mam, &torrent.dl_link).await.ok();
    let mut torrent = torrent;
    torrent.meta = meta;
    rw.upsert(torrent)?;
    rw.commit()?;
    write_event(
        db,
        Event::new(hash, Some(mam_id), EventType::Updated { fields: diff }),
    );
    Ok(())
}

pub async fn get_mam_torrent_hash(mam: &MaM<'_>, dl_link: &str) -> Result<String> {
    let torrent_file_bytes = get_mam_torrent_file(mam, dl_link).await?;
    let torrent_file = Torrent::read_from_bytes(torrent_file_bytes.clone())?;
    let hash = torrent_file.info_hash();
    Ok(hash)
}

async fn get_mam_torrent_file(mam: &MaM<'_>, dl_link: &str) -> Result<Bytes> {
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

fn add_duplicate_torrent(
    rw: &RwTransaction<'_>,
    duplicate_of: Option<String>,
    dl_link: Option<String>,
    title_search: String,
    meta: TorrentMeta,
) -> Result<()> {
    rw.upsert(DuplicateTorrent {
        mam_id: meta.mam_id,
        dl_link,
        title_search,
        meta,
        created_at: Timestamp::now(),
        duplicate_of,
    })?;
    Ok(())
}
