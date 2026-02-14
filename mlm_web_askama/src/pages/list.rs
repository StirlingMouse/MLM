use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{OriginalUri, Path, Query, State},
    response::{Html, Redirect},
};
use axum_extra::extract::Form;
use mlm_db::{
    AudiobookCategory, DatabaseExt as _, EbookCategory, List, ListItem, ListItemKey, Timestamp,
    TorrentStatus,
};
use native_db::Database;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{AppError, Page, time};

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

pub async fn list_page_post(
    State(db): State<Arc<Database<'static>>>,
    Path(list_id): Path<String>,
    uri: OriginalUri,
    Form(form): Form<ListPageForm>,
) -> Result<Redirect, AppError> {
    match form.action.as_str() {
        "mark-done" => {
            let (_guard, rw) = db.rw_async().await?;
            let Some(mut item) = rw.get().primary::<ListItem>((list_id, form.item_id))? else {
                return Err(anyhow::Error::msg("Could not find item").into());
            };
            item.marked_done_at = Some(Timestamp::now());
            rw.upsert(item)?;
            rw.commit()?;
        }
        action => {
            eprintln!("unknown action: {action}");
        }
    }

    Ok(Redirect::to(&uri.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct ListPageForm {
    action: String,
    item_id: String,
}

#[derive(Template)]
#[template(path = "pages/list.html")]
struct ListPageTemplate<'a> {
    show: Option<&'a str>,
    list: List,
    items: Vec<ListItem>,
}

impl<'a> Page for ListPageTemplate<'a> {}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListPageFilter {
    Show,
}

fn mam_search(item: &ListItem) -> String {
    let mut url: Url = "https://www.myanonamouse.net/tor/browse.php?thumbnail=true&tor[srchIn][title]=true&tor[srchIn][author]=true&tor[searchType]=all&tor[searchIn]=torrents"
            .parse()
            .unwrap();

    {
        let mut query = url.query_pairs_mut();
        query.append_pair(
            "tor[text]",
            &format!("{} {}", item.title, item.authors.join(" ")),
        );

        if item.allow_audio {
            for cat in AudiobookCategory::all()
                .into_iter()
                .map(AudiobookCategory::to_id)
            {
                query.append_pair("tor[cat][]", &cat.to_string());
            }
        }
        if item.allow_ebook {
            for cat in EbookCategory::all().into_iter().map(EbookCategory::to_id) {
                query.append_pair("tor[cat][]", &cat.to_string());
            }
        }
    }

    url.to_string()
}
