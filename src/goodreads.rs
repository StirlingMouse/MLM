use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use itertools::Itertools;
use matchr::score;
use native_db::Database;
use once_cell::sync::Lazy;
use quick_xml::de::from_reader;
use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::watch::Sender;
use tokio::time::sleep;
use tracing::{debug, instrument};
use tracing::{trace, warn};

use crate::config::GoodreadsList;
use crate::data::{ListItemTorrent, OldDbMainCat, Torrent, TorrentKey, TorrentStatus};
use crate::mam::enums::OldMainCat;
use crate::{
    autograbber::select_torrents,
    config::{Config, Cost, Grab},
    data::{List, ListItem, Timestamp, TorrentMeta},
    mam::{
        api::MaM,
        enums::SearchIn,
        meta::{clean_value, normalize_title},
        search::{MaMTorrent, SearchQuery, SearchResult, Tor},
        serde::DATE_FORMAT,
    },
};

pub static SERIES_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(.*?) \(([^)]*?),? #?(\d+(?:\.\d+)?)\)$").unwrap());

#[instrument(skip_all)]
pub async fn run_goodreads_import(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    mam: Arc<MaM<'_>>,
    autograb_trigger: Sender<()>,
) -> Result<()> {
    let user_info = mam.user_info().await?;
    let max_torrents = user_info.unsat.limit.saturating_sub(user_info.unsat.count);
    debug!(
        "goodreads import, unsats: {:#?}; max_torrents: {max_torrents}",
        user_info.unsat
    );

    let mut selected_torrents = 0;
    for list in &config.goodreads_lists {
        let max_torrents = max_torrents
            .saturating_sub(list.unsat_buffer.unwrap_or(config.unsat_buffer))
            .saturating_sub(selected_torrents);

        if max_torrents > 0 {
            let content = reqwest::get(&list.url).await?.bytes().await?;

            let mut rss: Rss = from_reader(&content[..])?;
            trace!("Scanning Goodreads list {}", rss.channel.title);

            let list_id = list.list_id()?;

            if !list.dry_run {
                let rw = db.rw_transaction()?;
                rw.upsert(List {
                    id: list_id.clone(),
                    title: rss.channel.title,
                    updated_at: Some(Timestamp::now()),
                    // TODO: Parse
                    build_date: Some(Timestamp::now()),
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

            for mut item in rss.channel.items.into_iter() {
                if let Ok(title) = clean_value(&item.title) {
                    item.title = title;
                }
                if let Some(author_name) = &item.author_name
                    && let Ok(author_name) = clean_value(author_name)
                {
                    item.author_name = Some(author_name);
                }
                if let Some((series_name, num)) = &item.series
                    && let Ok(series_name) = clean_value(series_name)
                {
                    item.series = Some((series_name, *num));
                }
                let db_item = match db
                    .r_transaction()?
                    .get()
                    .primary::<ListItem>((list_id.clone(), item.guid.clone()))?
                {
                    Some(mut db_item) => {
                        if db_item.prefer_format != list.prefer_format
                            || db_item.allow_audio != list.allow_audio()
                            || db_item.allow_ebook != list.allow_ebook()
                            || db_item.title != item.title
                            || db_item.series.first() != item.series.as_ref()
                        {
                            db_item.prefer_format = list.prefer_format;
                            db_item.allow_audio = list.allow_audio();
                            db_item.allow_ebook = list.allow_ebook();
                            db_item.title = item.title.clone();
                            db_item.series = item.series.iter().cloned().collect();
                            if !list.dry_run {
                                let rw = db.rw_transaction()?;
                                rw.upsert(db_item.clone())?;
                                rw.commit()?;
                            }
                        }
                        if (db_item.audio_torrent.is_some() && db_item.ebook_torrent.is_some())
                            || (list.prefer_format == Some(OldDbMainCat::Audio)
                                && db_item.audio_torrent.is_some())
                            || (list.prefer_format == Some(OldDbMainCat::Ebook)
                                && db_item.ebook_torrent.is_some())
                        {
                            continue;
                        }
                        db_item
                    }
                    None => {
                        let db_item = item.as_list_item(&list_id, list);
                        if !list.dry_run {
                            let rw = db.rw_transaction()?;
                            rw.insert(db_item.clone())?;
                            rw.commit()?;
                        }
                        db_item
                    }
                };
                trace!("Searching for book {} from Goodreads list", item.title);
                selected_torrents +=
                    search_item(&config, &db, &mam, list, &item, db_item, max_torrents)
                        .await
                        .context("search goodreads book")?;
                sleep(Duration::from_millis(400)).await;
            }
        }
    }

    autograb_trigger.send(())?;

    Ok(())
}

#[instrument(skip_all)]
async fn search_item(
    config: &Config,
    db: &Database<'_>,
    mam: &MaM<'_>,
    list: &GoodreadsList,
    item: &Item,
    mut db_item: ListItem,
    max_torrents: u64,
) -> Result<u64> {
    if !db_item.want_audio() && !db_item.want_ebook() {
        return Ok(0);
    }

    let has_updates = search_library(config, db, &mut db_item, item).context("search_library")?;
    if !list.dry_run && has_updates {
        let rw = db.rw_transaction()?;
        rw.upsert(db_item.clone())?;
        rw.commit()?;
    }
    if !db_item.want_audio() && !db_item.want_ebook() {
        return Ok(0);
    }

    let mut torrents = vec![];
    for grab in &list.grab {
        let results = search_grab(config, mam, item, &db_item, grab)
            .await
            .context("search_grab")?;
        torrents.push(results);
    }
    let mut audiobook = select_torrent(&torrents, OldMainCat::Audio);
    let mut ebook = select_torrent(&torrents, OldMainCat::Ebook);

    let mut has_updates = false;
    if audiobook.is_some() && ebook.is_some() {
        match list.prefer_format {
            Some(OldDbMainCat::Audio) => {
                let updated = not_wanted(&mut db_item.ebook_torrent, &mut ebook);
                has_updates = updated || has_updates;
            }
            Some(OldDbMainCat::Ebook) => {
                let updated = not_wanted(&mut db_item.audio_torrent, &mut audiobook);
                has_updates = updated || has_updates;
            }
            None => {}
        }
    }
    if !list.dry_run && has_updates {
        let rw = db.rw_transaction()?;
        rw.upsert(db_item.clone())?;
        rw.commit()?;
    }

    let mut has_updates = false;
    if check_cost(&mut db_item.audio_torrent, &mut audiobook) {
        has_updates = true;
    }
    if check_cost(&mut db_item.ebook_torrent, &mut ebook) {
        has_updates = true;
    }
    if !list.dry_run && has_updates {
        let rw = db.rw_transaction()?;
        rw.upsert(db_item.clone())?;
        rw.commit()?;
    }

    let mut has_updates = false;
    if let Some(found) = audiobook
        && db_item
            .audio_torrent
            .as_ref()
            .is_none_or(|t| !(t.status == TorrentStatus::Selected && t.mam_id == found.0.id))
    {
        db_item.audio_torrent = Some(ListItemTorrent {
            mam_id: found.0.id,
            status: TorrentStatus::Selected,
            at: Timestamp::now(),
        });
        has_updates = true;
    }
    if let Some(found) = ebook
        && db_item
            .ebook_torrent
            .as_ref()
            .is_none_or(|t| !(t.status == TorrentStatus::Selected && t.mam_id == found.0.id))
    {
        db_item.ebook_torrent = Some(ListItemTorrent {
            mam_id: found.0.id,
            status: TorrentStatus::Selected,
            at: Timestamp::now(),
        });
        has_updates = true;
    }
    if !list.dry_run && has_updates {
        let rw = db.rw_transaction()?;
        rw.upsert(db_item.clone())?;
        rw.commit()?;
    }

    let mut selected_torrents = 0;
    if let Some(audiobook) = audiobook {
        selected_torrents += select_torrents(
            config,
            db,
            mam,
            [audiobook.0.clone()].into_iter(),
            &audiobook.3.filter,
            audiobook.3.cost,
            list.unsat_buffer,
            None,
            list.dry_run,
            max_torrents,
            item.book_id,
        )
        .await
        .context("select_torrents")?;
    }
    if let Some(ebook) = ebook {
        selected_torrents += select_torrents(
            config,
            db,
            mam,
            [ebook.0.clone()].into_iter(),
            &ebook.3.filter,
            ebook.3.cost,
            list.unsat_buffer,
            None,
            list.dry_run,
            max_torrents,
            item.book_id,
        )
        .await
        .context("select_torrents")?;
    }

    Ok(selected_torrents)
}

#[instrument(skip_all)]
fn search_library(
    config: &Config,
    db: &Database<'_>,
    db_item: &mut ListItem,
    item: &Item,
) -> Result<bool> {
    let r = db.r_transaction()?;
    let title_search = normalize_title(&item.title);
    let mut library = {
        r.scan()
            .secondary::<Torrent>(TorrentKey::title_search)?
            .start_with(title_search.as_str())?
            .filter(|t| t.as_ref().is_ok_and(|t| db_item.matches(&t.meta)))
            .collect::<Result<Vec<_>, _>>()
    }?;

    library.sort_by_key(|torrent| {
        let preferred_types = torrent.meta.media_type.preferred_types(config);
        preferred_types
            .iter()
            .position(|t| torrent.meta.filetypes.contains(t))
    });

    let audiobook = library
        .iter()
        .find(|t| t.meta.media_type.matches(OldMainCat::Audio.into()));
    let ebook = library
        .iter()
        .find(|t| t.meta.media_type.matches(OldMainCat::Ebook.into()));

    let mut updated_any = false;
    if let Some(audiobook) = audiobook {
        let updated = set_existing(&mut db_item.audio_torrent, audiobook);
        trace!(
            "Found old audiobook torrent matching RSS item: {}",
            audiobook.meta.title
        );
        updated_any = updated || updated_any;
    }
    if let Some(ebook) = ebook {
        let updated = set_existing(&mut db_item.ebook_torrent, ebook);
        trace!(
            "Found old ebook torrent matching RSS item: {}",
            ebook.meta.title
        );
        updated_any = updated || updated_any;
    }

    Ok(updated_any)
}

fn set_existing(field: &mut Option<ListItemTorrent>, torrent: &Torrent) -> bool {
    if let Some(field) = field {
        if field.status == TorrentStatus::Selected {
            return false;
        }
        if field.status == TorrentStatus::Existing && field.mam_id == torrent.meta.mam_id {
            return false;
        }
    }
    field.replace(ListItemTorrent {
        mam_id: torrent.meta.mam_id,
        status: TorrentStatus::Existing,
        at: Timestamp::now(),
    });
    true
}

pub static BAD_CHARATERS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"['`/?]|\s+[\(\[][^\)\]]+[\)\]]").unwrap());

pub static AND: Lazy<Regex> = Lazy::new(|| Regex::new(r"&|\band\b").unwrap());

#[instrument(skip_all)]
async fn search_grab(
    config: &Config,
    mam: &MaM<'_>,
    item: &Item,
    db_item: &ListItem,
    grab: &Grab,
) -> Result<Vec<(MaMTorrent, TorrentMeta, usize, Grab)>> {
    let (flags_is_hide, flags) = grab.filter.flags.as_search_bitfield();

    let title_query = db_item.title.replace("*", "\"*\"");
    let title_query = BAD_CHARATERS.replace_all(&title_query, " ");
    let mut title_query = AND.replace_all(&title_query, "(and|&)");
    if let Some((primary_title, _)) = title_query.split_once(':') {
        title_query = Cow::from(format!("(\"{primary_title}\"|\"{title_query}\")"));
    }
    let query = format!(
        "@title {} @author ({})",
        title_query,
        db_item.authors.iter().map(|a| format!("\"{a}\"")).join("|")
    );

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
                    text: query.clone(),
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
                .map(|author| {
                    db_item
                        .authors
                        .iter()
                        .map(|author_name| score(author_name, author))
                        .max()
                        .unwrap_or_default()
                })
                .max()
                .unwrap_or_default();
            let series_score = item
                .series
                .as_ref()
                .and_then(|(i_name, i_num)| {
                    t.series_info
                        .values()
                        .map(|series| {
                            let Value::String(t_name) = series.get(0).unwrap_or(&Value::Null)
                            else {
                                return 0;
                            };
                            let Value::String(t_num) = series.get(1).unwrap_or(&Value::Null) else {
                                return 0;
                            };
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
            let preferred_types = meta.media_type.preferred_types(config);
            let preference = preferred_types
                .iter()
                .position(|t| meta.filetypes.contains(t));
            Ok((t, meta, preference.unwrap_or_default(), grab.clone()))
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
    book_id: Option<u64>,
    title: String,
    author_name: Option<String>,
    series: Option<(String, f64)>,
    book_large_image_url: Option<String>,
    // book_description: String,
    isbn: Option<String>,
    description: String,
}

impl Item {
    fn as_list_item(&self, list_id: &str, list: &GoodreadsList) -> ListItem {
        let fragment = Html::parse_fragment(&self.description);

        let cover_url = if let Some(cover) = &self.book_large_image_url {
            cover.clone()
        } else {
            let cover_selector = Selector::parse(".cover img").unwrap();
            fragment
                .select(&cover_selector)
                .next()
                .and_then(|e| e.attr("src"))
                .map(|s| s.to_string())
                .unwrap_or_default()
        };

        let user_list_selector =
            Selector::parse("a[href^=\"https://www.goodreads.com/book/show/\"]").unwrap();
        let group_list_selector = Selector::parse("a[href^=\"/book/show/\"]").unwrap();
        let book_url = fragment
            .select(&user_list_selector)
            .next()
            .and_then(|e| e.attr("href"))
            .map(|s| s.to_string())
            .or_else(|| {
                fragment
                    .select(&group_list_selector)
                    .next()
                    .and_then(|e| e.attr("href"))
                    .map(|url| format!("https://www.goodreads.com{url}"))
            });

        let authors = if let Some(author_name) = &self.author_name {
            vec![author_name.clone().replace('.', " ")]
        } else {
            let fragment = Html::parse_fragment(&self.description);
            let author_selector =
                Selector::parse("[itemprop=\"author\"] [itemprop=\"name\"]").unwrap();
            fragment
                .select(&author_selector)
                .map(|e| e.text().collect::<String>().replace('.', " "))
                .collect()
        };

        ListItem {
            guid: (list_id.to_owned(), self.guid.clone()),
            list_id: list_id.to_owned(),
            title: self.title.clone(),
            authors,
            series: self.series.iter().cloned().collect(),
            cover_url,
            book_url,
            isbn: self.isbn.as_ref().and_then(|isbn| isbn.parse().ok()),
            prefer_format: list.prefer_format,
            allow_audio: list.allow_audio(),
            audio_torrent: None,
            allow_ebook: list.allow_ebook(),
            ebook_torrent: None,
            created_at: Timestamp::now(),
            marked_done_at: None,
        }
    }
}

impl ListItem {
    fn matches(&self, meta: &TorrentMeta) -> bool {
        if score(&self.title, &meta.title) < 80 {
            return false;
        }

        let authors = self
            .authors
            .iter()
            .map(|a| a.to_lowercase())
            .collect::<Vec<_>>();

        meta.authors
            .iter()
            .map(|a| a.to_lowercase())
            .any(|a| authors.iter().any(|b| score(b, &a) > 90))
    }
}

fn select_torrent(
    torrents: &[Vec<(MaMTorrent, TorrentMeta, usize, Grab)>],
    main_cat: OldMainCat,
) -> Option<&(MaMTorrent, TorrentMeta, usize, Grab)> {
    torrents
        .iter()
        .flatten()
        .filter(|t| t.1.media_type.matches(main_cat.into()))
        .find(|t| t.3.cost != Cost::Free || t.0.is_free())
        .or_else(|| {
            torrents
                .iter()
                .flatten()
                .find(|t| t.1.media_type.matches(main_cat.into()))
        })
}

fn not_wanted(
    field: &mut Option<ListItemTorrent>,
    unwanted: &mut Option<&(MaMTorrent, TorrentMeta, usize, Grab)>,
) -> bool {
    let found = unwanted.take().unwrap();
    if field
        .as_ref()
        .is_none_or(|t| t.status != TorrentStatus::NotWanted)
    {
        debug!("Skipped {:?} torrent as is not wanted", found.1.main_cat);
        field.replace(ListItemTorrent {
            mam_id: found.0.id,
            status: TorrentStatus::NotWanted,
            at: Timestamp::now(),
        });
        true
    } else {
        false
    }
}
fn check_cost(
    field: &mut Option<ListItemTorrent>,
    selected: &mut Option<&(MaMTorrent, TorrentMeta, usize, Grab)>,
) -> bool {
    let take =
        selected.is_some_and(|selected| selected.3.cost == Cost::Free && !selected.0.is_free());
    if take {
        let found = selected.take().unwrap();
        warn!("Skipped {:?} torrent as it is not free", found.1.main_cat);
        if field
            .as_ref()
            .is_none_or(|t| !(t.status == TorrentStatus::Wanted && t.mam_id == found.0.id))
        {
            field.replace(ListItemTorrent {
                mam_id: found.0.id,
                status: TorrentStatus::Wanted,
                at: Timestamp::now(),
            });
            true
        } else {
            false
        }
    } else {
        false
    }
}
