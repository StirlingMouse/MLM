use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use native_db::Database;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    data::{AudiobookCategory, EbookCategory, List, ListItem, ListItemKey, TorrentStatus},
    web::{AppError, time},
};

pub async fn list_page(
    State(db): State<Arc<Database<'static>>>,
    Path(list_id): Path<String>,
    Query(filter): Query<Vec<(ListPageFilter, String)>>,
) -> std::result::Result<Html<String>, AppError> {
    let Some(list) = db.r_transaction()?.get().primary::<List>(list_id)? else {
        return Err(AppError::NotFound);
    };
    let items = db
        .r_transaction()?
        .scan()
        .secondary::<ListItem>(ListItemKey::created_at)?;
    let items = items
        .all()?
        .rev()
        .filter(|t| t.as_ref().is_ok_and(|t| t.list_id == list.id))
        .filter(|t| {
            let Ok(t) = t else {
                return true;
            };
            for (field, value) in filter.iter() {
                let ok = match field {
                    ListPageFilter::Show => match value.as_str() {
                        "any" => t.want_audio() || t.want_ebook(),
                        "audio" => t.want_audio(),
                        "ebook" => t.want_ebook(),
                        _ => true,
                    },
                };
                if !ok {
                    return false;
                }
            }
            true
        })
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    let template = ListPageTemplate {
        show: filter.iter().find_map(|f| {
            if f.0 == ListPageFilter::Show {
                Some(f.1.as_str())
            } else {
                None
            }
        }),
        list,
        items,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

#[derive(Template)]
#[template(path = "pages/list.html")]
struct ListPageTemplate<'a> {
    show: Option<&'a str>,
    list: List,
    items: Vec<ListItem>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListPageFilter {
    Show,
}

impl ListItem {
    fn mam_search(&self) -> String {
        let mut url: Url = "https://www.myanonamouse.net/tor/browse.php?thumbnail=true&tor[srchIn][title]=true&tor[srchIn][author]=true&tor[searchType]=all&tor[searchIn]=torrents"
            .parse()
            .unwrap();

        {
            let mut query = url.query_pairs_mut();
            query.append_pair(
                "tor[text]",
                &format!("{} {}", self.title, self.authors.join(" ")),
            );

            if self.allow_audio {
                for cat in AudiobookCategory::all()
                    .into_iter()
                    .map(AudiobookCategory::to_id)
                {
                    query.append_pair("tor[cat][]", &cat.to_string());
                }
            }
            if self.allow_ebook {
                for cat in EbookCategory::all().into_iter().map(EbookCategory::to_id) {
                    query.append_pair("tor[cat][]", &cat.to_string());
                }
            }
        }

        url.to_string()
    }
}
