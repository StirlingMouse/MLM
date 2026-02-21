use crate::components::TaskBox;
#[cfg(feature = "server")]
use crate::error::{IntoServerFnError, OptionIntoServerFnError};
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
    use dioxus_fullstack::FullstackContext;
    use mlm_core::{Context, ContextExt};

    let ctx = FullstackContext::current().ok_or_server_err("FullstackContext not found")?;
    let context: Context = ctx
        .extension()
        .ok_or_server_err("Context not found in extensions")?;
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

    Ok(HomeData {
        username,
        mam_error: context.mam().err().map(|e| format!("{e}")),
        has_no_qbits: config.qbittorrent.is_empty(),
        autograbbers,
        snatchlist_grabbers,
        lists,
        torrent_linker: stats.torrent_linker_run_at.as_ref().map(|t| TaskInfo {
            last_run: Some(format_datetime(t)),
            result: stats
                .torrent_linker_result
                .as_ref()
                .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        }),
        folder_linker: stats.folder_linker_run_at.as_ref().map(|t| TaskInfo {
            last_run: Some(format_datetime(t)),
            result: stats
                .folder_linker_result
                .as_ref()
                .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        }),
        cleaner: stats.cleaner_run_at.as_ref().map(|t| TaskInfo {
            last_run: Some(format_datetime(t)),
            result: stats
                .cleaner_result
                .as_ref()
                .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        }),
        downloader: stats.downloader_run_at.as_ref().map(|t| TaskInfo {
            last_run: Some(format_datetime(t)),
            result: stats
                .downloader_result
                .as_ref()
                .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        }),
        audiobookshelf: stats.audiobookshelf_run_at.as_ref().map(|t| TaskInfo {
            last_run: Some(format_datetime(t)),
            result: stats
                .audiobookshelf_result
                .as_ref()
                .map(|r| r.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))),
        }),
    })
}

#[server]
pub async fn run_torrent_linker() -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    let context: mlm_core::Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_server_err("Context not found in extensions")?;
    if let Some(tx) = &context.triggers.torrent_linker_tx {
        tx.send(()).server_err()?;
    }
    Ok(())
}

#[server]
pub async fn run_folder_linker() -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    let context: mlm_core::Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_server_err("Context not found in extensions")?;
    if let Some(tx) = &context.triggers.folder_linker_tx {
        tx.send(()).server_err()?;
    }
    Ok(())
}

#[server]
pub async fn run_search(index: usize) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    let context: mlm_core::Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_server_err("Context not found in extensions")?;
    if let Some(tx) = context.triggers.search_tx.get(&index) {
        tx.send(()).server_err()?;
    } else {
        return Err(ServerFnError::new("Invalid index"));
    }
    Ok(())
}

#[server]
pub async fn run_import(index: usize) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    let context: mlm_core::Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_server_err("Context not found in extensions")?;
    if let Some(tx) = context.triggers.import_tx.get(&index) {
        tx.send(()).server_err()?;
    } else {
        return Err(ServerFnError::new("Invalid index"));
    }
    Ok(())
}

#[server]
pub async fn run_downloader() -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    let context: mlm_core::Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_server_err("Context not found in extensions")?;
    if let Some(tx) = &context.triggers.downloader_tx {
        tx.send(()).server_err()?;
    }
    Ok(())
}

#[server]
pub async fn run_abs_matcher() -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;

    let context: mlm_core::Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_server_err("Context not found in extensions")?;
    if let Some(tx) = &context.triggers.audiobookshelf_tx {
        tx.send(()).server_err()?;
    }
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
                for grab in data.autograbbers.clone() {
                    AutograbberBox { info: grab }
                }
                for grab in data.snatchlist_grabbers.clone() {
                    AutograbberBox { info: grab }
                }
            }

            if !data.lists.is_empty() {
                div { class: "infoboxes",
                    for list in data.lists.clone() {
                        ListBox { info: list }
                    }
                }
            }

            div { class: "infoboxes",
                if let Some(info) = &data.torrent_linker {
                    TaskBoxWrapper {
                        title: "Torrent Linker".to_string(),
                        info: info.clone(),
                        action: "torrent_linker",
                    }
                }
                if let Some(info) = &data.folder_linker {
                    TaskBoxWrapper {
                        title: "Folder Linker".to_string(),
                        info: info.clone(),
                        action: "folder_linker",
                    }
                }
                if let Some(info) = &data.cleaner {
                    TaskBoxWrapper {
                        title: "Cleaner".to_string(),
                        info: info.clone(),
                        action: "cleaner",
                    }
                }
                if let Some(info) = &data.downloader {
                    TaskBoxWrapper {
                        title: "Torrent downloader".to_string(),
                        info: info.clone(),
                        action: "downloader",
                    }
                }
                if let Some(info) = &data.audiobookshelf {
                    TaskBoxWrapper {
                        title: "Audiobookshelf Matcher".to_string(),
                        info: info.clone(),
                        action: "audiobookshelf",
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
fn AutograbberBox(info: AutograbberInfo) -> Element {
    let index = info.index;
    let display_name = info.display_name.clone();
    let has_run = info.last_run.is_some();

    rsx! {
        div { class: "infobox",
            h2 { "Autograbber: {display_name}" }
            TaskBox {
                title: String::new(),
                last_run: info.last_run.clone(),
                result: info.result.clone(),
                show_result: has_run,
            }
            button {
                onclick: move |_| {
                    let index = index;
                    spawn(async move {
                        let _ = run_search(index).await;
                    });
                },
                "run now"
            }
        }
    }
}

#[component]
fn ListBox(info: ListInfo) -> Element {
    let index = info.index;
    let list_type = info.list_type.clone();
    let display_name = info.display_name.clone();
    let has_run = info.last_run.is_some();

    rsx! {
        div { class: "infobox",
            h2 { "{list_type} Import: {display_name}" }
            TaskBox {
                title: String::new(),
                last_run: info.last_run.clone(),
                result: info.result.clone(),
                show_result: has_run,
            }
            button {
                onclick: move |_| {
                    let index = index;
                    spawn(async move {
                        let _ = run_import(index).await;
                    });
                },
                "run now"
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct TaskBoxWrapperProps {
    title: String,
    info: TaskInfo,
    action: String,
}

#[component]
fn TaskBoxWrapper(props: TaskBoxWrapperProps) -> Element {
    let action = props.action.clone();
    let has_run = props.info.last_run.is_some();
    let has_action = action != "cleaner";

    rsx! {
        div { class: "infobox",
            h2 { "{props.title}" }
            TaskBox {
                title: String::new(),
                last_run: props.info.last_run.clone(),
                result: props.info.result.clone(),
                show_result: has_run,
                on_run: if has_action {
                    Some(EventHandler::new(move |_| {
                        let action = action.clone();
                        spawn(async move {
                            match action.as_str() {
                                "torrent_linker" => { let _ = run_torrent_linker().await; }
                                "folder_linker" => { let _ = run_folder_linker().await; }
                                "downloader" => { let _ = run_downloader().await; }
                                "audiobookshelf" => { let _ = run_abs_matcher().await; }
                                _ => {}
                            }
                        });
                    }))
                } else {
                    None
                },
            }
        }
    }
}
