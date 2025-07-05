use std::{ops::RangeInclusive, sync::Arc, time::Duration};

use crate::{
    config::{Config, Cost, TorrentFilter, Type},
    data::{self, ErroredTorrentId, Event, EventType, SelectedTorrent, Timestamp, TorrentMeta},
    logging::{TorrentMetaError, update_errored_torrent, write_event},
    mam::{
        MaM, MetaError, SearchKind, SearchQuery, SearchResult, SearchTarget, Tor, normalize_title,
    },
    qbittorrent::QbitError,
};
use anyhow::{Context, Error, Result};
use lava_torrent::torrent::v1::Torrent;
use native_db::{Database, db_type, transaction::RwTransaction};
use once_cell::sync::Lazy;
use qbit::parameters::TorrentAddUrls;
use time::format_description::{self, OwnedFormatItem};
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, trace, warn};

pub static DATE_FORMAT: Lazy<OwnedFormatItem> =
    Lazy::new(|| format_description::parse_owned::<2>("[year]-[month]-[day]").unwrap());

#[instrument(skip_all)]
pub async fn run_autograbbers(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    qbit: &qbit::Api,
    mam: Arc<MaM<'_>>,
) -> Result<()> {
    let user_info = mam.user_info().await?;
    let max_torrents = user_info.unsat.limit.saturating_sub(user_info.unsat.count);
    debug!("user_info: {user_info:#?}; max_torrents: {max_torrents}");

    for autograb_config in &config.autograbs {
        let max_torrents = max_torrents
            .saturating_sub(autograb_config.unsat_buffer.unwrap_or(config.unsat_buffer));
        if max_torrents > 0 {
            select_torrents(
                config.clone(),
                db.clone(),
                autograb_config,
                mam.clone(),
                max_torrents,
            )
            .await
            .context("select_torrents")?;
        }
    }

    let selected_torrents = {
        let r = db.r_transaction()?;
        r.scan()
            .primary::<data::SelectedTorrent>()?
            .all()?
            .collect::<Result<Vec<_>, native_db::db_type::Error>>()
    }?;
    let mut snatched_torrents = 0;
    for torrent in selected_torrents {
        let max_torrents = max_torrents
            .saturating_sub(torrent.unsat_buffer.unwrap_or(config.unsat_buffer))
            .saturating_sub(snatched_torrents);
        if max_torrents == 0 {
            continue;
        }

        let result = grab_torrent(&config, &db, qbit, &mam, torrent.clone())
            .await
            .map_err(|err| anyhow::Error::new(TorrentMetaError(torrent.meta.clone(), err)));
        update_errored_torrent(
            &db,
            ErroredTorrentId::Grabber(torrent.mam_id),
            torrent.meta.title,
            result,
        );

        sleep(Duration::from_millis(1000)).await;
        snatched_torrents += 1;
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn select_torrents(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    torrent_filter: &TorrentFilter,
    mam: Arc<MaM<'_>>,
    max_torrents: u64,
) -> Result<()> {
    let target = match torrent_filter.kind {
        Type::Bookmarks => Some(SearchTarget::Bookmarks),
        _ => None,
    };
    let kind = match (torrent_filter.kind, torrent_filter.cost) {
        (Type::Freeleech, _) => Some(SearchKind::Freeleech),
        (_, Cost::Free) => Some(SearchKind::Free),
        _ => None,
    };
    let sort_type = match torrent_filter.kind {
        Type::New => "dateDesc",
        _ => "",
    };
    let (flags_is_hide, flags) = torrent_filter.filter.flags.as_search_bitfield();
    let paginate = matches!(torrent_filter.kind, Type::Bookmarks | Type::Freeleech);

    let mut results: Option<SearchResult> = None;
    loop {
        let mut page_results = mam
            .search(&SearchQuery {
                dl_link: true,
                perpage: 100.min(max_torrents),
                tor: Tor {
                    start_number: results.as_ref().map_or(0, |r| r.data.len() as u64),
                    target,
                    kind,
                    text: &torrent_filter.query.clone().unwrap_or_default(),
                    srch_in: torrent_filter.search_in.clone(),
                    main_cat: torrent_filter.filter.categories.get_main_cats(),
                    cat: torrent_filter.filter.categories.get_cats(),
                    browse_lang: torrent_filter
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
                    start_date: torrent_filter
                        .uploaded_after
                        .map_or_else(|| Ok(String::new()), |d| d.format(&DATE_FORMAT))?,
                    end_date: torrent_filter
                        .uploaded_before
                        .map_or_else(|| Ok(String::new()), |d| d.format(&DATE_FORMAT))?,
                    min_size: torrent_filter.filter.min_size.bytes(),
                    max_size: torrent_filter.filter.max_size.bytes(),
                    unit: torrent_filter
                        .filter
                        .min_size
                        .unit()
                        .max(torrent_filter.filter.max_size.unit()),
                    min_seeders: torrent_filter.min_seeders,
                    max_seeders: torrent_filter.max_seeders,
                    min_leechers: torrent_filter.min_leechers,
                    max_leechers: torrent_filter.max_leechers,
                    min_snatched: torrent_filter.min_snatched,
                    max_snatched: torrent_filter.max_snatched,
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

        if let Some(results) = &mut results {
            results.data.append(&mut page_results.data);
        } else {
            results = Some(page_results);
        }

        let results = results.as_ref().unwrap();
        if !paginate || results.data.len() >= results.found {
            break;
        }
    }

    let torrents = results
        .unwrap()
        .data
        .into_iter()
        .filter(|t| torrent_filter.filter.matches(t));

    'torrent: for torrent in torrents {
        let rw_opt = if torrent_filter.dry_run {
            None
        } else {
            Some(db.rw_transaction()?)
        };
        if let Some(rw) = &rw_opt {
            if let Some(old_selected) = rw
                .get()
                .primary::<data::SelectedTorrent>(torrent.id)
                .ok()
                .flatten()
            {
                if let Some(unsat_buffer) = torrent_filter.unsat_buffer {
                    if old_selected.unsat_buffer.is_none_or(|u| unsat_buffer < u) {
                        let mut updated = old_selected.clone();
                        updated.unsat_buffer = Some(unsat_buffer);
                        rw.update(old_selected, updated)?;
                        rw_opt.unwrap().commit()?;
                        continue;
                    }
                }
                continue;
            }
        }
        let title_search = normalize_title(&torrent.title);
        let meta = match torrent.as_meta() {
            Ok(it) => it,
            Err(err) => match err {
                MetaError::UnknownMainCat(_) => {
                    warn!("{err} for torrent {} {}", torrent.id, torrent.title);
                    continue;
                }
                _ => return Err(err.into()),
            },
        };
        let preferred_types = match meta.main_cat {
            data::MainCat::Audio => &config.audio_types,
            data::MainCat::Ebook => &config.ebook_types,
        };
        let preference = preferred_types
            .iter()
            .position(|t| meta.filetypes.contains(t));
        if preference.is_none() {
            continue;
        }
        if let Some(rw) = &rw_opt {
            let old_selected = {
                rw.scan()
                    .secondary::<data::SelectedTorrent>(data::SelectedTorrentKey::title_search)?
                    .range::<RangeInclusive<&str>>(title_search.as_str()..=title_search.as_str())?
                    .collect::<Result<Vec<_>, native_db::db_type::Error>>()
            }?;
            for old in old_selected {
                trace!(
                    "Checking old torrent {} with formats {:?}",
                    old.title_search, old.meta.filetypes
                );
                if meta.matches(&old.meta) {
                    let old_preference = preferred_types
                        .iter()
                        .position(|t| old.meta.filetypes.contains(t));
                    if old_preference <= preference {
                        if old_preference == preference {
                            if let Err(err) = add_duplicate_torrent(rw, None, title_search, meta) {
                                error!("Error writing duplicate torrent: {err}");
                            }
                        }
                        continue 'torrent;
                    } else {
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
                trace!(
                    "Checking old torrent {} with formats {:?}",
                    old.title_search, old.meta.filetypes
                );
                if meta.matches(&old.meta) {
                    let old_preference = preferred_types
                        .iter()
                        .position(|t| old.meta.filetypes.contains(t));
                    if old_preference <= preference {
                        if old_preference == preference {
                            if let Err(err) =
                                add_duplicate_torrent(rw, Some(old.hash), title_search, meta)
                            {
                                error!("Error writing duplicate torrent: {err}");
                            }
                        }
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
        let category = tags.iter().find_map(|t| t.category.clone());
        let tags = tags.iter().flat_map(|t| t.tags.clone()).collect();
        info!(
            "Selecting torrent \"{}\" in format {}, free: {}, fl_vip: {}, pf: {}, vip: {}, with category {:?} and tags {:?}",
            torrent.title,
            torrent.filetype,
            torrent.free,
            torrent.fl_vip,
            torrent.personal_freeleech,
            torrent.vip,
            category,
            tags
        );
        if let Some(rw) = &rw_opt {
            rw.insert(data::SelectedTorrent {
                mam_id: torrent.id,
                dl_link: torrent
                    .dl
                    .clone()
                    .ok_or_else(|| Error::msg(format!("no dl field for torrent {}", torrent.id)))?,
                unsat_buffer: torrent_filter.unsat_buffer,
                category,
                tags,
                title_search,
                meta,
                created_at: Timestamp::now(),
            })?;
            rw_opt.unwrap().commit()?;
        }
    }

    Ok(())
}

#[instrument(skip_all)]
async fn grab_torrent(
    config: &Config,
    db: &Database<'_>,
    qbit: &qbit::Api,
    mam: &MaM<'_>,
    torrent: SelectedTorrent,
) -> Result<()> {
    info!(
        "Grabbing torrent \"{}\", with category {:?} and tags {:?}",
        torrent.meta.title, torrent.category, torrent.tags,
    );
    let torrent_file_bytes = mam.get_torrent_file(&torrent.dl_link).await?;
    let torrent_file = Torrent::read_from_bytes(torrent_file_bytes.clone())?;
    let hash = torrent_file.info_hash();
    qbit.add_torrent(TorrentAddUrls {
        torrents: vec![torrent_file_bytes.iter().copied().collect()],
        stopped: config.add_torrents_stopped,
        category: torrent.category.clone(),
        tags: if torrent.tags.is_empty() {
            None
        } else {
            Some(torrent.tags.clone())
        },
        ..TorrentAddUrls::default(vec![])
    })
    .await
    .map_err(QbitError)?;

    let mam_id = torrent.mam_id;
    {
        let rw = db.rw_transaction()?;
        rw.insert(data::Torrent {
            hash: hash.clone(),
            library_path: None,
            library_files: Default::default(),
            selected_audio_format: None,
            selected_ebook_format: None,
            title_search: torrent.title_search.clone(),
            meta: torrent.meta.clone(),
            created_at: Timestamp::now(),
            replaced_with: None,
            request_matadata_update: false,
        })
        .or_else(|err| {
            if let db_type::Error::DuplicateKey { .. } = err {
                warn!("Got dup key on {:?}", torrent);
                Ok(())
            } else {
                Err(err)
            }
        })?;
        rw.remove(torrent)?;
        rw.commit()?;
    }

    write_event(db, Event::new(Some(hash), Some(mam_id), EventType::Grabbed));

    Ok(())
}

fn add_duplicate_torrent(
    rw: &RwTransaction<'_>,
    duplicate_of: Option<String>,
    title_search: String,
    meta: TorrentMeta,
) -> Result<()> {
    rw.upsert(data::DuplicateTorrent {
        mam_id: meta.mam_id,
        title_search,
        meta,
        created_at: Timestamp::now(),
        duplicate_of,
        request_replace: false,
    })?;
    Ok(())
}
