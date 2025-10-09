use std::{
    collections::{BTreeSet, HashMap},
    ops::Deref,
    path::PathBuf,
    sync::Arc,
};

use askama::Template;
use axum::{
    extract::{OriginalUri, Path, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use itertools::Itertools;
use native_db::Database;
use qbit::{
    models::{Category, Torrent as QbitTorrent, Tracker},
    parameters::TorrentState,
};
use serde::Deserialize;

use crate::{
    audiobookshelf::{Abs, LibraryItemMinified},
    cleaner::clean_torrent,
    config::Config,
    data::{ClientStatus, Size, Torrent, TorrentMeta},
    linker::{find_library, library_dir, refresh_metadata_relink},
    mam::MaMTorrent,
    qbittorrent::{self},
    web::{AppError, MaMState, Page, pages::torrents::TorrentsPageFilter},
};

pub async fn torrent_page(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    Path(hash): Path<String>,
) -> std::result::Result<Html<String>, AppError> {
    let abs = config.audiobookshelf.as_ref().map(Abs::new);
    let Some(mut torrent) = db.r_transaction()?.get().primary::<Torrent>(hash)? else {
        return Err(AppError::NotFound);
    };
    let replacement_torrent = torrent
        .replaced_with
        .as_ref()
        .map(|(hash, _)| {
            db.r_transaction()?
                .get()
                .primary::<Torrent>(hash.to_string())
        })
        .transpose()?
        .flatten();

    if replacement_torrent.is_none() && torrent.replaced_with.is_some() {
        let rw = db.rw_transaction()?;
        torrent.replaced_with = None;
        rw.upsert(torrent.clone())?;
        rw.commit()?;
    }
    let book = match abs {
        Some(abs) => abs?.get_book(&torrent).await?,
        None => None,
    };
    let Ok(mam) = mam.as_ref() else {
        return Err(anyhow::Error::msg("mam_id error").into());
    };
    let mam_torrent = mam.get_torrent_info(&torrent.hash).await?;
    let mam_meta = mam_torrent.as_ref().map(|t| t.as_meta()).transpose()?;

    let mut qbit_data = None;
    let mut wanted_path = None;
    if let Some((qbit_torrent, qbit)) = qbittorrent::get_torrent(&config, &torrent.hash).await? {
        let trackers = qbit.trackers(&torrent.hash).await?;
        let mut categories = qbit.categories().await?.into_values().collect_vec();
        categories.sort_by(|a, b| a.name.cmp(&b.name));
        let tags = qbit.tags().await?;

        wanted_path = find_library(&config, &qbit_torrent).and_then(|library| {
            library_dir(
                config.exclude_narrator_in_library_dir,
                library,
                &torrent.meta,
            )
        });

        qbit_data = Some(QbitData {
            torrent_tags: qbit_torrent.tags.split(", ").map(str::to_string).collect(),
            torrent: qbit_torrent,
            trackers,
            categories,
            tags,
        });
    }

    println!("book: {:?}", book);
    println!("mam_torrent: {:?}", mam_torrent);
    println!(
        "mam_meta: {} {:?}",
        Some(&torrent.meta) == mam_meta.as_ref(),
        mam_meta
    );
    println!("qbit: {:?}", qbit_data);

    if qbit_data.is_none() && torrent.client_status != Some(ClientStatus::NotInClient) {
        let rw = db.rw_transaction()?;
        torrent.client_status = Some(ClientStatus::NotInClient);
        rw.upsert(torrent.clone())?;
        rw.commit()?;
    }

    let template = TorrentPageTemplate {
        abs_url: config
            .audiobookshelf
            .as_ref()
            .map(|abs| abs.url.clone())
            .unwrap_or_default(),
        torrent,
        replacement_torrent,
        book,
        mam_torrent,
        mam_meta,
        qbit_data,
        wanted_path,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn torrent_page_post(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    Path(hash): Path<String>,
    uri: OriginalUri,
    Form(form): Form<TorrentPageForm>,
) -> Result<Redirect, AppError> {
    match form.action.as_str() {
        "clean" => {
            let Some(torrent) = db.r_transaction()?.get().primary(hash)? else {
                return Err(anyhow::Error::msg("Could not find torrent").into());
            };
            clean_torrent(&config, &db, torrent).await?;
        }
        "refresh-relink" => {
            let Ok(mam) = mam.as_ref() else {
                return Err(anyhow::Error::msg("mam_id error").into());
            };
            refresh_metadata_relink(&config, &db, mam, hash).await?;
        }
        "remove" => {
            let rw = db.rw_transaction()?;
            let Some(torrent) = rw.get().primary::<Torrent>(hash)? else {
                return Err(anyhow::Error::msg("Could not find torrent").into());
            };
            rw.remove(torrent)?;
            rw.commit()?;
        }
        "torrent-start" => {
            let Some((_torrent, qbit)) = qbittorrent::get_torrent(&config, &hash).await? else {
                return Err(anyhow::Error::msg("Could not find torrent").into());
            };
            qbit.start(vec![&hash]).await?;
        }
        "torrent-stop" => {
            let Some((_torrent, qbit)) = qbittorrent::get_torrent(&config, &hash).await? else {
                return Err(anyhow::Error::msg("Could not find torrent").into());
            };
            qbit.stop(vec![&hash]).await?;
        }
        "clear-replacement" => {
            let rw = db.rw_transaction()?;
            let Some(mut torrent) = rw.get().primary::<Torrent>(hash)? else {
                return Err(anyhow::Error::msg("Could not find torrent").into());
            };
            torrent.replaced_with.take();
            rw.upsert(torrent)?;
            rw.commit()?;
        }
        "qbit" => {
            let Some((torrent, qbit)) = qbittorrent::get_torrent(&config, &hash).await? else {
                return Err(anyhow::Error::msg("Could not find torrent").into());
            };

            qbit.set_category(Some(vec![&hash]), &form.category).await?;
            let mut torrent_tags = torrent.tags.split(", ").collect::<BTreeSet<&str>>();
            if torrent.tags.is_empty() {
                torrent_tags.clear();
            }
            if !form.tags.is_empty() {
                let mut add_tags = form
                    .tags
                    .iter()
                    .map(|tag| tag.as_str())
                    .collect::<BTreeSet<&str>>();
                for tag in &torrent_tags {
                    add_tags.remove(tag);
                }
                if !add_tags.is_empty() {
                    println!("add tags {:?}", add_tags);
                    qbit.add_tags(Some(vec![&hash]), add_tags.into_iter().collect())
                        .await?;
                }
            }
            for tag in &form.tags {
                torrent_tags.remove(tag.as_str());
            }
            if !torrent_tags.is_empty() {
                println!("remove tags {torrent_tags:?}");
                qbit.remove_tags(
                    Some(vec![&hash]),
                    form.tags.iter().map(Deref::deref).collect(),
                )
                .await?;
            }
        }
        "remove-torrent" => {
            // let Some(torrent) = db.r_transaction()?.get().primary(hash)? else {
            //     return Err(anyhow::Error::msg("Could not find torrent").into());
            // };
            // remove_library_files(&torrent)?;
            for qbit_conf in config.qbittorrent.iter() {
                let qbit = qbit::Api::new_login_username_password(
                    &qbit_conf.url,
                    &qbit_conf.username,
                    &qbit_conf.password,
                )
                .await?;
                qbit.delete(vec![&hash], true).await?;
            }
        }
        action => {
            eprintln!("unknown action: {action}");
        }
    }

    Ok(Redirect::to(&uri.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct TorrentPageForm {
    action: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Template)]
#[template(path = "pages/torrent.html")]
struct TorrentPageTemplate {
    abs_url: String,
    torrent: Torrent,
    replacement_torrent: Option<Torrent>,
    book: Option<LibraryItemMinified>,
    mam_torrent: Option<MaMTorrent>,
    mam_meta: Option<TorrentMeta>,
    qbit_data: Option<QbitData>,
    wanted_path: Option<PathBuf>,
}

impl Page for TorrentPageTemplate {
    fn item_path(&self) -> &'static str {
        "/torrents"
    }
}

#[derive(Debug)]
struct QbitData {
    torrent: QbitTorrent,
    trackers: Vec<Tracker>,
    categories: Vec<Category>,
    tags: Vec<String>,
    torrent_tags: BTreeSet<String>,
}

fn duration(seconds: f64) -> String {
    let mut seconds = seconds as u64;
    let hours = seconds / 3600;
    seconds -= hours * 3600;
    let minutes = seconds / 60;
    seconds -= minutes * 60;
    let mut duration = vec![];
    if hours > 0 {
        duration.push(format!("{hours}h"));
    }
    if minutes > 0 {
        duration.push(format!("{minutes}m"));
    }
    if seconds > 0 {
        duration.push(format!("{seconds}s"));
    }
    duration.join(" ")
}
