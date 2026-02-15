mod goodreads;
mod notion;

use std::{borrow::Cow, sync::Arc};

use crate::config::{Config, GoodreadsList, Grab, NotionList};
use crate::lists::{goodreads::run_goodreads_import, notion::run_notion_import};
use anyhow::{Context, Result};
use itertools::Itertools;
use matchr::score;
use mlm_db::{
    ListItem, ListItemTorrent, OldMainCat, Torrent, TorrentKey, TorrentMeta, TorrentStatus,
};
use mlm_mam::{
    api::MaM,
    enums::SearchIn,
    search::{MaMTorrent, SearchFields, SearchQuery, SearchResult, Tor},
    serde::DATE_FORMAT,
};
use mlm_parse::normalize_title;
use native_db::Database;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use tokio::sync::watch::Sender;
use tracing::{debug, instrument, trace};

pub enum List {
    Goodreads(GoodreadsList),
    Notion(NotionList),
}

impl List {
    pub fn list_type(&self) -> &'static str {
        match self {
            List::Goodreads(_) => "Goodreads",
            List::Notion(_) => "Notion",
        }
    }

    pub fn display_name(&self, index: usize) -> String {
        match self {
            List::Goodreads(list) => list.name.clone().unwrap_or_else(|| index.to_string()),
            List::Notion(list) => list.name.clone(),
        }
    }

    pub fn search_interval(&self) -> Option<u64> {
        match self {
            List::Goodreads(list) => list.search_interval,
            List::Notion(list) => list.search_interval,
        }
    }

    fn unsat_buffer(&self) -> Option<u64> {
        match self {
            List::Goodreads(list) => list.unsat_buffer,
            List::Notion(list) => list.unsat_buffer,
        }
    }
}

pub fn get_lists(config: &Config) -> Vec<List> {
    let mut lists = vec![];
    for goodreads in &config.goodreads_lists {
        lists.push(List::Goodreads(goodreads.clone()));
    }
    for notion in &config.notion_lists {
        lists.push(List::Notion(notion.clone()));
    }
    lists
}

#[instrument(skip_all)]
pub async fn run_list_import(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    mam: Arc<MaM<'_>>,
    list: Arc<List>,
    index: usize,
    autograb_trigger: Sender<()>,
    events: &crate::stats::Events,
) -> Result<()> {
    let user_info = mam.user_info().await?;
    let max_torrents = user_info.unsat.limit.saturating_sub(user_info.unsat.count);
    debug!(
        "{} import, name: {}, unsats: {:#?}; max_torrents: {max_torrents}",
        list.list_type(),
        list.display_name(index),
        user_info.unsat
    );

    let max_torrents =
        max_torrents.saturating_sub(list.unsat_buffer().unwrap_or(config.unsat_buffer));

    if max_torrents > 0 {
        match list.as_ref() {
            List::Goodreads(list) => {
                run_goodreads_import(config, db, mam, list, max_torrents, events).await?;
            }
            List::Notion(list) => {
                run_notion_import(config, db, mam, list, max_torrents, events).await?;
            }
        }
    }

    autograb_trigger.send(())?;

    Ok(())
}

static BAD_CHARATERS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"['`/?]|\s+[\(\[][^\)\]]+[\)\]]").unwrap());

static AND: Lazy<Regex> = Lazy::new(|| Regex::new(r"&|\band\b").unwrap());

#[instrument(skip_all)]
fn search_library(config: &Config, db: &Database<'_>, db_item: &mut ListItem) -> Result<bool> {
    let r = db.r_transaction()?;
    let title_search = normalize_title(&db_item.title);
    let mut library = {
        r.scan()
            .secondary::<Torrent>(TorrentKey::title_search)?
            .start_with(title_search.as_str())?
            .filter(|t| t.as_ref().is_ok_and(|t| db_item.matches(&t.meta)))
            .collect::<Result<Vec<_>, _>>()
    }?;

    library.sort_by_key(|torrent| {
        let preferred_types = config.preferred_types(&torrent.meta.media_type);
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
        if field.status == TorrentStatus::Existing && field.mam_id == torrent.mam_id {
            return false;
        }
    }
    field.replace(ListItemTorrent {
        torrent_id: Some(torrent.id.clone()),
        mam_id: torrent.mam_id,
        status: TorrentStatus::Existing,
        at: torrent.created_at,
    });
    true
}

#[instrument(skip_all)]
async fn search_grab(
    config: &Config,
    mam: &MaM<'_>,
    db_item: &ListItem,
    grab: &Grab,
) -> Result<Vec<(MaMTorrent, TorrentMeta, usize, Grab)>> {
    let (flags_is_hide, flags) = grab.filter.edition.flags.as_search_bitfield();

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

    let mut categories = grab.filter.edition.categories.clone();
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
                fields: SearchFields {
                    dl_link: true,
                    ..Default::default()
                },
                perpage: 100,
                tor: Tor {
                    start_number: results.as_ref().map_or(0, |r| r.data.len() as u64),
                    text: query.clone(),
                    srch_in: vec![SearchIn::Title, SearchIn::Author],
                    main_cat: categories.get_main_cats(),
                    cat: categories.get_cats(),
                    browse_lang: grab
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
                    start_date: grab
                        .filter
                        .uploaded_after
                        .map_or_else(|| Ok(String::new()), |d| d.format(&DATE_FORMAT))?,
                    end_date: grab
                        .filter
                        .uploaded_before
                        .map_or_else(|| Ok(String::new()), |d| d.format(&DATE_FORMAT))?,
                    min_size: grab.filter.edition.min_size.bytes(),
                    max_size: grab.filter.edition.max_size.bytes(),
                    unit: grab
                        .filter
                        .edition
                        .min_size
                        .unit()
                        .max(grab.filter.edition.max_size.unit()),
                    min_seeders: grab.filter.min_seeders,
                    max_seeders: grab.filter.max_seeders,
                    min_leechers: grab.filter.min_leechers,
                    max_leechers: grab.filter.max_leechers,
                    min_snatched: grab.filter.min_snatched,
                    max_snatched: grab.filter.max_snatched,
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
            let title_score = score(&db_item.title, &t.title);
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
            let series_score: usize = db_item
                .series
                .iter()
                .map(|(i_name, i_num)| {
                    t.series_info
                        .values()
                        .map(|series| {
                            let Value::String(t_name) = series.first().unwrap_or(&Value::Null)
                            else {
                                return 0;
                            };
                            let Value::String(t_num) = series.get(1).unwrap_or(&Value::Null) else {
                                return 0;
                            };
                            score(i_name, t_name) + score(&i_num.to_string(), t_num)
                        })
                        .max()
                        .unwrap_or_default()
                })
                .sum();
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
            let preferred_types = config.preferred_types(&meta.media_type);
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
