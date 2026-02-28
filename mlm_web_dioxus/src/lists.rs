#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GoodreadsListInfo {
    pub list_id: String,
    pub name: Option<String>,
    pub title: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ListInfo {
    pub id: String,
    pub title: String,
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ListsData {
    pub lists: Vec<(GoodreadsListInfo, ListInfo)>,
    pub inactive_lists: Vec<ListInfo>,
}

#[server]
pub async fn get_lists() -> Result<ListsData, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use itertools::Itertools as _;
        use mlm_core::ContextExt;
        use mlm_db::{List, ListKey};

        let context = crate::error::get_context()?;

        let config = context.config().await;
        let db = context.db();

        let db_lists: Vec<List> = db
            .r_transaction()
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .scan()
            .secondary::<List>(ListKey::title)
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .all()
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .filter_map(|t| t.ok())
            .collect();

        let mut remaining_db_lists = db_lists;
        let mut lists = Vec::new();

        for list in config.goodreads_lists.iter() {
            let id = match list.list_id() {
                Ok(id) => id,
                Err(_) => continue,
            };
            if let Some((index, db_list)) = remaining_db_lists
                .iter()
                .find_position(|db_list| db_list.id == id)
                .map(|(i, l)| (i, l.clone()))
            {
                remaining_db_lists.remove(index);
                let info = GoodreadsListInfo {
                    list_id: id.clone(),
                    name: list.name.clone(),
                    title: id.clone(),
                };
                let list_info = ListInfo {
                    id: db_list.id,
                    title: db_list.title,
                    updated_at: db_list.updated_at.map(|ts| format_timestamp_db(&ts)),
                };
                lists.push((info, list_info));
            } else {
                let info = GoodreadsListInfo {
                    list_id: id.clone(),
                    name: list.name.clone(),
                    title: id.clone(),
                };
                let list_info = ListInfo {
                    id: id.clone(),
                    title: id,
                    updated_at: None,
                };
                lists.push((info, list_info));
            }
        }

        let inactive_lists: Vec<ListInfo> = remaining_db_lists
            .into_iter()
            .map(|l| ListInfo {
                id: l.id,
                title: l.title,
                updated_at: l.updated_at.map(|ts| format_timestamp_db(&ts)),
            })
            .collect();

        Ok(ListsData {
            lists,
            inactive_lists,
        })
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server feature not enabled"))
    }
}

#[component]
pub fn ListsPage() -> Element {
    let mut lists_data = use_server_future(move || async move { get_lists().await })?;

    use_effect(move || {
        let _ = *crate::sse::STATS_UPDATE_TRIGGER.read();
        lists_data.restart();
    });

    let data = lists_data.suspend()?;
    let data = data.read();

    rsx! {
        match &*data {
            Ok(data) => rsx! { ListsPageContent { data: data.clone() } },
            Err(e) => rsx! { p { class: "error", "Error loading lists: {e}" } },
        }
    }
}

#[component]
fn ListsPageContent(data: ListsData) -> Element {
    rsx! {
        div { class: "lists-page",
            h1 { "Goodreads Lists" }
            p { "Goodreads lists can be used to autograb want to read books" }

            for (config, list) in &data.lists {
                div {
                    Link { to: Route::ListPage { id: list.id.clone() },
                        h3 {
                            if let Some(name) = &config.name {
                                "{name}"
                            } else {
                                "{config.title}"
                            }
                        }
                    }
                    p {
                        "Last updated: "
                        if let Some(updated_at) = &list.updated_at {
                            "{updated_at}"
                        } else {
                            i { "never" }
                        }
                    }
                }
            }

            if !data.inactive_lists.is_empty() {
                h2 { "Inactive Lists" }
                p { "Lists that have been removed from the config but are still in the database. They won't be refreshed or have books searched for at MaM." }

                for list in &data.inactive_lists {
                    div {
                        Link { to: Route::ListPage { id: list.id.clone() },
                            h3 { "{list.title}" }
                        }
                        p {
                            "Last updated: "
                            if let Some(updated_at) = &list.updated_at {
                                "{updated_at}"
                            } else {
                                i { "never" }
                            }
                        }
                    }
                }
            }

            if data.lists.is_empty() {
                p { i { "You have no Goodreads lists" } }
            }
        }
    }
}

use crate::app::Route;
