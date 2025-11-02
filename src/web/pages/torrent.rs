use std::{collections::BTreeSet, ops::Deref, path::PathBuf, sync::Arc};

use anyhow::Error;
use askama::Template;
use axum::{
    body::Body,
    extract::{OriginalUri, Path, State},
    response::{Html, IntoResponse, Redirect},
};
use axum_extra::extract::Form;
use itertools::Itertools;
use native_db::Database;
use qbit::{
    models::{Category, Torrent as QbitTorrent, Tracker},
    parameters::TorrentState,
};
use reqwest::header;
use serde::Deserialize;
use tokio_util::io::ReaderStream;
use tracing::info;

use crate::{
    audiobookshelf::{Abs, LibraryItemMinified},
    cleaner::clean_torrent,
    config::Config,
    data::{
        ClientStatus, Event, EventKey, EventType, SelectedTorrent, Size, Timestamp, Torrent,
        TorrentCost, TorrentKey, TorrentMeta,
    },
    linker::{find_library, library_dir, map_path, refresh_metadata, refresh_metadata_relink},
    mam::{MaMTorrent, normalize_title},
    qbittorrent::{self},
    stats::Triggers,
    web::{
        AppError, Conditional, MaMState, Page, TorrentLink, pages::torrents::TorrentsPageFilter,
        tables::table_styles, time,
    },
};

pub async fn torrent_file(
    State((config, db)): State<(Arc<Config>, Arc<Database<'static>>)>,
    Path((hash, filename)): Path<(String, String)>,
) -> impl IntoResponse {
    let Some(torrent) = db.r_transaction()?.get().primary::<Torrent>(hash)? else {
        return Err(AppError::NotFound);
    };
    let Some(path) = (if let (Some(library_path), Some(library_file)) = (
        &torrent.library_path,
        torrent
            .library_files
            .iter()
            .find(|f| f.to_string_lossy() == filename),
    ) {
        Some(library_path.join(library_file))
    } else if let Some((torrent, qbit, qbit_config)) =
        qbittorrent::get_torrent(&config, &torrent.hash).await?
    {
        qbit.files(&torrent.hash, None)
            .await?
            .into_iter()
            .find(|f| f.name == filename)
            .map(|file| map_path(&qbit_config.path_mapping, &torrent.save_path).join(&file.name))
    } else {
        None
    }) else {
        return Err(AppError::NotFound);
    };
    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(_) => return Err(AppError::NotFound),
    };
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let headers = [
        (header::CONTENT_TYPE, "text/toml; charset=utf-8".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        ),
    ];

    Ok((headers, body))
}

pub async fn torrent_page(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    Path(hash_or_id): Path<String>,
) -> std::result::Result<Html<String>, AppError> {
    if let Ok(id) = hash_or_id.parse() {
        torrent_page_id(State((config, db, mam)), Path(id)).await
    } else {
        torrent_page_hash(State((config, db, mam)), Path(hash_or_id)).await
    }
}

async fn torrent_page_id(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    Path(mam_id): Path<u64>,
) -> std::result::Result<Html<String>, AppError> {
    if let Some(torrent) = db
        .r_transaction()?
        .scan()
        .secondary::<Torrent>(TorrentKey::mam_id)?
        .range(mam_id..=mam_id)?
        .next()
    {
        return torrent_page_hash(State((config, db, mam)), Path(torrent?.hash)).await;
    };

    let Ok(mam) = mam.as_ref() else {
        return Err(anyhow::Error::msg("mam_id error").into());
    };
    let Some(mam_torrent) = mam.get_torrent_info_by_id(mam_id).await? else {
        return Err(AppError::NotFound);
    };
    let meta = mam_torrent.as_meta()?;

    println!("mam_torrent: {:?}", mam_torrent);
    println!("mam_meta: {:?}", meta);

    let template = TorrentMamPageTemplate { mam_torrent, meta };
    Ok::<_, AppError>(Html(template.to_string()))
}

async fn torrent_page_hash(
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

    let events = db
        .r_transaction()?
        .scan()
        .secondary::<Event>(EventKey::created_at)?;
    let events = events.all()?.rev();
    let events = events
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            t.hash.as_deref() == Some(&torrent.hash)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let Ok(mam) = mam.as_ref() else {
        return Err(anyhow::Error::msg("mam_id error").into());
    };
    let mam_torrent = mam.get_torrent_info(&torrent.hash).await?;
    let mam_meta = mam_torrent.as_ref().map(|t| t.as_meta()).transpose()?;

    let mut qbit_data = None;
    let mut wanted_path = None;
    let mut qbit_files = vec![];
    if let Some((qbit_torrent, qbit, _)) = qbittorrent::get_torrent(&config, &torrent.hash).await? {
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

        qbit_files = qbit.files(&torrent.hash, None).await?;
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
        events,
        book,
        mam_torrent,
        mam_meta,
        qbit_data,
        wanted_path,
        qbit_files,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn torrent_page_post(
    State((config, db, mam, triggers)): State<(
        Arc<Config>,
        Arc<Database<'static>>,
        MaMState,
        Triggers,
    )>,
    Path(hash_or_id): Path<String>,
    uri: OriginalUri,
    Form(form): Form<TorrentPageForm>,
) -> Result<Redirect, AppError> {
    if let Ok(id) = hash_or_id.parse() {
        torrent_page_post_id(
            State((config, db, mam, triggers)),
            Path(id),
            uri,
            Form(form),
        )
        .await
    } else {
        torrent_page_post_hash(State((config, db, mam)), Path(hash_or_id), uri, Form(form)).await
    }
}

pub async fn torrent_page_post_id(
    State((config, db, mam, triggers)): State<(
        Arc<Config>,
        Arc<Database<'static>>,
        MaMState,
        Triggers,
    )>,
    Path(mam_id): Path<u64>,
    uri: OriginalUri,
    Form(form): Form<TorrentPageForm>,
) -> Result<Redirect, AppError> {
    if let Some(torrent) = db
        .r_transaction()?
        .scan()
        .secondary::<Torrent>(TorrentKey::mam_id)?
        .range(mam_id..=mam_id)?
        .next()
        .transpose()?
    {
        return torrent_page_post_hash(
            State((config, db, mam)),
            Path(torrent.hash),
            uri,
            Form(form),
        )
        .await;
    };

    let Ok(mam) = mam.as_ref() else {
        return Err(anyhow::Error::msg("mam_id error").into());
    };
    let Some(torrent) = mam.get_torrent_info_by_id(mam_id).await? else {
        return Err(AppError::NotFound);
    };

    match form.action.as_str() {
        "select" | "wedge" => {
            let meta = torrent.as_meta()?;
            let tags: Vec<_> = config
                .tags
                .iter()
                .filter(|t| t.filter.matches(&torrent))
                .collect();
            let category = tags.iter().find_map(|t| t.category.clone());
            let tags = tags.iter().flat_map(|t| t.tags.clone()).collect();
            let cost = if torrent.vip > 0 {
                TorrentCost::Vip
            } else if torrent.personal_freeleech > 0 {
                TorrentCost::PersonalFreeleech
            } else if torrent.free > 0 {
                TorrentCost::GlobalFreeleech
            } else if form.action == "wedge" {
                TorrentCost::UseWedge
            } else {
                TorrentCost::Ratio
            };
            info!(
                "Selecting torrent \"{}\" in format {}, cost: {:?}, with category {:?} and tags {:?}",
                torrent.title, torrent.filetype, cost, category, tags
            );
            let rw = db.rw_transaction()?;
            rw.insert(SelectedTorrent {
                mam_id: torrent.id,
                hash: None,
                dl_link: torrent
                    .dl
                    .clone()
                    .ok_or_else(|| Error::msg(format!("no dl field for torrent {}", torrent.id)))?,
                unsat_buffer: None,
                cost,
                category,
                tags,
                title_search: normalize_title(&torrent.title),
                meta,
                grabber: None,
                created_at: Timestamp::now(),
                started_at: None,
                removed_at: None,
            })?;
            rw.commit()?;
            triggers.downloader_tx.send(())?;
        }
        action => {
            eprintln!("unknown action: {action}");
        }
    }

    Ok(Redirect::to(&uri.to_string()))
}

pub async fn torrent_page_post_hash(
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
            clean_torrent(&config, &db, torrent, true).await?;
        }
        "refresh" => {
            let Ok(mam) = mam.as_ref() else {
                return Err(anyhow::Error::msg("mam_id error").into());
            };
            refresh_metadata(&config, &db, mam, hash).await?;
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
            let Some((_torrent, qbit, _)) = qbittorrent::get_torrent(&config, &hash).await? else {
                return Err(anyhow::Error::msg("Could not find torrent").into());
            };
            qbit.start(vec![&hash]).await?;
        }
        "torrent-stop" => {
            let Some((_torrent, qbit, _)) = qbittorrent::get_torrent(&config, &hash).await? else {
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
            let Some((torrent, qbit, _)) = qbittorrent::get_torrent(&config, &hash).await? else {
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
    events: Vec<Event>,
    book: Option<LibraryItemMinified>,
    mam_torrent: Option<MaMTorrent>,
    mam_meta: Option<TorrentMeta>,
    qbit_data: Option<QbitData>,
    wanted_path: Option<PathBuf>,
    qbit_files: Vec<qbit::models::TorrentContent>,
}

impl TorrentPageTemplate {
    fn torrent_title<'a>(&'a self, torrent: &'a Option<Torrent>) -> Conditional<TorrentLink<'a>> {
        Conditional {
            template: torrent.as_ref().map(|t| TorrentLink {
                hash: &t.hash,
                title: &t.meta.title,
            }),
        }
    }
}

impl Page for TorrentPageTemplate {
    fn item_path(&self) -> &'static str {
        "/torrents"
    }
}

#[derive(Template)]
#[template(path = "pages/torrent_mam.html")]
struct TorrentMamPageTemplate {
    mam_torrent: MaMTorrent,
    meta: TorrentMeta,
}

impl Page for TorrentMamPageTemplate {
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
