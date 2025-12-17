use askama::Template;
use axum::{extract::State, response::Html};
use itertools::Itertools as _;

use crate::{
    config::GoodreadsList,
    data::{List, ListKey},
    stats::Context,
    web::{AppError, Page, time},
};

pub async fn lists_page(
    State(context): State<Context>,
) -> std::result::Result<Html<String>, AppError> {
    let config = context.config().await;
    let db_lists = context
        .db
        .r_transaction()?
        .scan()
        .secondary::<List>(ListKey::title)?;
    let mut db_lists = db_lists
        .all()?
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    let mut lists = vec![];

    for list in config.goodreads_lists.iter() {
        let id = list.list_id()?;
        if let Some((index, _)) = db_lists.iter().find_position(|db_list| db_list.id == id) {
            let db_list = db_lists.remove(index);
            lists.push((list.clone(), db_list));
        } else {
            lists.push((
                list.clone(),
                List {
                    id: id.clone(),
                    title: id,
                    updated_at: None,
                    build_date: None,
                },
            ));
        }
    }

    let template = ListsPageTemplate {
        lists,
        inactive_lists: db_lists,
    };
    Ok::<_, AppError>(Html(template.to_string()))
}

#[derive(Template)]
#[template(path = "pages/lists.html")]
struct ListsPageTemplate {
    lists: Vec<(GoodreadsList, List)>,
    inactive_lists: Vec<List>,
}

impl Page for ListsPageTemplate {}
