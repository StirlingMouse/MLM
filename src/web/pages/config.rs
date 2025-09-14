use std::{ops::Deref, sync::Arc};

use anyhow::Result;
use askama::Template;
use axum::{
    extract::{OriginalUri, Query, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use native_db::Database;
use reqwest::Url;
use serde::Deserialize;
use tracing::{error, info};

use crate::{
    config::{Config, Cost, Library, TorrentFilter, Type},
    data::{AudiobookCategory, EbookCategory, Torrent},
    mam::{DATE_FORMAT, MaM},
    qbittorrent::QbitError,
    web::{AppError, filter, yaml_items},
};

pub async fn config_page(
    State(config): State<Arc<Config>>,
    Query(query): Query<ConfigPageQuery>,
) -> std::result::Result<Html<String>, AppError> {
    let template = ConfigPageTemplate {
        config,
        show_apply_tags: query.show_apply_tags.unwrap_or_default(),
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

pub async fn config_page_post(
    State((config, db, mam)): State<(
        Arc<Config>,
        Arc<Database<'static>>,
        Arc<Result<Arc<MaM<'static>>>>,
    )>,
    uri: OriginalUri,
    Form(form): Form<ConfigPageForm>,
) -> Result<Redirect, AppError> {
    match form.action.as_str() {
        "apply" => {
            let tag_filter = form
                .tag_filter
                .ok_or(anyhow::Error::msg("apply requires tag_filter"))?;
            let tag_filter = config
                .tags
                .get(tag_filter)
                .ok_or(anyhow::Error::msg("invalid tag_filter"))?;
            let qbit_conf = config
                .qbittorrent
                .get(form.qbit_index.unwrap_or_default())
                .ok_or(anyhow::Error::msg("requires a qbit config"))?;
            let qbit = qbit::Api::login(&qbit_conf.url, &qbit_conf.username, &qbit_conf.password)
                .await
                .map_err(QbitError)?;
            let torrents = db.r_transaction()?.scan().primary::<Torrent>()?;
            for torrent in torrents.all()? {
                let torrent = torrent?;
                match tag_filter.filter.matches_lib(&torrent) {
                    Ok(matches) => {
                        if !matches {
                            continue;
                        }
                    }
                    Err(err) => {
                        info!("need to ask mam due to: {err}");
                        let Ok(mam) = mam.as_ref() else {
                            return Err(anyhow::Error::msg("mam_id error").into());
                        };
                        let Some(mam_torrent) = mam.get_torrent_info(&torrent.hash).await? else {
                            continue;
                        };
                        if !tag_filter.filter.matches(&mam_torrent) {
                            continue;
                        }
                    }
                };
                if let Some(category) = &tag_filter.category {
                    qbit.set_category(Some(vec![torrent.hash.as_str()]), category)
                        .await
                        .map_err(QbitError)?;
                    info!(
                        "set category {} on torrent {}",
                        category, torrent.meta.title
                    );
                }

                if !tag_filter.tags.is_empty() {
                    qbit.add_tags(
                        Some(vec![torrent.hash.as_str()]),
                        tag_filter.tags.iter().map(Deref::deref).collect(),
                    )
                    .await
                    .map_err(QbitError)?;
                    println!(
                        "set tags {:?} on torrent {}",
                        tag_filter.tags, torrent.meta.title
                    );
                }
            }
        }
        action => {
            eprintln!("unknown action: {action}");
        }
    }

    Ok(Redirect::to(&uri.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct ConfigPageQuery {
    show_apply_tags: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigPageForm {
    action: String,
    qbit_index: Option<usize>,
    tag_filter: Option<usize>,
}

#[derive(Template)]
#[template(path = "pages/config.html")]
struct ConfigPageTemplate {
    config: Arc<Config>,
    show_apply_tags: bool,
}

impl TorrentFilter {
    fn mam_search(&self) -> String {
        let mut url: Url = "https://www.myanonamouse.net/tor/browse.php?thumbnail=true"
            .parse()
            .unwrap();

        {
            let mut query = url.query_pairs_mut();
            if let Some(text) = &self.query {
                query.append_pair("tor[text]", text);
            }

            for srch_in in &self.search_in {
                query.append_pair(&format!("tor[srchIn][{}]", srch_in.as_str()), "true");
            }
            let search_in = match self.kind {
                Type::Bookmarks => "bookmarks",
                _ => "torrents",
            };
            query.append_pair("tor[searchIn]", search_in);
            let search_type = match (self.kind, self.cost) {
                (Type::Freeleech, _) => "fl",
                (_, Cost::Free) => "fl-VIP",
                _ => "all",
            };
            query.append_pair("tor[searchType]", search_type);
            let sort_type = match self.kind {
                Type::New => "dateDesc",
                _ => "",
            };
            if !sort_type.is_empty() {
                query.append_pair("tor[sort_type]", search_type);
            }
            for cat in self
                .filter
                .categories
                .audio
                .clone()
                .unwrap_or_else(AudiobookCategory::all)
                .into_iter()
                .map(AudiobookCategory::to_id)
            {
                query.append_pair("tor[cat][]", &cat.to_string());
            }
            for cat in self
                .filter
                .categories
                .ebook
                .clone()
                .unwrap_or_else(EbookCategory::all)
                .into_iter()
                .map(EbookCategory::to_id)
            {
                query.append_pair("tor[cat][]", &cat.to_string());
            }
            for lang in &self.filter.languages {
                query.append_pair("tor[browse_lang][]", &lang.to_id().to_string());
            }

            let (flags_is_hide, flags) = self.filter.flags.as_search_bitfield();
            if !flags.is_empty() {
                query.append_pair(
                    "tor[browseFlagsHideVsShow]",
                    if flags_is_hide { "0" } else { "1" },
                );
            }
            for flag in flags {
                query.append_pair("tor[browseFlags][]", &flag.to_string());
            }

            if self.filter.min_size.bytes() > 0 || self.filter.max_size.bytes() > 0 {
                query.append_pair("tor[unit]", "1");
            }
            if self.filter.min_size.bytes() > 0 {
                query.append_pair("tor[minSize]", &self.filter.min_size.bytes().to_string());
            }
            if self.filter.max_size.bytes() > 0 {
                query.append_pair("tor[maxSize]", &self.filter.max_size.bytes().to_string());
            }

            if let Some(uploaded_after) = self.filter.uploaded_after {
                query.append_pair(
                    "tor[startDate]",
                    &uploaded_after.format(&DATE_FORMAT).unwrap(),
                );
            }
            if let Some(uploaded_before) = self.filter.uploaded_before {
                query.append_pair(
                    "tor[endDate]",
                    &uploaded_before.format(&DATE_FORMAT).unwrap(),
                );
            }
            if let Some(min_seeders) = self.filter.min_seeders {
                query.append_pair("tor[minSeeders]", &min_seeders.to_string());
            }
            if let Some(max_seeders) = self.filter.max_seeders {
                query.append_pair("tor[maxSeeders]", &max_seeders.to_string());
            }
            if let Some(min_leechers) = self.filter.min_leechers {
                query.append_pair("tor[minLeechers]", &min_leechers.to_string());
            }
            if let Some(max_leechers) = self.filter.max_leechers {
                query.append_pair("tor[maxLeechers]", &max_leechers.to_string());
            }
            if let Some(min_snatched) = self.filter.min_snatched {
                query.append_pair("tor[minSnatched]", &min_snatched.to_string());
            }
            if let Some(max_snatched) = self.filter.max_snatched {
                query.append_pair("tor[maxSnatched]", &max_snatched.to_string());
            }
        }

        url.to_string()
    }
}
