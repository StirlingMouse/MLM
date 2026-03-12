use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use mlm_db::{
    DatabaseExt as _, List, ListItem, ListItemTorrent, OldDbMainCat, OldMainCat, Timestamp,
    TorrentMeta, TorrentStatus,
};
use mlm_mam::{api::MaM, search::MaMTorrent};
use mlm_parse::clean_value;
use native_db::Database;
use once_cell::sync::Lazy;
use quick_xml::de::from_reader;
use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;
use tokio::time::sleep;
use tracing::{debug, instrument, trace, warn};

use crate::config::{Config, Cost, GoodreadsList, Grab};
use crate::{
    autograbber::select_torrents,
    lists::{search_grab, search_library},
};

pub static SERIES_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(.*?) \(([^)]*?),? #?(\d+(?:\.\d+)?)\)$").unwrap());

static IMPORT_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[instrument(skip_all)]
pub async fn run_goodreads_import(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    mam: Arc<MaM<'_>>,
    list: &GoodreadsList,
    max_torrents: u64,
    events: &crate::stats::Events,
) -> Result<()> {
    // Make sure we are only running one import at a time
    let _guard = IMPORT_MUTEX.lock().await;

    let content = reqwest::Client::new()
        .get(&list.url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36")
        .send()
        .await?
        .bytes()
        .await?;

    let mut rss: Rss = from_reader(&content[..])?;
    trace!("Scanning Goodreads list {}", rss.channel.title);

    let list_id = list.list_id()?;

    if !list.dry_run {
        let (_guard, rw) = db.rw_async().await?;
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
                        let (_guard, rw) = db.rw_async().await?;
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
                    let (_guard, rw) = db.rw_async().await?;
                    rw.insert(db_item.clone())?;
                    rw.commit()?;
                }
                db_item
            }
        };
        trace!("Searching for book {} from Goodreads list", item.title);
        search_item(
            &config,
            &db,
            &mam,
            list,
            &item,
            db_item,
            max_torrents,
            events,
        )
        .await
        .context("search goodreads book")?;
        sleep(Duration::from_millis(400)).await;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn search_item(
    config: &Config,
    db: &Database<'_>,
    mam: &MaM<'_>,
    list: &GoodreadsList,
    item: &Item,
    mut db_item: ListItem,
    max_torrents: u64,
    events: &crate::stats::Events,
) -> Result<u64> {
    if !db_item.want_audio() && !db_item.want_ebook() {
        return Ok(0);
    }

    let has_updates = search_library(config, db, &mut db_item).context("search_library")?;
    if !list.dry_run && has_updates {
        let (_guard, rw) = db.rw_async().await?;
        rw.upsert(db_item.clone())?;
        rw.commit()?;
    }
    if !db_item.want_audio() && !db_item.want_ebook() {
        return Ok(0);
    }

    let mut torrents = vec![];
    for grab in &list.grab {
        let results = search_grab(config, mam, &db_item, grab)
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
        let (_guard, rw) = db.rw_async().await?;
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
        let (_guard, rw) = db.rw_async().await?;
        rw.upsert(db_item.clone())?;
        rw.commit()?;
    }

    let mut has_updates = false;
    if let Some(found) = audiobook
        && db_item
            .audio_torrent
            .as_ref()
            .is_none_or(|t| !(t.status == TorrentStatus::Selected && t.mam_id == Some(found.0.id)))
    {
        db_item.audio_torrent = Some(ListItemTorrent {
            torrent_id: None,
            mam_id: Some(found.0.id),
            status: TorrentStatus::Selected,
            at: Timestamp::now(),
        });
        has_updates = true;
    }
    if let Some(found) = ebook
        && db_item
            .ebook_torrent
            .as_ref()
            .is_none_or(|t| !(t.status == TorrentStatus::Selected && t.mam_id == Some(found.0.id)))
    {
        db_item.ebook_torrent = Some(ListItemTorrent {
            torrent_id: None,
            mam_id: Some(found.0.id),
            status: TorrentStatus::Selected,
            at: Timestamp::now(),
        });
        has_updates = true;
    }
    if !list.dry_run && has_updates {
        let (_guard, rw) = db.rw_async().await?;
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
            list.wedge_buffer,
            None,
            list.dry_run,
            max_torrents,
            item.book_id,
            events,
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
            list.wedge_buffer,
            None,
            list.dry_run,
            max_torrents,
            item.book_id,
            events,
        )
        .await
        .context("select_torrents")?;
    }

    Ok(selected_torrents)
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
            torrent_id: None,
            mam_id: Some(found.0.id),
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
            .is_none_or(|t| !(t.status == TorrentStatus::Wanted && t.mam_id == Some(found.0.id)))
        {
            field.replace(ListItemTorrent {
                torrent_id: None,
                mam_id: Some(found.0.id),
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
