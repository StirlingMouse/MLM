#[cfg(feature = "server")]
use crate::dto::{Event as DbEventDto, Series, convert_event_type};
#[cfg(feature = "server")]
use crate::error::{IntoServerFnError, OptionIntoServerFnError};
use crate::search::SearchTorrent;
#[cfg(feature = "server")]
use crate::utils::format_timestamp_db;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use mlm_core::{
    Context, ContextExt, Event as DbEvent, EventKey,
    Torrent as DbTorrent, metadata::mam_meta::match_meta,
};
#[cfg(feature = "server")]
use mlm_db::DatabaseExt;
#[cfg(feature = "server")]
use mlm_db::ids;

#[cfg(feature = "server")]
fn format_qbit_state(state: &qbit::parameters::TorrentState) -> String {
    use qbit::parameters::TorrentState;
    match state {
        TorrentState::Downloading => "Downloading".to_string(),
        TorrentState::Uploading => "Seeding".to_string(),
        TorrentState::StoppedDownloading => "Stopped (Downloading)".to_string(),
        TorrentState::StoppedUploading => "Stopped (Seeding)".to_string(),
        TorrentState::QueuedDownloading => "Queued (Downloading)".to_string(),
        TorrentState::QueuedUploading => "Queued (Seeding)".to_string(),
        TorrentState::StalledDownloading => "Stalled (Downloading)".to_string(),
        TorrentState::StalledUploading => "Stalled (Seeding)".to_string(),
        TorrentState::CheckingDownloading => "Checking (Downloading)".to_string(),
        TorrentState::CheckingUploading => "Checking (Seeding)".to_string(),
        TorrentState::CheckingResumeData => "Checking Resume Data".to_string(),
        TorrentState::ForcedDownloading => "Forced Downloading".to_string(),
        TorrentState::ForcedUploading => "Forced Seeding".to_string(),
        TorrentState::Allocating => "Allocating".to_string(),
        TorrentState::Error => "Error".to_string(),
        TorrentState::MissingFiles => "Missing Files".to_string(),
        TorrentState::Moving => "Moving".to_string(),
        TorrentState::MetadataDownloading => "Metadata Downloading".to_string(),
        TorrentState::ForcedMetadataDownloading => "Forced Metadata Downloading".to_string(),
        TorrentState::Unknown => "Unknown".to_string(),
    }
}

#[cfg(feature = "server")]
fn map_event(e: DbEvent) -> DbEventDto {
    DbEventDto {
        id: e.id.0.to_string(),
        created_at: format_timestamp_db(&e.created_at),
        event: convert_event_type(&e.event),
    }
}

#[cfg(feature = "server")]
fn torrent_info_from_meta(
    meta: &mlm_db::TorrentMeta,
    id: String,
    mam_id: Option<u64>,
) -> super::types::TorrentInfo {
    use mlm_parse::clean_html;

    let goodreads_id = meta.ids.get(ids::GOODREADS).cloned();
    let flags = mlm_db::Flags::from_bitfield(meta.flags.map_or(0, |f| f.0));
    let flag_values = crate::utils::flags_to_strings(&flags);

    super::types::TorrentInfo {
        id,
        title: meta.title.clone(),
        edition: meta.edition.as_ref().map(|(ed, _)| ed.clone()),
        authors: meta.authors.clone(),
        narrators: meta.narrators.clone(),
        series: meta
            .series
            .iter()
            .map(|s| Series {
                name: s.name.clone(),
                entries: s.entries.to_string(),
            })
            .collect(),
        tags: meta.tags.clone(),
        description: clean_html(&meta.description),
        media_type: meta.media_type.to_string(),
        main_cat: meta.main_cat.map(|c| c.to_string()),
        language: meta.language.as_ref().map(|l| l.to_string()),
        filetypes: meta.filetypes.iter().map(|f| f.to_string()).collect(),
        size: meta.size.to_string(),
        num_files: meta.num_files,
        categories: meta.categories.clone(),
        flags: flag_values,
        library_path: None,
        library_files: vec![],
        linker: None,
        category: None,
        mam_id,
        vip_status: meta.vip_status.as_ref().map(|v| v.to_string()),
        source: format!("{:?}", meta.source),
        uploaded_at: format_timestamp_db(&meta.uploaded_at),
        client_status: None,
        replaced_with: None,
        goodreads_id,
    }
}

#[cfg(feature = "server")]
fn map_mam_torrent(mam_torrent: &mlm_mam::search::MaMTorrent) -> super::types::MamTorrentInfo {
    super::types::MamTorrentInfo {
        id: mam_torrent.id,
        owner_name: mam_torrent.owner_name.clone(),
        tags: mam_torrent.tags.clone(),
        description: mam_torrent.description.clone(),
        vip: mam_torrent.vip,
        personal_freeleech: mam_torrent.personal_freeleech,
        free: mam_torrent.free,
    }
}

#[cfg(feature = "server")]
async fn other_torrents_data(
    context: &Context,
    meta: &mlm_db::TorrentMeta,
) -> Result<Vec<SearchTorrent>, ServerFnError> {
    use itertools::Itertools;
    use mlm_mam::{
        enums::SearchIn,
        search::{SearchFields, SearchQuery, Tor},
    };

    let mam = context.mam().server_err()?;
    let config = context.config().await;
    let title = meta
        .title
        .split_once(':')
        .map_or(meta.title.as_str(), |(base, _)| base)
        .trim()
        .to_string();
    let text = if meta.authors.is_empty() {
        title
    } else {
        format!(
            "{} ({})",
            title,
            meta.authors
                .iter()
                .map(|author| format!("\"{author}\""))
                .join(" | ")
        )
    };

    let result = mam
        .search(&SearchQuery {
            fields: SearchFields {
                media_info: true,
                ..Default::default()
            },
            tor: Tor {
                text,
                srch_in: vec![SearchIn::Title, SearchIn::Author],
                ..Default::default()
            },
            ..Default::default()
        })
        .await
        .server_err()?;

    let r = context.db().r_transaction().server_err()?;

    result
        .data
        .into_iter()
        .filter(|t| Some(t.id) != meta.mam_id())
        .map(|mam_torrent| {
            let meta = mam_torrent.as_meta().server_err()?;
            let torrent = r
                .get()
                .secondary::<DbTorrent>(mlm_db::TorrentKey::mam_id, Some(mam_torrent.id))
                .server_err()?;
            let selected = r
                .get()
                .primary::<mlm_db::SelectedTorrent>(mam_torrent.id)
                .server_err()?;
            let can_wedge = config
                .search
                .wedge_over
                .is_some_and(|wedge_over| meta.size >= wedge_over && !mam_torrent.is_free());
            let media_duration = mam_torrent
                .media_info
                .as_ref()
                .map(|m| m.general.duration.clone());
            let media_format = mam_torrent
                .media_info
                .as_ref()
                .map(|m| format!("{} {}", m.general.format, m.audio.format));
            let audio_bitrate = mam_torrent
                .media_info
                .as_ref()
                .map(|m| format!("{} {}", m.audio.bitrate, m.audio.mode));
            let old_category = meta.cat.as_ref().map(|cat| cat.to_string());

            Ok(SearchTorrent {
                mam_id: mam_torrent.id,
                mediatype_id: mam_torrent.mediatype,
                main_cat_id: mam_torrent.main_cat,
                lang_code: mam_torrent.lang_code,
                title: meta.title.clone(),
                edition: meta.edition.as_ref().map(|(ed, _)| ed.clone()),
                authors: meta.authors.clone(),
                narrators: meta.narrators.clone(),
                series: meta
                    .series
                    .iter()
                    .map(|s| Series {
                        name: s.name.clone(),
                        entries: s.entries.to_string(),
                    })
                    .collect(),
                tags: mam_torrent.tags,
                categories: meta.categories.clone(),
                flags: {
                    let flags = mlm_db::Flags::from_bitfield(meta.flags.map_or(0, |f| f.0));
                    crate::utils::flags_to_strings(&flags)
                },
                old_category,
                media_type: meta.media_type.as_str().to_string(),
                size: meta.size.to_string(),
                filetypes: meta.filetypes.clone(),
                num_files: mam_torrent.numfiles,
                uploaded_at: mam_torrent.added,
                owner_name: mam_torrent.owner_name,
                seeders: mam_torrent.seeders,
                leechers: mam_torrent.leechers,
                snatches: mam_torrent.times_completed,
                comments: mam_torrent.comments,
                media_duration,
                media_format,
                audio_bitrate,
                vip: mam_torrent.vip,
                personal_freeleech: mam_torrent.personal_freeleech,
                free: mam_torrent.free,
                is_downloaded: torrent.is_some(),
                is_selected: selected.is_some(),
                can_wedge,
            })
        })
        .collect()
}

#[cfg(feature = "server")]
async fn get_downloaded_torrent_detail(
    context: &Context,
    torrent_id: String,
) -> Result<super::types::TorrentDetailData, ServerFnError> {
    use mlm_core::audiobookshelf::Abs;
    use time::UtcDateTime;

    let config = context.config().await;
    let db = context.db();
    let mut torrent = db
        .r_transaction()
        .server_err()?
        .get()
        .primary::<DbTorrent>(torrent_id.clone())
        .server_err()?
        .ok_or_server_err("Torrent not found")?;

    let replacement_torrent = torrent
        .replaced_with
        .as_ref()
        .map(|(id, _)| {
            db.r_transaction()
                .server_err()?
                .get()
                .primary::<DbTorrent>(id.clone())
                .server_err()
        })
        .transpose()?
        .flatten();
    let replacement_missing = replacement_torrent.is_none() && torrent.replaced_with.is_some();
    if replacement_missing {
        let (_guard, rw) = db.rw_async().await.server_err()?;
        torrent.replaced_with = None;
        rw.upsert(torrent.clone()).server_err()?;
        rw.commit().server_err()?;
    }

    let mut mam_torrent = None;
    let mut mam_meta_diff = vec![];
    if let Some(mam_id) = torrent.mam_id
        && let Ok(mam) = context.mam()
    {
        mam_torrent = mam.get_torrent_info_by_id(mam_id).await.server_err()?;
        if let Some(ref mam_torrent_data) = mam_torrent {
            let mut mam_meta = mam_torrent_data.as_meta().server_err()?;
            let mut ids = torrent.meta.ids.clone();
            ids.append(&mut mam_meta.ids);
            mam_meta.ids = ids;

            if torrent.meta.uploaded_at.0 == UtcDateTime::UNIX_EPOCH {
                let (_guard, rw) = db.rw_async().await.server_err()?;
                torrent.meta.uploaded_at = mam_meta.uploaded_at;
                rw.upsert(torrent.clone()).server_err()?;
                rw.commit().server_err()?;
            }

            if torrent.meta != mam_meta {
                mam_meta_diff = torrent
                    .meta
                    .diff(&mam_meta)
                    .into_iter()
                    .map(|f| TorrentMetaDiff {
                        field: f.field.to_string(),
                        from: f.from,
                        to: f.to,
                    })
                    .collect();
            }
        }
    }

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

    let mut torrent_info =
        torrent_info_from_meta(&torrent.meta, torrent.id.clone(), torrent.mam_id);
    torrent_info.library_path = torrent.library_path.clone();
    torrent_info.library_files = library_files;
    torrent_info.linker = torrent.linker.clone();
    torrent_info.category = torrent.category.clone();
    torrent_info.client_status = torrent.client_status.as_ref().map(|s| match s {
        mlm_db::ClientStatus::NotInClient => "Not in Client".to_string(),
        mlm_db::ClientStatus::RemovedFromTracker => "Removed from Tracker".to_string(),
    });
    torrent_info.replaced_with = torrent.replaced_with.as_ref().map(|(id, _)| id.clone());

    let mut events_data: Vec<DbEventDto> = db
        .r_transaction()
        .server_err()?
        .scan()
        .secondary(EventKey::torrent_id)
        .server_err()?
        .range(Some(torrent.id.clone())..=Some(torrent.id.clone()))
        .server_err()?
        .map(|event| event.map(map_event))
        .collect::<Result<Vec<_>, _>>()
        .server_err()?;
    events_data.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let abs_item_url = if let Some(abs_cfg) = config.audiobookshelf.as_ref() {
        let abs = Abs::new(abs_cfg).server_err()?;
        abs.get_book(&torrent)
            .await
            .server_err()?
            .map(|book| format!("{}/audiobookshelf/item/{}", abs_cfg.url, book.id))
    } else {
        None
    };

    Ok(super::types::TorrentDetailData {
        torrent: torrent_info,
        events: events_data,
        replacement_torrent: replacement_torrent.map(|replacement| {
            super::types::ReplacementTorrentInfo {
                id: replacement.id,
                title: replacement.meta.title,
                size: replacement.meta.size.to_string(),
                filetypes: replacement.meta.filetypes,
                library_path: replacement.library_path,
            }
        }),
        replacement_missing,
        abs_item_url,
        mam_torrent: mam_torrent.as_ref().map(map_mam_torrent),
        mam_meta_diff,
    })
}

#[server]
pub async fn get_torrent_detail(
    id: String,
) -> Result<super::types::TorrentPageData, ServerFnError> {
    let context = crate::error::get_context()?;

    if context
        .db()
        .r_transaction()
        .server_err()?
        .get()
        .primary::<DbTorrent>(id.clone())
        .server_err()?
        .is_some()
    {
        return get_downloaded_torrent_detail(&context, id)
            .await
            .map(super::types::TorrentPageData::Downloaded);
    }

    if let Ok(mam_id) = id.parse::<u64>()
        && let Ok(mam) = context.mam()
    {
        if let Some(torrent) = context
            .db()
            .r_transaction()
            .server_err()?
            .get()
            .secondary::<DbTorrent>(mlm_db::TorrentKey::mam_id, Some(mam_id))
            .server_err()?
        {
            return get_downloaded_torrent_detail(&context, torrent.id)
                .await
                .map(super::types::TorrentPageData::Downloaded);
        }

        let mam_torrent = mam
            .get_torrent_info_by_id(mam_id)
            .await
            .server_err()?
            .ok_or_server_err("Torrent not found")?;
        let meta = mam_torrent.as_meta().server_err()?;
        return Ok(super::types::TorrentPageData::MamOnly(
            super::types::TorrentMamData {
                mam_torrent: map_mam_torrent(&mam_torrent),
                meta: torrent_info_from_meta(&meta, mam_id.to_string(), Some(mam_id)),
            },
        ));
    }

    Err(ServerFnError::new("Torrent not found"))
}

#[server]
pub async fn select_torrent_action(mam_id: u64, wedge: bool) -> Result<(), ServerFnError> {
    use mlm_db::{SelectedTorrent, Timestamp};

    let context = crate::error::get_context()?;

    let mam = context.mam().server_err()?;
    let torrent = mam
        .get_torrent_info_by_id(mam_id)
        .await
        .server_err()?
        .ok_or_server_err("Torrent not found")?;

    let meta = torrent.as_meta().server_err()?;
    let config = context.config().await;

    let tags: Vec<_> = config
        .tags
        .iter()
        .filter(|t| t.filter.matches(&torrent))
        .collect();
    let category = tags.iter().find_map(|t| t.category.clone());
    let tags: Vec<String> = tags.iter().flat_map(|t| t.tags.clone()).collect();
    let cost = if torrent.vip {
        mlm_db::TorrentCost::Vip
    } else if torrent.personal_freeleech {
        mlm_db::TorrentCost::PersonalFreeleech
    } else if torrent.free {
        mlm_db::TorrentCost::GlobalFreeleech
    } else if wedge {
        mlm_db::TorrentCost::UseWedge
    } else {
        mlm_db::TorrentCost::Ratio
    };

    let (_guard, rw) = context.db().rw_async().await.server_err()?;
    rw.insert(SelectedTorrent {
        mam_id: torrent.id,
        hash: None,
        dl_link: torrent
            .dl
            .clone()
            .ok_or_server_err(&format!("No dl field for torrent {}", torrent.id))?,
        unsat_buffer: None,
        wedge_buffer: None,
        cost,
        category,
        tags,
        title_search: mlm_parse::normalize_title(&meta.title),
        meta,
        grabber: None,
        created_at: Timestamp::now(),
        started_at: None,
        removed_at: None,
    })
    .server_err()?;
    rw.commit().server_err()?;

    if let Some(tx) = &context.triggers.downloader_tx {
        tx.send(()).server_err()?;
    }

    Ok(())
}

#[server]
pub async fn remove_torrent_action(id: String) -> Result<(), ServerFnError> {
    let context = crate::error::get_context()?;

    let torrent = context
        .db()
        .r_transaction()
        .server_err()?
        .get()
        .primary::<DbTorrent>(id.clone())
        .server_err()?
        .ok_or_server_err("Torrent not found")?;

    let (_guard, rw) = context.db().rw_async().await.server_err()?;
    rw.remove(torrent).server_err()?;
    rw.commit().server_err()?;
    Ok(())
}

#[server]
pub async fn clean_torrent_action(id: String) -> Result<(), ServerFnError> {
    use mlm_core::cleaner::clean_torrent;
    let context = crate::error::get_context()?;
    let config = context.config().await;
    let Some(torrent) = context
        .db()
        .r_transaction()
        .server_err()?
        .get()
        .primary::<DbTorrent>(id)
        .server_err()?
    else {
        return Err(ServerFnError::new("Could not find torrent"));
    };
    clean_torrent(&config, context.db(), torrent, true, &context.events)
        .await
        .server_err()?;
    Ok(())
}

#[server]
pub async fn refresh_metadata_action(id: String) -> Result<(), ServerFnError> {
    use mlm_core::linker::refresh_mam_metadata;
    let context = crate::error::get_context()?;
    let config = context.config().await;
    let mam = context.mam().server_err()?;
    refresh_mam_metadata(&config, context.db(), &mam, id, &context.events)
        .await
        .server_err()?;
    Ok(())
}

#[server]
pub async fn relink_torrent_action(id: String) -> Result<(), ServerFnError> {
    use mlm_core::linker::relink;
    let context = crate::error::get_context()?;
    let config = context.config().await;
    relink(&config, context.db(), id, &context.events)
        .await
        .server_err()?;
    Ok(())
}

#[server]
pub async fn refresh_and_relink_action(id: String) -> Result<(), ServerFnError> {
    use mlm_core::linker::refresh_metadata_relink;
    let context = crate::error::get_context()?;
    let config = context.config().await;
    let mam = context.mam().server_err()?;
    refresh_metadata_relink(&config, context.db(), &mam, id, &context.events)
        .await
        .server_err()?;
    Ok(())
}

#[server]
pub async fn match_metadata_action(id: String, provider: String) -> Result<(), ServerFnError> {
    use mlm_db::Event as DbEvent;

    let context = crate::error::get_context()?;
    let Some(mut torrent) = context
        .db()
        .r_transaction()
        .server_err()?
        .get()
        .primary::<DbTorrent>(id.clone())
        .server_err()?
    else {
        return Err(ServerFnError::new("Could not find torrent"));
    };

    let (new_meta, pid, fields) = match_meta(&context, &torrent.meta, &provider)
        .await
        .server_err()?;

    let (_guard, rw) = context.db().rw_async().await.server_err()?;

    let mut meta = new_meta;
    meta.source = mlm_core::MetadataSource::Match;
    torrent.meta = meta;
    torrent.title_search = mlm_parse::normalize_title(&torrent.meta.title);

    rw.upsert(torrent.clone()).server_err()?;
    rw.commit().server_err()?;
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
    let context = crate::error::get_context()?;
    let (_guard, rw) = context.db().rw_async().await.server_err()?;
    let Some(mut torrent) = rw.get().primary::<DbTorrent>(id).server_err()? else {
        return Err(ServerFnError::new("Could not find torrent"));
    };
    torrent.replaced_with.take();
    rw.upsert(torrent).server_err()?;
    rw.commit().server_err()?;
    Ok(())
}

#[server]
pub async fn get_metadata_providers() -> Result<Vec<String>, ServerFnError> {
    let context = crate::error::get_context()?;
    Ok(context.metadata().enabled_providers())
}

#[server]
pub async fn get_other_torrents(id: String) -> Result<Vec<SearchTorrent>, ServerFnError> {
    let context = crate::error::get_context()?;

    if let Some(torrent) = context
        .db()
        .r_transaction()
        .server_err()?
        .get()
        .primary::<DbTorrent>(id.clone())
        .server_err()?
    {
        return other_torrents_data(&context, &torrent.meta).await;
    }

    if let Ok(mam_id) = id.parse::<u64>() {
        if let Some(torrent) = context
            .db()
            .r_transaction()
            .server_err()?
            .get()
            .secondary::<DbTorrent>(mlm_db::TorrentKey::mam_id, Some(mam_id))
            .server_err()?
        {
            return other_torrents_data(&context, &torrent.meta).await;
        }
        if let Ok(mam) = context.mam()
            && let Some(mam_torrent) = mam.get_torrent_info_by_id(mam_id).await.server_err()?
        {
            let meta = mam_torrent.as_meta().server_err()?;
            return other_torrents_data(&context, &meta).await;
        }
    }

    Ok(vec![])
}

#[server]
pub async fn preview_match_metadata(
    id: String,
    provider: String,
) -> Result<Vec<crate::dto::TorrentMetaDiff>, ServerFnError> {
    let context = crate::error::get_context()?;
    let Some(torrent) = context
        .db()
        .r_transaction()
        .server_err()?
        .get()
        .primary::<DbTorrent>(id)
        .server_err()?
    else {
        return Err(ServerFnError::new("Could not find torrent"));
    };

    let (_, _, fields) = match_meta(&context, &torrent.meta, &provider)
        .await
        .server_err()?;

    Ok(fields
        .into_iter()
        .map(|f| crate::dto::TorrentMetaDiff {
            field: f.field.to_string(),
            from: f.from,
            to: f.to,
        })
        .collect())
}

#[server]
pub async fn get_qbit_data(id: String) -> Result<Option<super::types::QbitData>, ServerFnError> {
    use mlm_core::linker::{find_library, library_dir};
    use mlm_core::qbittorrent::get_torrent;

    let context = crate::error::get_context()?;
    let config = context.config().await;
    let db = context.db();

    let Some(torrent) = db
        .r_transaction()
        .server_err()?
        .get()
        .primary::<DbTorrent>(id.clone())
        .server_err()?
    else {
        return Ok(None);
    };

    let Some((qbit_torrent, qbit, _qbit_config)) =
        get_torrent(&config, &torrent.id).await.server_err()?
    else {
        if !config.qbittorrent.is_empty()
            && torrent.client_status != Some(mlm_db::ClientStatus::NotInClient)
        {
            let (_guard, rw) = db.rw_async().await.server_err()?;
            let mut torrent = torrent.clone();
            torrent.client_status = Some(mlm_db::ClientStatus::NotInClient);
            rw.upsert(torrent).server_err()?;
            rw.commit().server_err()?;
        }
        return Ok(None);
    };

    let mut categories: Vec<super::types::QbitCategory> = qbit
        .categories()
        .await
        .server_err()?
        .into_values()
        .map(|cat| super::types::QbitCategory { name: cat.name })
        .collect();
    categories.sort_by(|a, b| a.name.cmp(&b.name));

    let tags: Vec<String> = qbit.tags().await.server_err()?;

    let trackers_raw = qbit.trackers(&torrent.id).await.server_err()?;
    let tracker_message = trackers_raw
        .iter()
        .rev()
        .find_map(|tracker| (!tracker.msg.is_empty()).then(|| tracker.msg.clone()));
    let trackers = trackers_raw
        .into_iter()
        .map(|t| super::types::QbitTracker {
            url: t.url,
            msg: (!t.msg.is_empty()).then_some(t.msg),
        })
        .collect();

    let expected_path = find_library(&config, &qbit_torrent).and_then(|library| {
        library_dir(
            config.exclude_narrator_in_library_dir,
            library,
            &torrent.meta,
        )
    });
    let no_longer_wanted = expected_path.is_none() && torrent.library_path.is_some();
    let wanted_path =
        expected_path.filter(|expected| torrent.library_path.as_ref() != Some(expected));

    let qbit_files = qbit
        .files(&qbit_torrent.hash, None)
        .await
        .server_err()?
        .into_iter()
        .map(|file| file.name)
        .collect();

    let torrent_tags: Vec<String> = qbit_torrent
        .tags
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    Ok(Some(super::types::QbitData {
        torrent_state: format_qbit_state(&qbit_torrent.state),
        torrent_category: qbit_torrent.category.clone(),
        torrent_tags,
        categories,
        tags,
        trackers,
        tracker_message,
        uploaded: mlm_db::Size::from_bytes(qbit_torrent.uploaded as u64).to_string(),
        wanted_path,
        no_longer_wanted,
        qbit_files,
    }))
}

#[server]
pub async fn torrent_start_action(id: String) -> Result<(), ServerFnError> {
    use mlm_core::qbittorrent::get_torrent;

    let context = crate::error::get_context()?;
    let config = context.config().await;
    let Some((qbit_torrent, qbit, _config)) = get_torrent(&config, &id).await.server_err()? else {
        return Err(ServerFnError::new(
            "Torrent not found in qBittorrent".to_string(),
        ));
    };

    qbit.start(vec![&qbit_torrent.hash]).await.server_err()?;

    Ok(())
}

#[server]
pub async fn torrent_stop_action(id: String) -> Result<(), ServerFnError> {
    use mlm_core::qbittorrent::get_torrent;

    let context = crate::error::get_context()?;
    let config = context.config().await;
    let Some((qbit_torrent, qbit, _config)) = get_torrent(&config, &id).await.server_err()? else {
        return Err(ServerFnError::new(
            "Torrent not found in qBittorrent".to_string(),
        ));
    };

    qbit.stop(vec![&qbit_torrent.hash]).await.server_err()?;

    Ok(())
}

#[server]
pub async fn set_qbit_category_tags_action(
    id: String,
    category: String,
    tags: Vec<String>,
) -> Result<(), ServerFnError> {
    use mlm_core::qbittorrent::{ensure_category_exists, get_torrent};

    let context = crate::error::get_context()?;
    let config = context.config().await;
    let Some((qbit_torrent, qbit, qbit_config)) = get_torrent(&config, &id).await.server_err()?
    else {
        return Err(ServerFnError::new(
            "Torrent not found in qBittorrent".to_string(),
        ));
    };

    ensure_category_exists(&qbit, &qbit_config.url, &category)
        .await
        .server_err()?;
    qbit.set_category(Some(vec![&qbit_torrent.hash]), &category)
        .await
        .server_err()?;

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
        .server_err()?;
    }

    if !tags.is_empty() {
        qbit.add_tags(
            Some(vec![&qbit_torrent.hash]),
            tags.iter().map(|s| s.as_str()).collect(),
        )
        .await
        .server_err()?;
    }

    Ok(())
}

#[server]
pub async fn remove_seeding_files_action(id: String) -> Result<(), ServerFnError> {
    use mlm_core::qbittorrent::get_torrent;

    let context = crate::error::get_context()?;
    let config = context.config().await;
    let db = context.db();

    let torrent = db
        .r_transaction()
        .server_err()?
        .get()
        .primary::<DbTorrent>(id.clone())
        .server_err()?
        .ok_or_server_err("Torrent not found")?;

    let Some((qbit_torrent, qbit, _config)) = get_torrent(&config, &id).await.server_err()? else {
        return Err(ServerFnError::new(
            "Torrent not found in qBittorrent".to_string(),
        ));
    };

    let files = qbit.files(&qbit_torrent.hash, None).await.server_err()?;

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
                std::fs::remove_file(path).server_err()?;
            }
        }
    }

    Ok(())
}
