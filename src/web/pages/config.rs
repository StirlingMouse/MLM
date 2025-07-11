use std::sync::Arc;

use askama::Template;
use axum::{extract::State, response::Html};
use reqwest::Url;

use crate::{
    config::{Config, Cost, Library, TorrentFilter, Type},
    mam::DATE_FORMAT,
    mam_enums::{AudiobookCategory, EbookCategory},
    web::{AppError, filter, yaml_items},
};

pub async fn config_page(
    State(config): State<Arc<Config>>,
) -> std::result::Result<Html<String>, AppError> {
    let template = ConfigPageTemplate { config };
    Ok::<_, AppError>(Html(template.to_string()))
}

#[derive(Template)]
#[template(path = "pages/config.html")]
struct ConfigPageTemplate {
    config: Arc<Config>,
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
