use std::{ops::RangeInclusive, sync::Arc, time::Duration};

use crate::{
    config::{Config, Cost, TorrentFilter, Type},
    data::{self, ErroredTorrent, ErroredTorrentId, SelectedTorrent, TorrentMeta},
    mam::{MaM, MetaError, SearchKind, SearchQuery, SearchTarget, Tor, normalize_title},
    qbittorrent::QbitError,
};
use anyhow::{Error, Result};
use lava_torrent::torrent::v1::Torrent;
use native_db::{Database, db_type, transaction::RwTransaction};
use qbit::parameters::TorrentAddUrls;
use tokio::time::sleep;

pub async fn run_autograbbers(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    qbit: &qbit::Api,
    mam: Arc<MaM<'_>>,
) -> Result<()> {
    let user_info = mam.user_info().await?;
    let max_torrents = user_info
        .classname
        .unsats()
        .saturating_sub(user_info.unsat.count);
    println!("user_info: {user_info:#?}; max_torrents: {max_torrents}");

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
            .await?;
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

        let mam_id = torrent.mam_id;
        let title = torrent.meta.title.clone();
        let result = grab_torrent(&config, &db, qbit, &mam, torrent).await;
        if let Err(err) = update_errored_torrent(&db, mam_id, title, result) {
            eprintln!("Error writing errored torrent: {err}");
        }

        sleep(Duration::from_millis(1000)).await;
        snatched_torrents += 1;
    }

    Ok(())
}

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
    let results = mam
        .search(&SearchQuery {
            dl_link: true,
            perpage: 100.min(max_torrents),
            tor: Tor {
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
                browse_flags: flags,
                min_size: torrent_filter.filter.min_size.bytes(),
                max_size: torrent_filter.filter.max_size.bytes(),
                unit: torrent_filter
                    .filter
                    .min_size
                    .unit()
                    .max(torrent_filter.filter.max_size.unit()),
                sort_type,
                ..Default::default()
            },

            ..Default::default()
        })
        .await?;

    let torrents = results
        .data
        .into_iter()
        .filter(|t| torrent_filter.filter.matches(t));

    'torrent: for torrent in torrents {
        let rw = db.rw_transaction()?;
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
                    rw.commit()?;
                    continue;
                }
            }
            continue;
        }
        let title_search = normalize_title(&torrent.title);
        let meta = match torrent.as_meta() {
            Ok(it) => it,
            Err(err) => match err {
                MetaError::UnknownMainCat(_) => {
                    println!("{err} for torrent {} {}", torrent.id, torrent.title);
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
        {
            let old_selected = {
                rw.scan()
                    .secondary::<data::SelectedTorrent>(data::SelectedTorrentKey::title_search)?
                    .range::<RangeInclusive<&str>>(title_search.as_str()..=title_search.as_str())?
                    .collect::<Result<Vec<_>, native_db::db_type::Error>>()
            }?;
            for old in old_selected {
                println!(
                    "Checking old torrent {} with formats {:?}",
                    old.title_search, old.meta.filetypes
                );
                if meta.matches(&old.meta) {
                    let old_preference = preferred_types
                        .iter()
                        .position(|t| old.meta.filetypes.contains(t));
                    if old_preference <= preference {
                        if old_preference == preference {
                            if let Err(err) = add_duplicate_torrent(&rw, None, title_search, meta) {
                                eprintln!("Error writing duplicate torrent: {err}");
                            }
                        }
                        continue 'torrent;
                    } else {
                        println!(
                            "Unselecting torrent \"{}\" with formats {:?}",
                            old.meta.title, old.meta.filetypes
                        );
                        rw.remove(old)?;
                    }
                }
            }
        }
        {
            let old_library = {
                rw.scan()
                    .secondary::<data::Torrent>(data::TorrentKey::title_search)?
                    .range::<RangeInclusive<&str>>(title_search.as_str()..=title_search.as_str())?
                    .collect::<Result<Vec<_>, native_db::db_type::Error>>()
            }?;
            for old in old_library {
                println!(
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
                                add_duplicate_torrent(&rw, Some(old.hash), title_search, meta)
                            {
                                eprintln!("Error writing duplicate torrent: {err}");
                            }
                        }
                        continue 'torrent;
                    } else {
                        println!(
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
        println!(
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
        })?;
        rw.commit()?;
    }

    Ok(())
}

async fn grab_torrent(
    config: &Config,
    db: &Database<'_>,
    qbit: &qbit::Api,
    mam: &MaM<'_>,
    torrent: SelectedTorrent,
) -> Result<()> {
    println!(
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

    let rw = db.rw_transaction()?;
    rw.insert(data::Torrent {
        hash,
        library_path: None,
        library_files: Default::default(),
        title_search: torrent.title_search.clone(),
        meta: torrent.meta.clone(),
        replaced_with: None,
        request_matadata_update: false,
    })
    .or_else(|err| {
        if let db_type::Error::DuplicateKey { .. } = err {
            println!("Got dup key on {:?}", torrent);
            Ok(())
        } else {
            Err(err)
        }
    })?;
    rw.remove(torrent)?;
    rw.commit()?;

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
        duplicate_of,
        request_replace: false,
    })?;
    Ok(())
}

fn update_errored_torrent(
    db: &Database<'_>,
    mam_id: u64,
    torrent: String,
    result: Result<(), Error>,
) -> Result<()> {
    let rw = db.rw_transaction()?;
    let id = ErroredTorrentId::Grabber(mam_id);
    if let Err(err) = result {
        println!("add_errored_torrent {torrent} - {err} - Grabber");
        rw.upsert(ErroredTorrent {
            id,
            title: torrent,
            error: format!("{err}"),
            meta: None,
        })?;
    } else if let Some(error) = rw.get().primary::<ErroredTorrent>(id)? {
        rw.remove(error)?;
    }
    rw.commit()?;
    Ok(())
}
