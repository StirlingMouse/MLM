use std::sync::Arc;

use askama::Template;
use axum::{extract::State, response::Html};
use native_db::Database;

use crate::{
    data::{List, ListKey},
    web::AppError,
};

pub async fn lists_page(
    State(db): State<Arc<Database<'static>>>,
) -> std::result::Result<Html<String>, AppError> {
    let lists = db
        .r_transaction()?
        .scan()
        .secondary::<List>(ListKey::title)?;
    let lists = lists
        .all()?
        .collect::<Result<Vec<_>, native_db::db_type::Error>>()?;
    let template = ListsPageTemplate { lists };
    Ok::<_, AppError>(Html(template.to_string()))
}

#[derive(Template)]
#[template(path = "pages/lists.html")]
struct ListsPageTemplate {
    lists: Vec<List>,
}
