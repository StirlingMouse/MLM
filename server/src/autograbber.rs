use std::{
    fs::File,
    io::{BufWriter, Write as _},
    ops::RangeInclusive,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Error, Result};
use itertools::Itertools as _;
use lava_torrent::torrent::v1::Torrent;
use mlm_db::{
    ClientStatus, DatabaseExt as _, DuplicateTorrent, Event, EventType, MetadataSource,
    SelectedTorrent, Timestamp, TorrentCost, TorrentKey, TorrentMeta, VipStatus, ids,
};
use mlm_mam::{
    api::MaM,
    enums::{SearchKind, SearchTarget},
    meta::MetaError,
    search::{MaMTorrent, SearchFields, SearchQuery, SearchResult, Tor},
    serde::DATE_FORMAT,
};
use mlm_parse::normalize_title;
use native_db::{Database, db_type, transaction::RwTransaction};
use tokio::{
    fs,
    sync::{MutexGuard, watch::Sender},
    time::sleep,
};
use tracing::{Level, debug, enabled, error, info, instrument, trace, warn};
use uuid::Uuid;

use crate::{
    audiobookshelf::{self as abs, Abs},
    config::{Config, Cost, SortBy, TorrentFilter, TorrentSearch, Type},
    logging::write_event,
    torrent_downloader::get_mam_torrent_file,
};

static AUTOGRABBER_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[instrument(skip_all)]
pub async fn run_autograbber(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    mam: Arc<MaM<'_>>,
    autograb_trigger: Sender<()>,
    index: usize,
    autograb_config: Arc<TorrentSearch>,
) -> Result<()> {
    // Make sure we are only running one autograbber at a time
    let _guard = AUTOGRABBER_MUTEX.lock().await;

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
            .filter(|t| {
                t.as_ref()
                    .is_ok_and(|t| t.grabber.as_ref() == Some(&name) && t.removed_at.is_none())
            })
            .count() as u64;
        max_torrents = max_torrents.min(max_active_downloads.saturating_sub(downloading_torrents));
    }

    if max_torrents > 0
        || autograb_config.cost == Cost::MetadataOnly
        || autograb_config.cost == Cost::MetadataOnlyAdd
    {
        search_and_select_torrents(
            &config,
            &db,
            &autograb_config,
            SearchFields {
                dl_link: true,
                ..Default::default()
            },
            &mam,
            max_torrents,
        )
        .await
        .context("search_torrents")?;
    }

    if !config.qbittorrent.is_empty() {
        autograb_trigger.send(())?;
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn search_and_select_torrents(
    config: &Config,
    db: &Database<'_>,
    torrent_search: &TorrentSearch,
    fields: SearchFields,
    mam: &MaM<'_>,
    max_torrents: u64,
) -> Result<u64> {
    let torrents = search_torrents(torrent_search, fields, mam)
        .await
        .context("search_torrents")?;

    if torrent_search.mark_removed {
        let torrents = torrents.collect::<Vec<_>>();
        mark_removed_torrents(db, mam, &torrents)
            .await
            .context("mark_removed_torrents")?;

        return select_torrents(
            config,
            db,
            mam,
            torrents.into_iter(),
            &torrent_search.filter,
            torrent_search.cost,
            torrent_search.unsat_buffer,
            torrent_search.wedge_buffer,
            torrent_search.category.clone(),
            torrent_search.dry_run,
            max_torrents,
            None,
        )
        .await
        .context("select_torrents");
    }

    select_torrents(
        config,
        db,
        mam,
        torrents,
        &torrent_search.filter,
        torrent_search.cost,
        torrent_search.unsat_buffer,
        torrent_search.wedge_buffer,
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
    fields: SearchFields,
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
    let (flags_is_hide, flags) = torrent_search.filter.edition.flags.as_search_bitfield();
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
                fields,
                perpage: 100,
                tor: Tor {
                    start_number: results.as_ref().map_or(0, |r| r.data.len() as u64),
                    target,
                    kind,
                    text: torrent_search.query.clone().unwrap_or_default(),
                    srch_in: torrent_search.search_in.clone(),
                    main_cat: torrent_search.filter.edition.categories.get_main_cats(),
                    cat: torrent_search.filter.edition.categories.get_cats(),
                    browse_lang: torrent_search
                        .filter
                        .edition
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
                    min_size: torrent_search.filter.edition.min_size.bytes(),
                    max_size: torrent_search.filter.edition.max_size.bytes(),
                    unit: torrent_search
                        .filter
                        .edition
                        .min_size
                        .unit()
                        .max(torrent_search.filter.edition.max_size.unit()),
                    min_seeders: torrent_search.filter.min_seeders,
                    max_seeders: torrent_search.filter.max_seeders,
                    min_leechers: torrent_search.filter.min_leechers,
                    max_leechers: torrent_search.filter.max_leechers,
                    min_snatched: torrent_search.filter.min_snatched,
                    max_snatched: torrent_search.filter.max_snatched,
                    sort_type: sort_type.to_string(),
                    ..Default::default()
                },
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
pub async fn mark_removed_torrents(
    db: &Database<'_>,
    mam: &MaM<'_>,
    torrents: &[MaMTorrent],
) -> Result<()> {
    if let (Some(first), Some(last)) = (torrents.first(), torrents.last()) {
        let ids = first.id.min(last.id)..=first.id.max(last.id);
        for id in ids {
            let is_removed = torrents.iter().all(|t| t.id != id);
            if is_removed {
                let (guard, rw) = db.rw_async().await?;
                let torrent = rw
                    .get()
                    .secondary::<mlm_db::Torrent>(TorrentKey::mam_id, id)?;
                if let Some(mut torrent) = torrent
                    && torrent.client_status != Some(ClientStatus::RemovedFromTracker)
                {
                    if mam.get_torrent_info_by_id(id).await?.is_none() {
                        torrent.client_status = Some(ClientStatus::RemovedFromTracker);
                        let tid = Some(torrent.id.clone());
                        rw.upsert(torrent)?;
                        rw.commit()?;
                        drop(guard);
                        write_event(db, Event::new(tid, Some(id), EventType::RemovedFromTracker))
                            .await;
                    }
                    sleep(Duration::from_millis(400)).await;
                }
            }
        }
    }
    Ok(())
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
    wedge_buffer: Option<u64>,
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

        let mut meta = match torrent.as_meta() {
            Ok(it) => it,
            Err(err) => match err {
                MetaError::UnknownMediaType(_) => {
                    warn!("{err} for torrent {} {}", torrent.id, torrent.title);
                    continue;
                }
                _ => return Err(err.into()),
            },
        };
        if let Some(goodreads_id) = goodreads_id {
            meta.ids
                .insert(ids::GOODREADS.to_string(), goodreads_id.to_string());
        }
        let rw_opt = if dry_run {
            None
        } else {
            Some(db.rw_async().await?)
        };
        if let Some((_, rw)) = &rw_opt
            && let Some(old_selected) = rw
                .get()
                .primary::<mlm_db::SelectedTorrent>(torrent.id)
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
                    rw_opt.unwrap().1.commit()?;
                }
            } else if old_selected.meta != meta {
                update_selected_torrent_meta(db, rw_opt.unwrap(), mam, old_selected, meta).await?;
            }
            trace!("Torrent {} is already selected", torrent.id);
            continue;
        }
        if let Some((_, rw)) = &rw_opt {
            let old_library = rw
                .get()
                .secondary::<mlm_db::Torrent>(TorrentKey::mam_id, meta.mam_id())?;
            if let Some(old) = old_library {
                if old.meta != meta
                    || (cost == Cost::MetadataOnlyAdd
                        && old.linker.is_none()
                        && !torrent.owner_name.is_empty())
                {
                    update_torrent_meta(
                        config,
                        db,
                        rw_opt.unwrap(),
                        Some(&torrent),
                        old,
                        meta,
                        false,
                        cost == Cost::MetadataOnlyAdd,
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
        if cost == Cost::MetadataOnly {
            continue 'torrent;
        }
        let title_search = normalize_title(&meta.title);
        let preferred_types = config.preferred_types(&meta.media_type);
        let preference = preferred_types
            .iter()
            .position(|t| meta.filetypes.contains(t));
        if preference.is_none() {
            debug!(
                "Could not find any wanted formats in torrent {}, formats: {:?}, wanted: {:?}",
                torrent.id, meta.filetypes, preferred_types
            );
            continue 'torrent;
        }
        if let Some((_, rw)) = &rw_opt {
            let old_selected = {
                rw.scan()
                    .secondary::<mlm_db::SelectedTorrent>(mlm_db::SelectedTorrentKey::title_search)?
                    .range::<RangeInclusive<&str>>(title_search.as_str()..=title_search.as_str())?
                    .collect::<Result<Vec<_>, native_db::db_type::Error>>()
            }?;
            for old in old_selected {
                if old.mam_id == torrent.id {
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
                        if let Err(err) = add_duplicate_torrent(
                            rw,
                            None,
                            torrent.dl.clone(),
                            title_search,
                            torrent.id,
                            meta,
                        ) {
                            error!("Error writing duplicate torrent: {err}");
                        }
                        rw_opt.unwrap().1.commit()?;
                        trace!(
                            "Skipping torrent {} as we have {} selected",
                            torrent.id, old.mam_id
                        );
                        continue 'torrent;
                    } else {
                        if let Err(err) = add_duplicate_torrent(
                            rw,
                            None,
                            torrent.dl.clone(),
                            title_search.clone(),
                            torrent.id,
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
        if let Some((_, rw)) = &rw_opt {
            let old_library = {
                rw.scan()
                    .secondary::<mlm_db::Torrent>(mlm_db::TorrentKey::title_search)?
                    .range::<RangeInclusive<&str>>(title_search.as_str()..=title_search.as_str())?
                    .collect::<Result<Vec<_>, native_db::db_type::Error>>()
            }?;
            for old in old_library {
                if old.mam_id == Some(torrent.id) {
                    if old.meta != meta {
                        update_torrent_meta(
                            config,
                            db,
                            rw_opt.unwrap(),
                            Some(&torrent),
                            old,
                            meta,
                            false,
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
                        trace!(
                            "Skipping torrent {} as we have {} in libary",
                            torrent.id, &old.id
                        );
                        if let Err(err) = add_duplicate_torrent(
                            rw,
                            Some(old.id),
                            torrent.dl.clone(),
                            title_search,
                            torrent.id,
                            meta,
                        ) {
                            error!("Error writing duplicate torrent: {err}");
                        }
                        rw_opt.unwrap().1.commit()?;
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
        if let Some((_, rw)) = &rw_opt {
            selected_torrents += 1;
            rw.insert(mlm_db::SelectedTorrent {
                mam_id: torrent.id,
                hash: None,
                dl_link: torrent
                    .dl
                    .clone()
                    .ok_or_else(|| Error::msg(format!("no dl field for torrent {}", torrent.id)))?,
                unsat_buffer,
                wedge_buffer,
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
            rw_opt.unwrap().1.commit()?;
            if selected_torrents >= max_torrents {
                break;
            }
        }
    }

    Ok(selected_torrents)
}

#[instrument(skip_all)]
pub async fn add_metadata_only_torrent(
    (_guard, rw): (MutexGuard<'_, ()>, RwTransaction<'_>),
    torrent: MaMTorrent,
    meta: TorrentMeta,
) -> Result<()> {
    info!("Adding metadata only torrent \"{}\"", meta.title);
    let id = Uuid::new_v4().to_string();

    let mam_id = torrent.id;
    {
        rw.insert(mlm_db::Torrent {
            id,
            id_is_hash: false,
            mam_id: Some(mam_id),
            library_path: None,
            library_files: Default::default(),
            linker: if torrent.owner_name.is_empty() {
                None
            } else {
                Some(torrent.owner_name)
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

#[allow(clippy::too_many_arguments)]
pub async fn update_torrent_meta(
    config: &Config,
    db: &Database<'_>,
    (guard, rw): (MutexGuard<'_, ()>, RwTransaction<'_>),
    mam_torrent: Option<&MaMTorrent>,
    mut torrent: mlm_db::Torrent,
    mut meta: TorrentMeta,
    allow_non_mam: bool,
    linker_is_owner: bool,
) -> Result<()> {
    meta.ids.extend(torrent.meta.ids.clone());
    meta.tags = torrent.meta.tags.clone();
    if meta.description.is_empty() {
        meta.description = torrent.meta.description.clone();
    }

    if !allow_non_mam && torrent.meta.source != MetadataSource::Mam {
        // Update VIP status and uploaded_at still
        if torrent.meta.vip_status != meta.vip_status
            || torrent.meta.uploaded_at != meta.uploaded_at
        {
            torrent.meta.vip_status = meta.vip_status;
            torrent.meta.uploaded_at = meta.uploaded_at;
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

    // Check uploaded_at and num_files
    if torrent.meta.uploaded_at != meta.uploaded_at || torrent.meta.num_files != meta.num_files {
        torrent.meta.uploaded_at = meta.uploaded_at;
        torrent.meta.num_files = meta.num_files;
        // If uploaded_at or num_files was the only change, just silently update the database
        if torrent.meta == meta {
            rw.upsert(torrent.clone())?;
            rw.commit()?;
            return Ok(());
        }
    }

    if linker_is_owner
        && torrent.linker.is_none()
        && let Some(mam_torrent) = mam_torrent
    {
        torrent.linker = Some(mam_torrent.owner_name.clone());
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

    if let Some(library_path) = &torrent.library_path
        && let serde_json::Value::Object(new) = abs::create_metadata(&meta)
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
            debug!("updated ABS metadata file {}", id);
        }
        if let (Some(abs_id), Some(abs_config)) =
            (&torrent.meta.ids.get(ids::ABS), &config.audiobookshelf)
        {
            let abs = Abs::new(abs_config)?;
            match abs.update_book(abs_id, &meta).await {
                Ok(_) => debug!("updated ABS via API {}", id),
                Err(err) => warn!("Failed updating book {} in abs: {err}", id),
            }
        }
    }

    if !diff.is_empty() {
        let mam_id = mam_torrent.map(|m| m.id);
        write_event(
            db,
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

async fn update_selected_torrent_meta(
    db: &Database<'_>,
    (guard, rw): (MutexGuard<'_, ()>, RwTransaction<'_>),
    mam: &MaM<'_>,
    torrent: SelectedTorrent,
    meta: TorrentMeta,
) -> Result<()> {
    let mam_id = torrent.mam_id;
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
    let source = meta.source.clone();
    torrent.meta = meta;
    rw.upsert(torrent)?;
    rw.commit()?;
    drop(guard);
    write_event(
        db,
        Event::new(
            hash,
            Some(mam_id),
            EventType::Updated {
                fields: diff,
                source: (source, String::new()),
            },
        ),
    )
    .await;
    Ok(())
}

pub async fn get_mam_torrent_hash(mam: &MaM<'_>, dl_link: &str) -> Result<String> {
    let torrent_file_bytes = get_mam_torrent_file(mam, dl_link).await?;
    let torrent_file = Torrent::read_from_bytes(torrent_file_bytes.clone())?;
    let hash = torrent_file.info_hash();
    Ok(hash)
}

fn add_duplicate_torrent(
    rw: &RwTransaction<'_>,
    duplicate_of: Option<String>,
    dl_link: Option<String>,
    title_search: String,
    mam_id: u64,
    meta: TorrentMeta,
) -> Result<()> {
    rw.upsert(DuplicateTorrent {
        mam_id,
        dl_link,
        title_search,
        meta,
        created_at: Timestamp::now(),
        duplicate_of,
    })?;
    Ok(())
}
