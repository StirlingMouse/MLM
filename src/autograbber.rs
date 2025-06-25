use std::{ops::RangeInclusive, time::Duration};

use crate::{
    config::{Config, Cost, TorrentFilter, Type},
    data,
    mam::{MaM, SearchKind, SearchQuery, SearchTarget, Tor, normalize_title},
    qbittorrent::QbitError,
};
use anyhow::{Error, Result};
use lava_torrent::torrent::v1::Torrent;
use native_db::Database;
use qbit::parameters::TorrentAddUrls;
use tokio::time::sleep;

pub async fn run_autograbbers(
    config: &Config,
    db: &Database<'_>,
    qbit: &qbit::Api,
    mam: &MaM<'_>,
) -> Result<()> {
    let user_info = mam.user_info().await?;
    let max_torrents = user_info
        .classname
        .unsats()
        .saturating_sub(user_info.unsat.count as u8);
    println!("user_info: {user_info:#?}; max_torrents: {max_torrents}");

    for autograb_config in &config.autograbs {
        let max_torrents = max_torrents
            .saturating_sub(autograb_config.unsat_buffer.unwrap_or(config.unsat_buffer));
        if max_torrents > 0 {
            autograb(config, db, autograb_config, mam, max_torrents).await?;
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

        println!(
            "Grabbing torrent \"{}\" in format {}, with category {:?} and tags {:?}",
            torrent.meta.title, torrent.meta.filetype, torrent.category, torrent.tags,
        );
        let torrent_file_bytes = mam.get_torrent_file(&torrent.dl_link).await?;
        let torrent_file = Torrent::read_from_bytes(torrent_file_bytes.clone())?;
        let hash = torrent_file.info_hash();
        qbit.add_torrent(TorrentAddUrls {
            torrents: vec![torrent_file_bytes.iter().copied().collect()],
            stopped: true,
            category: torrent.category.clone(),
            tags: if torrent.tags.is_empty() {
                None
            } else {
                Some(torrent.tags.clone())
            },
            ..TorrentAddUrls::deafult(vec![])
        })
        .await
        .map_err(QbitError)?;
        {
            let rw = db.rw_transaction()?;
            rw.insert(data::Torrent {
                hash,
                library_path: None,
                title_search: torrent.title_search.clone(),
                meta: torrent.meta.clone(),
            })?;
            rw.remove(torrent)?;
            rw.commit()?;
        }
        println!("Added and should be deleted",);
        sleep(Duration::from_millis(1000)).await;
        snatched_torrents += 1;
        // return Ok(());
    }

    Ok(())
}

pub async fn autograb(
    config: &Config,
    db: &Database<'_>,
    torrent_filter: &TorrentFilter,
    mam: &MaM<'_>,
    max_torrents: u8,
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
                max_size: torrent_filter.filter.max_size.bytes(),
                unit: torrent_filter.filter.max_size.unit(),
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
        let meta = torrent.as_meta()?;
        let preferred_types = match meta.main_cat {
            data::MainCat::Audio => &config.audio_types,
            data::MainCat::Ebook => &config.ebook_types,
        };
        let preference = preferred_types.iter().position(|t| t == &meta.filetype);
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
                    "Checking old torrent {} with format {}",
                    old.title_search, old.meta.filetype
                );
                if old.meta.main_cat == meta.main_cat {
                    let old_preference =
                        preferred_types.iter().position(|t| t == &old.meta.filetype);
                    if old_preference <= preference {
                        continue 'torrent;
                    } else {
                        println!(
                            "Unselecting torrent \"{}\" in format {}",
                            old.meta.title, old.meta.filetype
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
                    "Checking old torrent {} with format {}",
                    old.title_search, old.meta.filetype
                );
                if old.meta.main_cat == meta.main_cat {
                    let old_preference =
                        preferred_types.iter().position(|t| t == &old.meta.filetype);
                    if old_preference <= preference {
                        continue 'torrent;
                    } else {
                        println!(
                            "Replacing library torrent \"{}\" in format {}",
                            old.meta.title, old.meta.filetype
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
