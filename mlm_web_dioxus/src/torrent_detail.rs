use crate::events::Event;
#[cfg(feature = "server")]
use crate::events::{EventType, MetadataSource, TorrentCost, TorrentMetaDiff};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "server")]
use mlm_core::{
    Context, ContextExt, Event as DbEvent, EventKey, EventType as DbEventType,
    MetadataSource as DbMetadataSource, Torrent as DbTorrent, TorrentCost as DbTorrentCost,
    metadata::mam_meta::match_meta,
};
#[cfg(feature = "server")]
use mlm_db::{DatabaseExt, Timestamp as DbTimestamp};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentDetailData {
    pub torrent: TorrentInfo,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentInfo {
    pub id: String,
    pub title: String,
    pub edition: Option<String>,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<Series>,
    pub tags: Vec<String>,
    pub description: String,
    pub media_type: String,
    pub main_cat: Option<String>,
    pub language: Option<String>,
    pub filetypes: Vec<String>,
    pub size: String,
    pub num_files: u64,
    pub categories: Vec<String>,
    pub flags: Option<String>,
    pub library_path: Option<PathBuf>,
    pub library_files: Vec<PathBuf>,
    pub linker: Option<String>,
    pub category: Option<String>,
    pub mam_id: Option<u64>,
    pub vip_status: Option<String>,
    pub source: String,
    pub uploaded_at: String,
    pub client_status: Option<String>,
    pub replaced_with: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Series {
    pub name: String,
    pub entries: String,
}

#[cfg(feature = "server")]
fn format_timestamp(ts: &DbTimestamp) -> String {
    let dt: time::OffsetDateTime = ts.0.into();
    dt.replace_nanosecond(0)
        .unwrap()
        .format(
            &time::format_description::parse_owned::<2>(
                "[year]-[month]-[day] [hour]:[minute]:[second]",
            )
            .unwrap(),
        )
        .unwrap_or_default()
}

#[cfg(feature = "server")]
fn format_series(series: &mlm_db::Series) -> String {
    use mlm_db::SeriesEntry;
    let entries: Vec<String> = series
        .entries
        .0
        .iter()
        .map(|e| match e {
            SeriesEntry::Num(n) => format!("#{n}"),
            SeriesEntry::Range(start, end) => format!("#{start}-{end}"),
            SeriesEntry::Part(entry, part) => format!("#{entry}p{part}"),
        })
        .collect();
    entries.join(", ")
}

#[server]
pub async fn get_torrent_detail(id: String) -> Result<TorrentDetailData, ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use itertools::Itertools;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let db = context.db();

    let torrent = db
        .r_transaction()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .get()
        .primary::<DbTorrent>(id.clone())
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Torrent not found".to_string()))?;

    let events_data: Vec<Event> = db
        .r_transaction()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .scan()
        .secondary(EventKey::torrent_id)
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .range(Some(id.clone())..=Some(id.clone()))
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .map_ok(|e: DbEvent| Event {
            id: e.id.0.to_string(),
            created_at: format_timestamp(&e.created_at),
            event: match e.event {
                DbEventType::Grabbed {
                    grabber,
                    cost,
                    wedged,
                } => EventType::Grabbed {
                    grabber,
                    cost: cost.map(|c| match c {
                        DbTorrentCost::Vip => TorrentCost::Vip,
                        DbTorrentCost::GlobalFreeleech => TorrentCost::GlobalFreeleech,
                        DbTorrentCost::PersonalFreeleech => TorrentCost::PersonalFreeleech,
                        DbTorrentCost::UseWedge => TorrentCost::UseWedge,
                        DbTorrentCost::TryWedge => TorrentCost::TryWedge,
                        DbTorrentCost::Ratio => TorrentCost::Ratio,
                    }),
                    wedged,
                },
                DbEventType::Linked {
                    linker,
                    library_path,
                } => EventType::Linked {
                    linker,
                    library_path,
                },
                DbEventType::Cleaned {
                    library_path,
                    files,
                } => EventType::Cleaned {
                    library_path,
                    files,
                },
                DbEventType::Updated { fields, source } => EventType::Updated {
                    fields: fields
                        .into_iter()
                        .map(|f| TorrentMetaDiff {
                            field: f.field.to_string(),
                            from: f.from,
                            to: f.to,
                        })
                        .collect(),
                    source: (
                        match source.0 {
                            DbMetadataSource::Mam => MetadataSource::Mam,
                            DbMetadataSource::Manual => MetadataSource::Manual,
                            DbMetadataSource::File => MetadataSource::File,
                            DbMetadataSource::Match => MetadataSource::Match,
                        },
                        source.1,
                    ),
                },
                DbEventType::RemovedFromTracker => EventType::RemovedFromTracker,
            },
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let library_files = torrent
        .library_path
        .as_ref()
        .and_then(|p| std::fs::read_dir(p).ok())
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let torrent_info = TorrentInfo {
        id: torrent.id.clone(),
        title: torrent.meta.title.clone(),
        edition: torrent.meta.edition.as_ref().map(|(ed, _)| ed.clone()),
        authors: torrent.meta.authors.clone(),
        narrators: torrent.meta.narrators.clone(),
        series: torrent
            .meta
            .series
            .iter()
            .map(|s| Series {
                name: s.name.clone(),
                entries: format_series(s),
            })
            .collect(),
        tags: torrent.meta.tags.clone(),
        description: torrent.meta.description.clone(),
        media_type: torrent.meta.media_type.to_string(),
        main_cat: torrent.meta.main_cat.map(|c| c.to_string()),
        language: torrent.meta.language.as_ref().map(|l| l.to_string()),
        filetypes: torrent
            .meta
            .filetypes
            .iter()
            .map(|f| f.to_string())
            .collect(),
        size: torrent.meta.size.to_string(),
        num_files: torrent.meta.num_files,
        categories: torrent.meta.categories.clone(),
        flags: torrent.meta.flags.as_ref().map(|f| format!("{:?}", f)),
        library_path: torrent.library_path.clone(),
        library_files,
        linker: torrent.linker.clone(),
        category: torrent.category.clone(),
        mam_id: torrent.mam_id,
        vip_status: torrent.meta.vip_status.as_ref().map(|v| v.to_string()),
        source: format!("{:?}", torrent.meta.source),
        uploaded_at: format_timestamp(&torrent.meta.uploaded_at),
        client_status: torrent.client_status.as_ref().map(|s| format!("{:?}", s)),
        replaced_with: torrent.replaced_with.as_ref().map(|(id, _)| id.clone()),
    };

    Ok(TorrentDetailData {
        torrent: torrent_info,
        events: events_data,
    })
}

#[server]
pub async fn remove_torrent_action(id: String) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;

    let torrent = context
        .db()
        .r_transaction()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .get()
        .primary::<DbTorrent>(id.clone())
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Torrent not found".to_string()))?;

    let (_guard, rw) = context
        .db()
        .rw_async()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    rw.remove(torrent)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    rw.commit().map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn clean_torrent_action(id: String) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use mlm_core::cleaner::clean_torrent;
    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let config = context.config().await;
    let Some(torrent) = context
        .db()
        .r_transaction()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .get()
        .primary::<DbTorrent>(id)
        .map_err(|e| ServerFnError::new(e.to_string()))?
    else {
        return Err(ServerFnError::new("Could not find torrent".to_string()));
    };
    clean_torrent(&config, context.db(), torrent, true, &context.events)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn refresh_metadata_action(id: String) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use mlm_core::linker::refresh_mam_metadata;
    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let config = context.config().await;
    let mam = context
        .mam()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    refresh_mam_metadata(&config, context.db(), &mam, id, &context.events)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn relink_torrent_action(id: String) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use mlm_core::linker::relink;
    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let config = context.config().await;
    relink(&config, context.db(), id, &context.events)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn refresh_and_relink_action(id: String) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use mlm_core::linker::refresh_metadata_relink;
    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let config = context.config().await;
    let mam = context
        .mam()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    refresh_metadata_relink(&config, context.db(), &mam, id, &context.events)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn match_metadata_action(id: String, provider: String) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use mlm_db::Event as DbEvent;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let Some(mut torrent) = context
        .db()
        .r_transaction()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .get()
        .primary::<DbTorrent>(id.clone())
        .map_err(|e| ServerFnError::new(e.to_string()))?
    else {
        return Err(ServerFnError::new("Could not find torrent".to_string()));
    };

    let (new_meta, pid, fields) = match_meta(&context, &torrent.meta, &provider)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let (_guard, rw) = context
        .db()
        .rw_async()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut meta = new_meta;
    meta.source = mlm_core::MetadataSource::Match;
    torrent.meta = meta;
    torrent.title_search = mlm_parse::normalize_title(&torrent.meta.title);

    rw.upsert(torrent.clone())
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    rw.commit().map_err(|e| ServerFnError::new(e.to_string()))?;
    drop(_guard);

    mlm_core::logging::write_event(
        context.db(),
        &context.events,
        DbEvent::new(
            Some(torrent.id.clone()),
            torrent.mam_id,
            mlm_core::EventType::Updated {
                fields: fields.clone(),
                source: (mlm_core::MetadataSource::Match, pid.clone()),
            },
        ),
    )
    .await;

    Ok(())
}

#[server]
pub async fn clear_replacement_action(id: String) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let (_guard, rw) = context
        .db()
        .rw_async()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let Some(mut torrent) = rw
        .get()
        .primary::<DbTorrent>(id)
        .map_err(|e| ServerFnError::new(e.to_string()))?
    else {
        return Err(ServerFnError::new("Could not find torrent".to_string()));
    };
    torrent.replaced_with.take();
    rw.upsert(torrent)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    rw.commit().map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn get_metadata_providers() -> Result<Vec<String>, ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    Ok(context.metadata().enabled_providers())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct QbitData {
    pub torrent_state: String,
    pub torrent_category: String,
    pub torrent_tags: Vec<String>,
    pub categories: Vec<QbitCategory>,
    pub tags: Vec<String>,
    pub trackers: Vec<QbitTracker>,
    pub uploaded: String,
    pub wanted_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct QbitCategory {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct QbitTracker {
    pub url: String,
}

#[server]
pub async fn get_qbit_data(id: String) -> Result<Option<QbitData>, ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use mlm_core::linker::{find_library, library_dir};
    use mlm_core::qbittorrent::get_torrent;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let config = context.config().await;
    let db = context.db();

    let torrent = db
        .r_transaction()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .get()
        .primary::<DbTorrent>(id.clone())
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Torrent not found".to_string()))?;

    let Some((qbit_torrent, qbit, _qbit_config)) = get_torrent(&config, &torrent.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    else {
        return Ok(None);
    };

    let mut categories: Vec<QbitCategory> = qbit
        .categories()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .into_values()
        .map(|cat| QbitCategory { name: cat.name })
        .collect();
    categories.sort_by(|a, b| a.name.cmp(&b.name));

    let tags: Vec<String> = qbit
        .tags()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let trackers = qbit
        .trackers(&torrent.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .into_iter()
        .map(|t| QbitTracker { url: t.url })
        .collect();

    let wanted_path = find_library(&config, &qbit_torrent).and_then(|library| {
        library_dir(
            config.exclude_narrator_in_library_dir,
            library,
            &torrent.meta,
        )
        .filter(|expected| torrent.library_path.as_ref() != Some(expected))
    });

    let torrent_tags: Vec<String> = qbit_torrent
        .tags
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    Ok(Some(QbitData {
        torrent_state: format!("{:?}", qbit_torrent.state),
        torrent_category: qbit_torrent.category.clone(),
        torrent_tags,
        categories,
        tags,
        trackers,
        uploaded: mlm_db::Size::from_bytes(qbit_torrent.uploaded as u64).to_string(),
        wanted_path,
    }))
}

#[server]
pub async fn torrent_start_action(id: String) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use mlm_core::qbittorrent::get_torrent;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let config = context.config().await;
    let Some((qbit_torrent, qbit, _config)) = get_torrent(&config, &id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    else {
        return Err(ServerFnError::new(
            "Torrent not found in qBittorrent".to_string(),
        ));
    };

    qbit.start(vec![&qbit_torrent.hash])
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[server]
pub async fn torrent_stop_action(id: String) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use mlm_core::qbittorrent::get_torrent;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let config = context.config().await;
    let Some((qbit_torrent, qbit, _config)) = get_torrent(&config, &id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    else {
        return Err(ServerFnError::new(
            "Torrent not found in qBittorrent".to_string(),
        ));
    };

    qbit.stop(vec![&qbit_torrent.hash])
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[server]
pub async fn set_qbit_category_tags_action(
    id: String,
    category: String,
    tags: Vec<String>,
) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use mlm_core::qbittorrent::get_torrent;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let config = context.config().await;
    let Some((qbit_torrent, qbit, _config)) = get_torrent(&config, &id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    else {
        return Err(ServerFnError::new(
            "Torrent not found in qBittorrent".to_string(),
        ));
    };

    qbit.set_category(Some(vec![&qbit_torrent.hash]), &category)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let existing_tags: Vec<String> = qbit_torrent
        .tags
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    if !existing_tags.is_empty() {
        qbit.remove_tags(
            Some(vec![&qbit_torrent.hash]),
            existing_tags.iter().map(|s| s.as_str()).collect(),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    }

    if !tags.is_empty() {
        qbit.add_tags(
            Some(vec![&qbit_torrent.hash]),
            tags.iter().map(|s| s.as_str()).collect(),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    }

    Ok(())
}

#[server]
pub async fn remove_seeding_files_action(id: String) -> Result<(), ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use mlm_core::qbittorrent::get_torrent;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let config = context.config().await;
    let db = context.db();

    let torrent = db
        .r_transaction()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .get()
        .primary::<DbTorrent>(id.clone())
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Torrent not found".to_string()))?;

    let Some((qbit_torrent, qbit, _config)) = get_torrent(&config, &id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    else {
        return Err(ServerFnError::new(
            "Torrent not found in qBittorrent".to_string(),
        ));
    };

    let files = qbit
        .files(&qbit_torrent.hash, None)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let library_files_set: std::collections::HashSet<_> = torrent
        .library_path
        .as_ref()
        .and_then(|p| std::fs::read_dir(p).ok())
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();

    let files_to_remove: Vec<_> = files
        .iter()
        .filter(|f| {
            let file_path = std::path::PathBuf::from(&qbit_torrent.save_path).join(&f.name);
            !library_files_set.contains(&file_path)
        })
        .map(|f| f.index)
        .collect();

    if !files_to_remove.is_empty() {
        for file_id in files_to_remove {
            let path = std::path::PathBuf::from(&qbit_torrent.save_path).join(
                files
                    .iter()
                    .find(|f| f.index == file_id)
                    .map(|f| &f.name)
                    .unwrap_or(&String::new()),
            );
            if path.exists() {
                std::fs::remove_file(path).map_err(|e| ServerFnError::new(e.to_string()))?;
            }
        }
    }

    Ok(())
}

#[component]
pub fn TorrentDetailPage(id: String) -> Element {
    let mut status_msg = use_signal(|| None::<(String, bool)>); // (message, is_error)

    let mut data_res = use_server_future(move || {
        let id = id.clone();
        async move {
            let detail = get_torrent_detail(id.clone()).await;
            let providers = get_metadata_providers().await;
            let qbit = get_qbit_data(id).await;
            (detail, providers, qbit)
        }
    })?;

    let data = data_res.suspend()?;
    let data = data.read();
    let (detail, providers, qbit) = &*data;

    rsx! {
        div { class: "torrent-detail-page",
            if let Some((msg, is_error)) = status_msg.read().as_ref() {
                div {
                    class: if *is_error { "error" } else { "success" },
                    style: "padding: 10px; margin-bottom: 10px; border-radius: 4px; background: if *is_error { \"#fdd\" } else { \"#dfd\" }; color: #000;",
                    "{msg}"
                    button {
                        style: "margin-left: 10px; cursor: pointer;",
                        onclick: move |_| status_msg.set(None),
                        "⨯"
                    }
                }
            }
            match detail {
                Ok(data) => {
                    rsx! {
                        TorrentDetailContent {
                            data: data.clone(),
                            providers: providers.as_ref().ok().cloned().unwrap_or_default(),
                            qbit_data: qbit.as_ref().ok().cloned().flatten(),
                            status_msg: status_msg,
                            on_refresh: move |_| data_res.restart()
                        }
                    }
                },
                Err(e) => rsx! { p { class: "error", "Error: {e}" } },
            }
        }
    }
}

#[component]
fn TorrentDetailContent(
    data: TorrentDetailData,
    providers: Vec<String>,
    qbit_data: Option<QbitData>,
    status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    let torrent = data.torrent;
    let events = data.events;

    let series_text = torrent
        .series
        .iter()
        .map(|s| format!("{} ({})", s.name, s.entries))
        .collect::<Vec<_>>()
        .join(", ");

    rsx! {
        div { class: "torrent-detail-grid",
            div { class: "torrent-side",
                div { class: "pill", "{torrent.media_type}" }

                if !torrent.categories.is_empty() {
                    div {
                        h3 { "Categories" }
                        for cat in &torrent.categories {
                            span { class: "pill", "{cat}" }
                        }
                    }
                }

                h3 { "Metadata" }
                dl { class: "metadata-table",
                    if let Some(lang) = &torrent.language {
                        dt { "Language" } dd { "{lang}" }
                    }
                    if let Some(ed) = &torrent.edition {
                        dt { "Edition" } dd { "{ed}" }
                    }
                    if let Some(mam_id) = torrent.mam_id {
                        dt { "MaM ID" }
                        dd {
                            a { href: "https://www.myanonamouse.net/t/{mam_id}", target: "_blank", "{mam_id}" }
                        }
                    }
                    dt { "Size" } dd { "{torrent.size}" }
                    dt { "Files" } dd { "{torrent.num_files}" }
                    if !torrent.filetypes.is_empty() {
                        dt { "File Types" } dd { "{torrent.filetypes.join(\", \")}" }
                    }
                    dt { "Uploaded" } dd { "{torrent.uploaded_at}" }
                    dt { "Source" } dd { "{torrent.source}" }
                    if let Some(vip) = &torrent.vip_status {
                        dt { "VIP" } dd { "{vip}" }
                    }
                    if let Some(path) = &torrent.library_path {
                        dt { "Library Path" } dd { "{path.display()}" }
                    }
                    if let Some(linker) = &torrent.linker {
                        dt { "Linker" } dd { "{linker}" }
                    }
                    if let Some(cat) = &torrent.category {
                        dt { "Category" } dd { "{cat}" }
                    }
                    if let Some(status) = &torrent.client_status {
                        dt { "Client Status" } dd { "{status}" }
                    }
                    if let Some(flags) = &torrent.flags {
                        dt { "Flags" } dd { "{flags}" }
                    }
                }
            }

            div { class: "torrent-main",
                h1 { "{torrent.title}" }

                if !torrent.authors.is_empty() {
                    p { strong { "Authors: " } "{torrent.authors.join(\", \")}" }
                }
                if !torrent.narrators.is_empty() {
                    p { strong { "Narrators: " } "{torrent.narrators.join(\", \")}" }
                }
                if !torrent.series.is_empty() {
                    p {
                        strong { "Series: " }
                        "{series_text}"
                    }
                }
                if !torrent.tags.is_empty() {
                    div {
                        strong { "Tags: " }
                        for tag in &torrent.tags {
                            span { class: "pill", "{tag}" }
                        }
                    }
                }

                TorrentActions {
                    torrent_id: torrent.id.clone(),
                    providers: providers,
                    has_replacement: torrent.replaced_with.is_some(),
                    status_msg: status_msg,
                    on_refresh: on_refresh
                }
            }

            div { class: "torrent-description",
                h3 { "Description" }
                p { "{torrent.description}" }

                h3 { "Event History" }
                for event in events {
                    div { class: "event-item",
                        "{event.created_at}: "
                        crate::events::EventContent {
                            event: event,
                            torrent: None, // Don't show redundant torrent info
                            replacement: None
                        }
                    }
                }
            }

            div { class: "torrent-below",
                if !torrent.library_files.is_empty() {
                    details {
                        summary { "Library Files ({torrent.library_files.len()})" }
                        ul {
                            for file in &torrent.library_files {
                                li { "{file.display()}" }
                            }
                        }
                    }
                }

                if let Some(qbit) = qbit_data {
                    QbitControls {
                        torrent_id: torrent.id,
                        qbit: qbit,
                        status_msg: status_msg,
                        on_refresh: on_refresh
                    }
                }
            }
        }
    }
}

#[component]
fn TorrentActions(
    torrent_id: String,
    providers: Vec<String>,
    has_replacement: bool,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    let mut selected_provider = use_signal(|| providers.first().cloned().unwrap_or_default());
    let mut loading = use_signal(|| false);

    let handle_action = move |name: String,
                              fut: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), ServerFnError>>>,
    >| {
        spawn(async move {
            loading.set(true);
            status_msg.set(None);
            match fut.await {
                Ok(_) => {
                    status_msg.set(Some((format!("{} succeeded", name), false)));
                    on_refresh.call(());
                    loading.set(false);
                }
                Err(e) => {
                    status_msg.set(Some((format!("{} failed: {}", name, e), true)));
                    loading.set(false);
                }
            }
        });
    };

    rsx! {
        div { class: "torrent-actions-widget", style: "margin-top: 1em;",
            h3 { "Actions" }

            div { style: "display: flex; gap: 0.5em; align-items: center; margin: 0.5em 0;",
                select {
                    disabled: *loading.read(),
                    onchange: move |ev| selected_provider.set(ev.value()),
                    for p in providers {
                        option { value: "{p}", "{p}" }
                    }
                }
                button {
                    class: "btn",
                    disabled: *loading.read(),
                    onclick: {
                        let torrent_id = torrent_id.clone();
                        move |_| {
                            let id = torrent_id.clone();
                            let provider = selected_provider.read().clone();
                            handle_action("Match Metadata".to_string(), Box::pin(match_metadata_action(id, provider)));
                        }
                    },
                    if *loading.read() { "Matching..." } else { "Match Metadata" }
                }
            }

            div { style: "display: flex; flex-wrap: wrap; gap: 0.5em;",
                button {
                    class: "btn",
                    disabled: *loading.read(),
                    onclick: {
                        let torrent_id = torrent_id.clone();
                        move |_| {
                            let id = torrent_id.clone();
                            handle_action("Clean".to_string(), Box::pin(clean_torrent_action(id)));
                        }
                    },
                    "Clean"
                }
                button {
                    class: "btn",
                    disabled: *loading.read(),
                    onclick: {
                        let torrent_id = torrent_id.clone();
                        move |_| {
                            let id = torrent_id.clone();
                            handle_action("Refresh".to_string(), Box::pin(refresh_metadata_action(id)));
                        }
                    },
                    "Refresh"
                }
                button {
                    class: "btn",
                    disabled: *loading.read(),
                    onclick: {
                        let torrent_id = torrent_id.clone();
                        move |_| {
                            let id = torrent_id.clone();
                            handle_action("Relink".to_string(), Box::pin(relink_torrent_action(id)));
                        }
                    },
                    "Relink"
                }
                button {
                    class: "btn",
                    disabled: *loading.read(),
                    onclick: {
                        let torrent_id = torrent_id.clone();
                        move |_| {
                            let id = torrent_id.clone();
                            handle_action("Refresh & Relink".to_string(), Box::pin(refresh_and_relink_action(id)));
                        }
                    },
                    "Refresh & Relink"
                }
                if has_replacement {
                    button {
                        class: "btn",
                        disabled: *loading.read(),
                        onclick: {
                            let torrent_id = torrent_id.clone();
                            move |_| {
                                let id = torrent_id.clone();
                                handle_action("Clear Replacement".to_string(), Box::pin(clear_replacement_action(id)));
                            }
                        },
                        "Clear Replacement"
                    }
                }
                button {
                    class: "btn",
                    style: "background: #fdd;",
                    disabled: *loading.read(),
                    onclick: {
                        let torrent_id = torrent_id.clone();
                        move |_| {
                            let id = torrent_id.clone();
                            handle_action("Remove".to_string(), Box::pin(remove_torrent_action(id)));
                        }
                    },
                    "Remove"
                }
            }
        }
    }
}

#[component]
fn QbitControls(
    torrent_id: String,
    qbit: QbitData,
    mut status_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> Element {
    let mut selected_category = use_signal(|| qbit.torrent_category.clone());
    let mut selected_tags = use_signal(|| qbit.torrent_tags.clone());
    let mut loading = use_signal(|| false);

    let is_paused = qbit.torrent_state.to_lowercase().contains("paused")
        || qbit.torrent_state.to_lowercase().contains("stopped");

    let handle_qbit_action = move |name: String,
                                   fut: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), ServerFnError>>>,
    >| {
        spawn(async move {
            loading.set(true);
            status_msg.set(None);
            match fut.await {
                Ok(_) => {
                    status_msg.set(Some((format!("{} succeeded", name), false)));
                    on_refresh.call(());
                    loading.set(false);
                }
                Err(e) => {
                    status_msg.set(Some((format!("{} failed: {}", name, e), true)));
                    loading.set(false);
                }
            }
        });
    };

    rsx! {
        div { style: "margin-top: 1em; padding: 1em; background: var(--above); border-radius: 4px;",
            h3 { "qBittorrent" }

            dl { class: "metadata-table",
                dt { "State" } dd { "{qbit.torrent_state}" }
                dt { "Uploaded" } dd { "{qbit.uploaded}" }
                if !qbit.trackers.is_empty() {
                    dt { "Trackers" }
                    dd { "{qbit.trackers.iter().map(|t| t.url.clone()).collect::<Vec<_>>().join(\", \")}" }
                }
            }

            if let Some(path) = qbit.wanted_path {
                div { style: "margin: 1em 0; padding: 0.5em; background: var(--bg); border-radius: 4px;",
                    p { strong { "⚠️ Torrent should be in: " } "{path.display()}" }
                    button {
                        class: "btn",
                        disabled: *loading.read(),
                        onclick: {
                            let torrent_id = torrent_id.clone();
                            move |_| {
                                let id = torrent_id.clone();
                                handle_qbit_action("Relink to Correct Path".to_string(), Box::pin(relink_torrent_action(id)));
                            }
                        },
                        "Relink to Correct Path"
                    }
                }
            }

            div { style: "display: flex; gap: 0.5em; margin: 1em 0;",
                if is_paused {
                    button {
                        class: "btn",
                        disabled: *loading.read(),
                        onclick: {
                            let torrent_id = torrent_id.clone();
                            move |_| {
                                let id = torrent_id.clone();
                                handle_qbit_action("Start".to_string(), Box::pin(torrent_start_action(id)));
                            }
                        },
                        "Start"
                    }
                } else {
                    button {
                        class: "btn",
                        disabled: *loading.read(),
                        onclick: {
                            let torrent_id = torrent_id.clone();
                            move |_| {
                                let id = torrent_id.clone();
                                handle_qbit_action("Stop".to_string(), Box::pin(torrent_stop_action(id)));
                            }
                        },
                        "Stop"
                    }
                }
                button {
                    class: "btn",
                    disabled: *loading.read(),
                    onclick: {
                        let torrent_id = torrent_id.clone();
                        move |_| {
                            let id = torrent_id.clone();
                            handle_qbit_action("Remove Seeding-only Files".to_string(), Box::pin(remove_seeding_files_action(id)));
                        }
                    },
                    "Remove Seeding-only Files"
                }
            }

            div { class: "option_group",
                "Category: "
                select {
                    disabled: *loading.read(),
                    onchange: move |ev| selected_category.set(ev.value()),
                    for cat in &qbit.categories {
                        option { value: "{cat.name}", selected: cat.name == qbit.torrent_category, "{cat.name}" }
                    }
                }
            }

            div { class: "option_group", style: "margin-top: 0.5em;",
                "Tags: "
                for tag in &qbit.tags {
                    label {
                        input {
                            r#type: "checkbox",
                            disabled: *loading.read(),
                            checked: selected_tags.read().contains(tag),
                            onchange: {
                                let tag = tag.clone();
                                move |ev| {
                                    if ev.value() == "true" {
                                        selected_tags.write().push(tag.clone());
                                    } else {
                                        selected_tags.write().retain(|t| t != &tag);
                                    }
                                }
                            }
                        }
                        "{tag}"
                    }
                }
            }

            button {
                class: "btn",
                style: "margin-top: 1em;",
                disabled: *loading.read(),
                onclick: {
                    let torrent_id = torrent_id.clone();
                    move |_| {
                        let id = torrent_id.clone();
                        let cat = selected_category.read().clone();
                        let tags = selected_tags.read().clone();
                        handle_qbit_action("Save Category & Tags".to_string(), Box::pin(set_qbit_category_tags_action(id, cat, tags)));
                    }
                },
                "Save Category & Tags"
            }
        }
    }
}
