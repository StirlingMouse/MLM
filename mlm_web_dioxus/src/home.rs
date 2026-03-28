use crate::components::TaskBox;
#[cfg(feature = "server")]
use crate::error::IntoServerFnError;
use crate::sse::STATS_UPDATE_TRIGGER;
#[cfg(feature = "server")]
use crate::utils::format_datetime;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HomeData {
    pub username: Option<String>,
    pub mam_error: Option<String>,
    pub has_no_qbits: bool,
    pub autograbbers: Vec<AutograbberInfo>,
    pub snatchlist_grabbers: Vec<AutograbberInfo>,
    pub lists: Vec<ListInfo>,
    pub torrent_linker: Option<TaskInfo>,
    pub folder_linker: Option<TaskInfo>,
    pub cleaner: Option<TaskInfo>,
    pub downloader: Option<TaskInfo>,
    pub mam_metadata_refresh: TaskInfo,
    pub audiobookshelf: Option<TaskInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AutograbberInfo {
    pub index: usize,
    pub display_name: String,
    pub last_run: Option<String>,
    pub result: Option<Result<(), String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ListInfo {
    pub index: usize,
    pub list_type: String,
    pub display_name: String,
    pub last_run: Option<String>,
    pub result: Option<Result<(), String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskInfo {
    pub last_run: Option<String>,
    pub result: Option<Result<(), String>>,
}

#[server]
pub async fn get_home_data() -> Result<HomeData, ServerFnError> {
    use mlm_core::ContextExt;

    let context = crate::error::get_context()?;
    let stats = context.stats.values.lock().await;

    let username = match context.mam() {
        Ok(mam) => mam.cached_user_info().await.map(|u| u.username),
        Err(_) => None,
    };

    let config = context.config().await;

    let mut autograbbers = Vec::new();
    for (i, grab) in config.autograbs.iter().enumerate() {
        autograbbers.push(AutograbberInfo {
            index: i,
            display_name: grab.filter.display_name(i),
            last_run: stats.autograbber_run_at.get(&i).map(format_datetime),
            result: stats
                .autograbber_result
                .get(&i)
                .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        });
    }

    let mut snatchlist_grabbers = Vec::new();
    for (i, grab) in config.snatchlist.iter().enumerate() {
        let idx = i + config.autograbs.len();
        snatchlist_grabbers.push(AutograbberInfo {
            index: idx,
            display_name: grab.filter.display_name(idx),
            last_run: stats.autograbber_run_at.get(&idx).map(format_datetime),
            result: stats
                .autograbber_result
                .get(&idx)
                .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        });
    }

    let config_lists = mlm_core::lists::get_lists(&config);
    let mut lists = Vec::new();
    for (i, list) in config_lists.iter().enumerate() {
        lists.push(ListInfo {
            index: i,
            list_type: list.list_type().to_string(),
            display_name: list.display_name(i),
            last_run: stats.import_run_at.get(&i).map(format_datetime),
            result: stats
                .import_result
                .get(&i)
                .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        });
    }

    let task_info = |run_at: Option<&time::OffsetDateTime>,
                     result: Option<&Result<(), anyhow::Error>>|
     -> Option<TaskInfo> {
        run_at.map(|t| TaskInfo {
            last_run: Some(format_datetime(t)),
            result: result.map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        })
    };

    let always_visible_task_info =
        |run_at: Option<&time::OffsetDateTime>, result: Option<&Result<(), anyhow::Error>>| {
            TaskInfo {
                last_run: run_at.map(format_datetime),
                result: result.map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
            }
        };

    Ok(HomeData {
        username,
        mam_error: context.mam().err().map(|e| format!("{e}")),
        has_no_qbits: config.qbittorrent.is_empty(),
        autograbbers,
        snatchlist_grabbers,
        lists,
        torrent_linker: task_info(
            stats.torrent_linker_run_at.as_ref(),
            stats.torrent_linker_result.as_ref(),
        ),
        folder_linker: task_info(
            stats.folder_linker_run_at.as_ref(),
            stats.folder_linker_result.as_ref(),
        ),
        cleaner: task_info(stats.cleaner_run_at.as_ref(), stats.cleaner_result.as_ref()),
        downloader: task_info(
            stats.downloader_run_at.as_ref(),
            stats.downloader_result.as_ref(),
        ),
        mam_metadata_refresh: always_visible_task_info(
            stats.mam_metadata_refresh_run_at.as_ref(),
            stats.mam_metadata_refresh_result.as_ref(),
        ),
        audiobookshelf: task_info(
            stats.audiobookshelf_run_at.as_ref(),
            stats.audiobookshelf_result.as_ref(),
        ),
    })
}

#[server]
pub async fn run_torrent_linker() -> Result<(), ServerFnError> {
    let context = crate::error::get_context()?;
    let tx = context
        .triggers
        .torrent_linker_tx
        .as_ref()
        .ok_or_else(|| ServerFnError::new("Torrent linker trigger is not configured"))?;
    tx.send(()).server_err()?;
    Ok(())
}

#[server]
pub async fn run_folder_linker() -> Result<(), ServerFnError> {
    let context = crate::error::get_context()?;
    let tx = context
        .triggers
        .folder_linker_tx
        .as_ref()
        .ok_or_else(|| ServerFnError::new("Folder linker trigger is not configured"))?;
    tx.send(()).server_err()?;
    Ok(())
}

#[server]
pub async fn run_search(index: usize) -> Result<(), ServerFnError> {
    let context = crate::error::get_context()?;
    if let Some(tx) = context.triggers.search_tx.get(&index) {
        tx.send(()).server_err()?;
    } else {
        return Err(ServerFnError::new("Invalid index"));
    }
    Ok(())
}

#[server]
pub async fn run_import(index: usize) -> Result<(), ServerFnError> {
    let context = crate::error::get_context()?;
    if let Some(tx) = context.triggers.import_tx.get(&index) {
        tx.send(()).server_err()?;
    } else {
        return Err(ServerFnError::new("Invalid index"));
    }
    Ok(())
}

#[server]
pub async fn run_downloader() -> Result<(), ServerFnError> {
    let context = crate::error::get_context()?;
    let tx = context
        .triggers
        .downloader_tx
        .as_ref()
        .ok_or_else(|| ServerFnError::new("Downloader trigger is not configured"))?;
    tx.send(()).server_err()?;
    Ok(())
}

#[server]
pub async fn run_mam_metadata_refresh() -> Result<(), ServerFnError> {
    let context = crate::error::get_context()?;
    let tx = context
        .triggers
        .mam_metadata_refresh_tx
        .as_ref()
        .ok_or_else(|| ServerFnError::new("MaM metadata refresh trigger is not configured"))?;
    tx.send(()).server_err()?;
    Ok(())
}

#[server]
pub async fn run_abs_matcher() -> Result<(), ServerFnError> {
    let context = crate::error::get_context()?;
    let tx = context
        .triggers
        .audiobookshelf_tx
        .as_ref()
        .ok_or_else(|| ServerFnError::new("Audiobookshelf trigger is not configured"))?;
    tx.send(()).server_err()?;
    Ok(())
}

#[component]
pub fn HomePage() -> Element {
    let mut home_data = use_server_future(move || async move { get_home_data().await })?;

    use_effect(move || {
        let _ = *STATS_UPDATE_TRIGGER.read();
        home_data.restart();
    });

    let data = home_data.suspend()?;
    let data = data.read();

    rsx! {
        match &*data {
            Ok(data) => rsx! { HomePageContent { data: data.clone() } },
            Err(e) => rsx! { p { class: "error", "Error loading home page: {e}" } },
        }
    }
}

#[component]
fn HomePageContent(data: HomeData) -> Element {
    let greeting = match &data.username {
        Some(u) => format!("Hi {}! Welcome to MLM, select a page above", u),
        None => "Welcome to MLM, select a page above".to_string(),
    };

    let mam_warning = data
        .mam_error
        .as_ref()
        .map(|err| format!("mam_id is invalid, all features are disabled: {}", err))
        .unwrap_or_default();

    let qbit_warning = if data.has_no_qbits {
        "no qbittorrent instances configured, all features are disabled"
    } else {
        ""
    };

    rsx! {
        div { class: "home-page",
            p { "{greeting}" }

            if !mam_warning.is_empty() {
                p { class: "missing", "{mam_warning}" }
            }

            if !qbit_warning.is_empty() {
                p { class: "missing", "{qbit_warning}" }
            }

            div { class: "infoboxes",
                {data.autograbbers.iter().map(|grab| {
                    let title = format!("Autograbber: {}", grab.display_name);
                    let last_run = grab.last_run.clone();
                    let result = grab.result.clone();
                    let index = grab.index;
                    rsx! {
                        InfoTaskBox {
                            title,
                            last_run,
                            result,
                            on_run: Some(EventHandler::new(move |_| {
                                spawn(async move { let _ = run_search(index).await; });
                            })),
                        }
                    }
                })}
                {data.snatchlist_grabbers.iter().map(|grab| {
                    let title = format!("Snatchlist: {}", grab.display_name);
                    let last_run = grab.last_run.clone();
                    let result = grab.result.clone();
                    let index = grab.index;
                    rsx! {
                        InfoTaskBox {
                            title,
                            last_run,
                            result,
                            on_run: Some(EventHandler::new(move |_| {
                                spawn(async move { let _ = run_search(index).await; });
                            })),
                        }
                    }
                })}
            }

            if !data.lists.is_empty() {
                div { class: "infoboxes",
                    {data.lists.iter().map(|list| {
                        let title = format!("{} Import: {}", list.list_type, list.display_name);
                        let last_run = list.last_run.clone();
                        let result = list.result.clone();
                        let index = list.index;
                        rsx! {
                            InfoTaskBox {
                                title,
                                last_run,
                                result,
                                on_run: Some(EventHandler::new(move |_| {
                                    spawn(async move { let _ = run_import(index).await; });
                                })),
                            }
                        }
                    })}
                }
            }

            div { class: "infoboxes",
                if let Some(info) = &data.torrent_linker {
                    InfoTaskBox {
                        title: "Torrent Linker".to_string(),
                        last_run: info.last_run.clone(),
                        result: info.result.clone(),
                        on_run: Some(EventHandler::new(move |_| {
                            spawn(async move { let _ = run_torrent_linker().await; });
                        })),
                    }
                }
                if let Some(info) = &data.folder_linker {
                    InfoTaskBox {
                        title: "Folder Linker".to_string(),
                        last_run: info.last_run.clone(),
                        result: info.result.clone(),
                        on_run: Some(EventHandler::new(move |_| {
                            spawn(async move { let _ = run_folder_linker().await; });
                        })),
                    }
                }
                if let Some(info) = &data.cleaner {
                    InfoTaskBox {
                        title: "Cleaner".to_string(),
                        last_run: info.last_run.clone(),
                        result: info.result.clone(),
                    }
                }
                if let Some(info) = &data.downloader {
                    InfoTaskBox {
                        title: "Torrent downloader".to_string(),
                        last_run: info.last_run.clone(),
                        result: info.result.clone(),
                        on_run: Some(EventHandler::new(move |_| {
                            spawn(async move { let _ = run_downloader().await; });
                        })),
                    }
                }
                InfoTaskBox {
                    title: "MaM Metadata Refresh".to_string(),
                    last_run: data.mam_metadata_refresh.last_run.clone(),
                    result: data.mam_metadata_refresh.result.clone(),
                    on_run: Some(EventHandler::new(move |_| {
                        spawn(async move { let _ = run_mam_metadata_refresh().await; });
                    })),
                }
                if let Some(info) = &data.audiobookshelf {
                    InfoTaskBox {
                        title: "Audiobookshelf Matcher".to_string(),
                        last_run: info.last_run.clone(),
                        result: info.result.clone(),
                        on_run: Some(EventHandler::new(move |_| {
                            spawn(async move { let _ = run_abs_matcher().await; });
                        })),
                    }
                }
            }

            hr {}
            p { style: "display:flex;align-items:center;gap:0.8ex",
                span { style: "font-size:2em", "🏳️‍⚧️" }
                " Trans Rights are Human Rights"
            }
        }
    }
}

#[component]
fn InfoTaskBox(
    title: String,
    last_run: Option<String>,
    result: Option<Result<(), String>>,
    #[props(default = None)] on_run: Option<EventHandler<()>>,
) -> Element {
    let has_run = last_run.is_some();
    rsx! {
        div { class: "infobox",
            h2 { "{title}" }
            TaskBox {
                last_run,
                result,
                show_result: has_run,
                on_run,
            }
        }
    }
}
