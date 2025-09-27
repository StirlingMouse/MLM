use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
};
use native_db::Database;

use crate::{
    audiobookshelf::{Abs, LibraryItem},
    config::Config,
    data::{Torrent, TorrentMeta},
    mam::MaMTorrent,
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

    println!("book: {:?}", book);
    println!("mam_torrent: {:?}", mam_torrent);
    println!(
        "mam_meta: {} {:?}",
        Some(&torrent.meta) == mam_meta.as_ref(),
        mam_meta
    );

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
}

impl Page for TorrentPageTemplate {}

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
