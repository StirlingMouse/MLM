use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use itertools::Itertools;
use mlm_db::{
    AudiobookCategory, DatabaseExt as _, EbookCategory, FlagBits, Flags, Language, MetadataSource,
    OldCategory, Series, Torrent, TorrentMeta, impls::format_serie,
};
use native_db::Database;
use serde::Deserialize;

use crate::{
    autograbber::update_torrent_meta,
    stats::Context,
    web::{AppError, Page},
};

pub async fn torrent_edit_page(
    State(db): State<Arc<Database<'static>>>,
    Path(hash): Path<String>,
) -> std::result::Result<Html<String>, AppError> {
    let Some(torrent) = db.r_transaction()?.get().primary::<Torrent>(hash)? else {
        return Err(AppError::NotFound);
    };

    let template = TorrentPageTemplate {
        flags: Flags::from_bitfield(torrent.meta.flags.map_or(0, |f| f.0)),
        torrent,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn torrent_edit_page_post(
    State(context): State<Context>,
    Path(hash): Path<String>,
    Form(form): Form<TorrentPageForm>,
) -> Result<Redirect, AppError> {
    let config = context.config().await;
    let mam = context.mam()?;
    let Some(torrent) = context
        .db
        .r_transaction()?
        .get()
        .primary::<Torrent>(hash.clone())?
    else {
        return Err(anyhow::Error::msg("Could not find torrent").into());
    };
    let Some(mam_torrent) = mam.get_torrent_info(&hash).await? else {
        return Err(anyhow::Error::msg("Could not find torrent on MaM").into());
    };

    let authors = form.authors.split("\r\n").map(ToOwned::to_owned).collect();
    let narrators = if form.narrators.trim().is_empty() {
        vec![]
    } else {
        form.narrators
            .split("\r\n")
            .map(ToOwned::to_owned)
            .collect()
    };
    let series = if form.series.trim().is_empty() {
        vec![]
    } else {
        form.series
            .split("\r\n")
            .map(|s| {
                s.split_once(" #")
                    .map(|(s, n)| Series::try_from((s.to_string(), n.to_string())))
                    .unwrap_or_else(|| Series::try_from((s.to_string(), "".to_string())))
                    .map_err(|err| {
                        anyhow::Error::msg(format!("failed to parse series \"{s}\": {err}"))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    let language =
        Language::from_id(form.language).ok_or_else(|| anyhow::Error::msg("Invalid language"))?;
    let category = OldCategory::from_one_id(form.category)
        .ok_or_else(|| anyhow::Error::msg("Invalid category"))?;
    let flags = Flags {
        crude_language: Some(form.crude_language),
        violence: Some(form.violence),
        some_explicit: Some(form.some_explicit),
        explicit: Some(form.explicit),
        abridged: Some(form.abridged),
        lgbt: Some(form.lgbt),
    };

    let meta = TorrentMeta {
        title: form.title,
        media_type: category.as_main_cat().into(),
        cat: Some(category),
        language: Some(language),
        flags: Some(FlagBits::new(flags.as_bitfield())),
        authors,
        narrators,
        series,
        source: MetadataSource::Manual,
        ..torrent.meta.clone()
    };

    update_torrent_meta(
        &config,
        &context.db,
        context.db.rw_async().await?,
        &mam_torrent,
        torrent,
        meta,
        true,
        false,
    )
    .await?;

    Ok(Redirect::to(&format!("/torrents/{}", hash)))
}

#[derive(Debug, Deserialize)]
pub struct TorrentPageForm {
    title: String,
    authors: String,
    narrators: String,
    series: String,
    language: u8,
    category: u64,

    // #[serde(flatten)]
    // flags: Flags,
    #[serde(default)]
    crude_language: bool,
    #[serde(default)]
    violence: bool,
    #[serde(default)]
    some_explicit: bool,
    #[serde(default)]
    explicit: bool,
    #[serde(default)]
    abridged: bool,
    #[serde(default)]
    lgbt: bool,
}

#[derive(Template)]
#[template(path = "pages/torrent_edit.html")]
struct TorrentPageTemplate {
    torrent: Torrent,
    flags: Flags,
}

impl TorrentPageTemplate {
    fn series(&self) -> String {
        self.torrent.meta.series.iter().map(format_serie).join("\n")
    }
}

impl Page for TorrentPageTemplate {
    fn item_path(&self) -> &'static str {
        "/torrents"
    }
}
