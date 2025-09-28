use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
};
use native_db::Database;
use qbit::models::{Torrent as QbitTorrent, Tracker};

use crate::{
    audiobookshelf::{Abs, LibraryItem},
    config::Config,
    data::{Size, Torrent, TorrentMeta},
    mam::MaMTorrent,
    qbittorrent::{self},
    web::{AppError, MaMState, Page, pages::torrents::TorrentsPageFilter, series, tables::items},
};

pub async fn torrent_page(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
    Path(hash): Path<String>,
) -> std::result::Result<Html<String>, AppError> {
    let abs = config.audiobookshelf.as_ref().map(Abs::new);
    let Some(torrent) = db.r_transaction()?.get().primary::<Torrent>(hash)? else {
        return Err(AppError::NotFound);
    };
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
    if let Some((qbit_torrent, qbit)) = qbittorrent::get_torrent(&config, &torrent.hash).await? {
        let trackers = qbit.trackers(&torrent.hash).await?;
        // let categories = qbit.categories().await.map_err(QbitError)?;
        let tags = qbit.tags().await?;

        qbit_data = Some(QbitData {
            torrent: qbit_torrent,
            trackers,
            categories: vec![],
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

    let template = TorrentPageTemplate {
        abs_url: config
            .audiobookshelf
            .as_ref()
            .map(|abs| abs.url.clone())
            .unwrap_or_default(),
        torrent,
        book,
        mam_torrent,
        mam_meta,
        qbit_data,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

#[derive(Template)]
#[template(path = "pages/torrent.html")]
struct TorrentPageTemplate {
    abs_url: String,
    torrent: Torrent,
    book: Option<LibraryItem>,
    mam_torrent: Option<MaMTorrent>,
    mam_meta: Option<TorrentMeta>,
    qbit_data: Option<QbitData>,
}

impl Page for TorrentPageTemplate {}

#[derive(Debug)]
struct QbitData {
    torrent: QbitTorrent,
    trackers: Vec<Tracker>,
    categories: Vec<String>,
    tags: Vec<String>,
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
    return duration.join(" ");
}
