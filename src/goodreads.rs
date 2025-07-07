use std::sync::Arc;

use anyhow::{Context, Result};
use matchr::score;
use native_db::Database;
use once_cell::sync::Lazy;
use quick_xml::de::from_reader;
use regex::Regex;
use reqwest::Url;
use serde::Deserialize;
use tracing::{debug, instrument};
use tracing::{trace, warn};

use crate::autograbber::grab_selected_torrents;
use crate::{
    autograbber::select_torrents,
    config::{Config, Cost, Grab},
    data::{List, ListItem, MainCat, Timestamp, TorrentMeta},
    mam::{DATE_FORMAT, MaM, MaMTorrent, SearchQuery, SearchResult, Tor},
    mam_enums::SearchIn,
};

pub static SERIES_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(.*?) \((.*?), #(\d+)\)$").unwrap());

#[instrument(skip_all)]
pub async fn run_goodreads_import(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    qbit: &qbit::Api,
    mam: Arc<MaM<'_>>,
) -> Result<()> {
    let user_info = mam.user_info().await?;
    let max_torrents = user_info.unsat.limit.saturating_sub(user_info.unsat.count);

    for list in &config.goodreads_lists {
        let max_torrents =
            max_torrents.saturating_sub(list.unsat_buffer.unwrap_or(config.unsat_buffer));

        if max_torrents > 0 {
            let content = reqwest::get(&list.url).await?.bytes().await?;

            let mut rss: Rss = from_reader(&content[..])?;
            trace!("Scanning Goodreads list {}", rss.channel.title);

            let link: Url = list.url.parse()?;
            let list_id: u64 = link
                .path_segments()
                .iter_mut()
                .flatten()
                .next_back()
                .ok_or(anyhow::Error::msg("Failed to get goodreads list id"))?
                .parse()?;

            if !list.dry_run {
                let rw = db.rw_transaction()?;
                rw.upsert(List {
                    id: list_id,
                    title: rss.channel.title,
                })?;
                rw.commit()?;
            }

            for item in rss.channel.items.iter_mut() {
                if let Some((_, [title, series_name, series_num])) =
                    SERIES_PATTERN.captures(&item.title).map(|c| c.extract())
                {
                    item.series = Some((series_name.to_owned(), series_num.parse()?));
                    item.title = title.to_owned();
                }
            }

            for item in rss.channel.items.iter() {
                let db_item = match db
                    .r_transaction()?
                    .get()
                    .primary::<ListItem>((list_id, item.guid.clone()))?
                {
                    Some(mut db_item) => {
                        if db_item.prefer_format != list.prefer_format {
                            db_item.prefer_format = list.prefer_format;
                            if !list.dry_run {
                                let rw = db.rw_transaction()?;
                                rw.upsert(db_item.clone())?;
                                rw.commit()?;
                            }
                        }
                        if (db_item.audio_torrent.is_some() && db_item.ebook_torrent.is_some())
                            || (list.prefer_format == Some(MainCat::Audio)
                                && db_item.audio_torrent.is_some())
                            || (list.prefer_format == Some(MainCat::Ebook)
                                && db_item.ebook_torrent.is_some())
                        {
                            continue;
                        }
                        db_item
                    }
                    None => {
                        let db_item = ListItem {
                            guid: (list_id, item.guid.clone()),
                            list_id,
                            title: item.title.clone(),
                            authors: vec![item.author_name.clone()],
                            series: item.series.iter().cloned().collect(),
                            cover_url: item.book_large_image_url.clone(),
                            book_url: None,
                            isbn: item.isbn.as_ref().and_then(|isbn| isbn.parse().ok()),
                            prefer_format: list.prefer_format,
                            audio_torrent: None,
                            wanted_audio_torrent: None,
                            ebook_torrent: None,
                            wanted_ebook_torrent: None,
                            created_at: Timestamp::now(),
                        };
                        if !list.dry_run {
                            let rw = db.rw_transaction()?;
                            rw.insert(db_item.clone())?;
                            rw.commit()?;
                        }
                        db_item
                    }
                };
                trace!("Searching for book {} from Goodreads list", item.title);
                search_item(&config, &db, &mam, &list, item, db_item)
                    .await
                    .context("search goodreads book")?;
            }
        }
    }

    grab_selected_torrents(&config, &db, qbit, &mam, max_torrents)
        .await
        .context("grab_selected_torrents")?;

    Ok(())
}

#[instrument(skip_all)]
async fn search_item(
    config: &Config,
    db: &Database<'_>,
    mam: &MaM<'_>,
    list: &&crate::config::GoodreadsList,
    item: &Item,
    mut db_item: ListItem,
) -> Result<()> {
    let mut torrents = vec![];
    for grab in &list.grab {
        let results = search_grab(config, mam, item, &db_item, grab)
            .await
            .context("search_grab")?;
        torrents.push(results);
    }
    let mut audiobook = torrents
        .iter()
        .flatten()
        .filter(|t| t.1.main_cat == MainCat::Audio)
        .find(|t| t.3 != Cost::Free || t.0.is_free())
        .or_else(|| {
            torrents
                .iter()
                .flatten()
                .find(|t| t.1.main_cat == MainCat::Audio)
        });
    let mut ebook = torrents
        .iter()
        .flatten()
        .filter(|t| t.1.main_cat == MainCat::Ebook)
        .find(|t| t.3 != Cost::Free || t.0.is_free())
        .or_else(|| {
            torrents
                .iter()
                .flatten()
                .find(|t| t.1.main_cat == MainCat::Ebook)
        });

    for selected in [audiobook, ebook].iter_mut() {
        let mut take = false;
        if let Some(selected) = selected {
            if selected.3 == Cost::Free && !selected.0.is_free() {
                take = true
            }
        }
        if take {
            let taken = selected.unwrap();
            warn!(
                "Skipping torrent {}, in format {} as it's not free",
                taken.1.title,
                taken.1.filetypes.join(", ")
            );
            match taken.1.main_cat {
                MainCat::Audio => {
                    db_item.wanted_audio_torrent = Some((taken.0.id, Timestamp::now()));
                    audiobook.take();
                }
                MainCat::Ebook => {
                    db_item.wanted_ebook_torrent = Some((taken.0.id, Timestamp::now()));
                    ebook.take();
                }
            }
            if !list.dry_run {
                let rw = db.rw_transaction()?;
                rw.upsert(db_item.clone())?;
                rw.commit()?;
            }
        }
    }

    if audiobook.is_some() && ebook.is_some() {
        match list.prefer_format {
            Some(MainCat::Audio) => {
                ebook.take();
            }
            Some(MainCat::Ebook) => {
                audiobook.take();
            }
            None => {}
        }
    }

    {
        db_item.audio_torrent = audiobook.map(|t| (t.0.id, Timestamp::now()));
        db_item.ebook_torrent = ebook.map(|t| (t.0.id, Timestamp::now()));
        if audiobook.is_some() {
            db_item.wanted_audio_torrent.take();
        }
        if ebook.is_some() {
            db_item.wanted_ebook_torrent.take();
        }
        if !list.dry_run {
            let rw = db.rw_transaction()?;
            rw.upsert(db_item.clone())?;
            rw.commit()?;
        }
    }

    if let Some(audiobook) = audiobook {
        select_torrents(
            config,
            db,
            [audiobook.0.clone()].into_iter(),
            audiobook.3,
            list.unsat_buffer,
            list.dry_run,
        )
        .await
        .context("select_torrents")?;
    }
    if let Some(ebook) = ebook {
        select_torrents(
            config,
            db,
            [ebook.0.clone()].into_iter(),
            ebook.3,
            list.unsat_buffer,
            list.dry_run,
        )
        .await
        .context("select_torrents")?;
    }

    Ok(())
}

#[instrument(skip_all)]
async fn search_grab(
    config: &Config,
    mam: &MaM<'_>,
    item: &Item,
    db_item: &ListItem,
    grab: &Grab,
) -> Result<Vec<(MaMTorrent, TorrentMeta, usize, Cost)>> {
    let (flags_is_hide, flags) = grab.filter.flags.as_search_bitfield();
    let query = format!("{} {}", item.title, item.author_name);

    let mut categories = grab.filter.categories.clone();
    if db_item.audio_torrent.is_some() {
        categories.audio = Some(vec![])
    }
    if db_item.ebook_torrent.is_some() {
        categories.ebook = Some(vec![])
    }

    let mut results: Option<SearchResult> = None;
    loop {
        let mut page_results = mam
            .search(&SearchQuery {
                dl_link: true,
                perpage: 100,
                tor: Tor {
                    start_number: results.as_ref().map_or(0, |r| r.data.len() as u64),
                    text: &query,
                    srch_in: vec![SearchIn::Title, SearchIn::Author],
                    main_cat: categories.get_main_cats(),
                    cat: categories.get_cats(),
                    browse_lang: grab.filter.languages.iter().map(|l| l.to_id()).collect(),
                    browse_flags_hide_vs_show: if flags.is_empty() {
                        None
                    } else {
                        Some(if flags_is_hide { 0 } else { 1 })
                    },
                    browse_flags: flags.clone(),
                    start_date: grab
                        .filter
                        .uploaded_after
                        .map_or_else(|| Ok(String::new()), |d| d.format(&DATE_FORMAT))?,
                    end_date: grab
                        .filter
                        .uploaded_before
                        .map_or_else(|| Ok(String::new()), |d| d.format(&DATE_FORMAT))?,
                    min_size: grab.filter.min_size.bytes(),
                    max_size: grab.filter.max_size.bytes(),
                    unit: grab.filter.min_size.unit().max(grab.filter.max_size.unit()),
                    min_seeders: grab.filter.min_seeders,
                    max_seeders: grab.filter.max_seeders,
                    min_leechers: grab.filter.min_leechers,
                    max_leechers: grab.filter.max_leechers,
                    min_snatched: grab.filter.min_snatched,
                    max_snatched: grab.filter.max_snatched,
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
        if results.data.len() >= results.found {
            break;
        }
    }

    let mut torrents = results
        .unwrap()
        .data
        .into_iter()
        .filter(|t| grab.filter.matches(t))
        .map(|t| {
            let title_score = score(&item.title, &t.title);
            let author_score = t
                .author_info
                .values()
                .map(|author| score(&item.author_name, author))
                .max()
                .unwrap_or_default();
            let series_score = item
                .series
                .as_ref()
                .and_then(|(i_name, i_num)| {
                    t.series_info
                        .values()
                        .map(|(t_name, t_num)| {
                            score(i_name, t_name) + score(&i_num.to_string(), t_num)
                        })
                        .max()
                })
                .unwrap_or_default();
            (t, title_score * 2 + author_score * 2 + series_score)
        })
        .collect::<Vec<_>>();
    torrents.sort_by_key(|t| -(t.1 as i64));

    if torrents.is_empty() {
        return Ok(Default::default());
    }
    let max_score = torrents[0].1;
    let mut torrents = torrents
        .into_iter()
        .take_while(|t| t.1 > max_score - 100)
        .map(|(t, _)| {
            let meta = t.as_meta()?;
            let preferred_types = match meta.main_cat {
                MainCat::Audio => &config.audio_types,
                MainCat::Ebook => &config.ebook_types,
            };
            let preference = preferred_types
                .iter()
                .position(|t| meta.filetypes.contains(t));
            Ok((t, meta, preference.unwrap_or_default(), grab.cost))
        })
        .collect::<Result<Vec<_>>>()?;
    torrents.sort_by(|a, b| {
        a.2.cmp(&b.2).then(
            a.0.numfiles
                .cmp(&b.0.numfiles)
                .then(a.1.size.bytes().cmp(&b.1.size.bytes()).reverse()),
        )
    });

    Ok(torrents)
}

#[derive(Debug, Deserialize)]
struct Rss {
    channel: Channel,
}

#[derive(Debug, Deserialize)]
struct Channel {
    title: String,
    #[serde(rename = "item")]
    items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    guid: String,
    title: String,
    author_name: String,
    series: Option<(String, u64)>,
    book_large_image_url: String,
    // book_description: String,
    isbn: Option<String>,
    // description: String,
}
