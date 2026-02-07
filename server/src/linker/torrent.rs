#[cfg(target_family = "windows")]
use std::os::windows::fs::MetadataExt as _;
use std::{
    fs::File,
    io::{BufWriter, Write},
    mem,
    ops::Deref,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use log::error;
use mlm_db::{
    ClientStatus, DatabaseExt as _, ErroredTorrentId, Event, EventType, LibraryMismatch,
    SelectedTorrent, SelectedTorrentKey, Timestamp, Torrent, TorrentKey, TorrentMeta,
};
use mlm_mam::{api::MaM, meta::MetaError, search::MaMTorrent};
use mlm_parse::normalize_title;
use native_db::Database;
use once_cell::sync::Lazy;
use qbit::{
    models::{Torrent as QbitTorrent, TorrentContent},
    parameters::TorrentListParams,
};

#[allow(async_fn_in_trait)]
pub trait MaMApi: Send + Sync {
    async fn get_torrent_info(&self, hash: &str) -> Result<Option<MaMTorrent>>;
    async fn get_torrent_info_by_id(&self, id: u64) -> Result<Option<MaMTorrent>>;
}

impl MaMApi for MaM<'_> {
    async fn get_torrent_info(&self, hash: &str) -> Result<Option<MaMTorrent>> {
        self.get_torrent_info(hash).await
    }
    async fn get_torrent_info_by_id(&self, id: u64) -> Result<Option<MaMTorrent>> {
        self.get_torrent_info_by_id(id).await
    }
}

impl<T: MaMApi + ?Sized> MaMApi for &T {
    async fn get_torrent_info(&self, hash: &str) -> Result<Option<MaMTorrent>> {
        (**self).get_torrent_info(hash).await
    }
    async fn get_torrent_info_by_id(&self, id: u64) -> Result<Option<MaMTorrent>> {
        (**self).get_torrent_info_by_id(id).await
    }
}

impl<T: MaMApi + ?Sized> MaMApi for Arc<T> {
    async fn get_torrent_info(&self, hash: &str) -> Result<Option<MaMTorrent>> {
        (**self).get_torrent_info(hash).await
    }
    async fn get_torrent_info_by_id(&self, id: u64) -> Result<Option<MaMTorrent>> {
        (**self).get_torrent_info_by_id(id).await
    }
}
use regex::Regex;
use tokio::fs::create_dir_all;
use tracing::{Level, debug, instrument, span, trace};

use crate::{
    audiobookshelf::{self as abs},
    autograbber::update_torrent_meta,
    cleaner::remove_library_files,
    config::{Config, Library, LibraryLinkMethod, QbitConfig},
    linker::{
        common::{copy, hard_link, select_format, symlink},
        library_dir, map_path,
    },
    logging::{TorrentMetaError, update_errored_torrent, write_event},
    qbittorrent::{QbitApi, ensure_category_exists},
};

pub static DISK_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:CD|Disc|Disk)\s*(\d+)").unwrap());

/// Calculates the relative path for a file within the library book directory.
/// Handles `Disc X` subdirectories.
pub fn calculate_library_file_path(torrent_file_path: &str) -> PathBuf {
    let torrent_path = PathBuf::from(torrent_file_path);
    // Split the path into components, find the file name (last component)
    // and search the ancestor directories from nearest to farthest for a
    // "Disc/CD/Disk" pattern. If found, return "Disc N/<filename>",
    // otherwise return just the file name.
    let mut components: Vec<Component> = torrent_path.components().collect();
    let file_name_component = components
        .pop()
        .expect("torrent file path should not be empty");

    // Search ancestors in reverse (nearest directory first)
    let mut disc_dir: Option<String> = None;
    for comp in components.iter().rev() {
        if let Component::Normal(os) = comp {
            let s = os.to_string_lossy();
            if let Some(caps) = DISK_PATTERN.captures(&s)
                && let Some(disc) = caps.get(1)
            {
                disc_dir = Some(format!("Disc {}", disc.as_str()));
                break;
            }
        }
    }

    if let Some(dir_name) = disc_dir {
        PathBuf::from(dir_name).join(file_name_component.as_os_str())
    } else {
        PathBuf::from(file_name_component.as_os_str())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FileLinkPlan {
    pub download_path: PathBuf,
    pub library_path: PathBuf,
    pub relative_library_path: PathBuf,
}

pub fn calculate_link_plans(
    qbit_config: &QbitConfig,
    torrent: &QbitTorrent,
    files: &[TorrentContent],
    selected_audio_format: Option<&str>,
    selected_ebook_format: Option<&str>,
    library_dir: &Path,
) -> Vec<FileLinkPlan> {
    let mut plans = vec![];
    for file in files {
        if !(selected_audio_format
            .as_ref()
            .is_some_and(|ext| file.name.ends_with(*ext))
            || selected_ebook_format
                .as_ref()
                .is_some_and(|ext| file.name.ends_with(*ext)))
        {
            continue;
        }

        let file_path = calculate_library_file_path(&file.name);
        let library_path = library_dir.join(&file_path);
        let download_path =
            map_path(&qbit_config.path_mapping, &torrent.save_path).join(&file.name);

        plans.push(FileLinkPlan {
            download_path,
            library_path,
            relative_library_path: file_path,
        });
    }
    plans
}

pub struct TorrentUpdate {
    pub changed: bool,
    pub events: Vec<Event>,
}

pub fn check_torrent_updates(
    torrent: &mut Torrent,
    qbit_torrent: &QbitTorrent,
    library: Option<&Library>,
    config: &Config,
    trackers: &[qbit::models::Tracker],
) -> TorrentUpdate {
    let mut changed = false;
    let mut events = vec![];

    let library_name = library.and_then(|l| l.options().name.as_ref());
    if torrent.linker.as_ref() != library_name {
        torrent.linker = library_name.map(ToOwned::to_owned);
        changed = true;
    }

    let category = if qbit_torrent.category.is_empty() {
        None
    } else {
        Some(qbit_torrent.category.as_str())
    };
    if torrent.category.as_deref() != category {
        torrent.category = category.map(ToOwned::to_owned);
        changed = true;
    }

    if torrent.client_status.is_none()
        && let Some(mam_tracker) = trackers.last()
        && mam_tracker.msg == "torrent not registered with this tracker"
    {
        torrent.client_status = Some(ClientStatus::RemovedFromTracker);
        changed = true;
        events.push(Event::new(
            Some(qbit_torrent.hash.clone()),
            None,
            EventType::RemovedFromTracker,
        ));
    }

    if let Some(library_path) = &torrent.library_path {
        let Some(library) = library else {
            if torrent.library_mismatch != Some(LibraryMismatch::NoLibrary) {
                torrent.library_mismatch = Some(LibraryMismatch::NoLibrary);
                changed = true;
            }
            return TorrentUpdate { changed, events };
        };

        if !library_path.starts_with(&library.options().library_dir) {
            let wanted = Some(LibraryMismatch::NewLibraryDir(
                library.options().library_dir.clone(),
            ));
            if torrent.library_mismatch != wanted {
                torrent.library_mismatch = wanted;
                changed = true;
            }
        } else {
            let dir = library_dir(
                config.exclude_narrator_in_library_dir,
                library,
                &torrent.meta,
            );
            let mut is_wrong = Some(library_path) != dir.as_ref();
            let wanted = match dir {
                Some(dir) => Some(LibraryMismatch::NewPath(dir)),
                None => Some(LibraryMismatch::NoLibrary),
            };

            if torrent.library_mismatch != wanted {
                if is_wrong {
                    // Try another attempt at matching with exclude_narrator flipped
                    let dir_2 = library_dir(
                        !config.exclude_narrator_in_library_dir,
                        library,
                        &torrent.meta,
                    );
                    if Some(library_path) == dir_2.as_ref() {
                        is_wrong = false
                    }
                }
                if is_wrong {
                    torrent.library_mismatch = wanted;
                    changed = true;
                } else if torrent.library_mismatch.is_some() {
                    torrent.library_mismatch = None;
                    changed = true;
                }
            }
        }
    }

    TorrentUpdate { changed, events }
}

#[instrument(skip_all)]
pub async fn link_torrents_to_library<Q, M>(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    qbit: (&QbitConfig, &Q),
    mam: &M,
) -> Result<()>
where
    Q: QbitApi + ?Sized,
    M: MaMApi + ?Sized,
{
    let torrents = qbit
        .1
        .torrents(Some(TorrentListParams::default()))
        .await
        .context("qbit main data")?;

    for torrent in torrents {
        if torrent.progress < 1.0 {
            continue;
        }
        let library = find_library(&config, &torrent);
        let r = db.r_transaction()?;
        let mut existing_torrent: Option<Torrent> = r.get().primary(torrent.hash.clone())?;
        {
            let selected_torrent: Option<SelectedTorrent> = r.get().secondary::<SelectedTorrent>(
                SelectedTorrentKey::hash,
                Some(torrent.hash.clone()),
            )?;
            if let Some(selected_torrent) = selected_torrent {
                debug!(
                    "Finished Downloading torrent {} {}",
                    selected_torrent.mam_id, selected_torrent.meta.title
                );
                let (_guard, rw) = db.rw_async().await?;
                rw.remove(selected_torrent)?;
                rw.commit()?;
            }
        }
        if let Some(t) = &mut existing_torrent {
            let trackers = if t.client_status.is_none() {
                match qbit.1.trackers(&torrent.hash).await {
                    Ok(trackers) => trackers,
                    Err(err) => {
                        error!("Error getting trackers for torrent {}: {err}", torrent.hash);
                        continue;
                    }
                }
            } else {
                vec![]
            };
            let update = check_torrent_updates(t, &torrent, library, &config, &trackers);
            if update.changed {
                let (_guard, rw) = db.rw_async().await?;
                rw.upsert(t.clone())?;
                rw.commit()?;
            }
            for event in update.events {
                write_event(&db, event).await;
            }

            if t.library_path.is_some() || t.replaced_with.is_some() {
                continue;
            }
        }

        let Some(library) = library else {
            trace!(
                "Could not find matching library for torrent \"{}\", save_path {}",
                torrent.name, torrent.save_path
            );
            continue;
        };

        if library.options().method == LibraryLinkMethod::NoLink && existing_torrent.is_some() {
            continue;
        }

        let result = match_torrent(
            config.clone(),
            db.clone(),
            qbit,
            mam,
            &torrent.hash,
            &torrent,
            library,
            existing_torrent,
        )
        .await
        .context("match_torrent");
        update_errored_torrent(
            &db,
            ErroredTorrentId::Linker(torrent.hash.clone()),
            torrent.name,
            result,
        )
        .await;
    }

    Ok(())
}

pub async fn handle_invalid_torrent<Q>(
    qbit: (&QbitConfig, &Q),
    on_invalid_torrent: &crate::config::QbitUpdate,
    hash: &str,
) -> Result<()>
where
    Q: QbitApi + ?Sized,
{
    if let Some(category) = &on_invalid_torrent.category {
        ensure_category_exists(qbit.1, &qbit.0.url, category).await?;
        qbit.1.set_category(Some(vec![hash]), category).await?;
    }

    if !on_invalid_torrent.tags.is_empty() {
        qbit.1
            .add_tags(
                Some(vec![hash]),
                on_invalid_torrent.tags.iter().map(Deref::deref).collect(),
            )
            .await?;
    }
    Ok(())
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
async fn match_torrent<Q, M>(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    qbit: (&QbitConfig, &Q),
    mam: &M,
    hash: &str,
    torrent: &QbitTorrent,
    library: &Library,
    mut existing_torrent: Option<Torrent>,
) -> Result<()>
where
    Q: QbitApi + ?Sized,
    M: MaMApi + ?Sized,
{
    let files = qbit.1.files(hash, None).await?;
    let selected_audio_format =
        select_format(&library.options().audio_types, &config.audio_types, &files);
    let selected_ebook_format =
        select_format(&library.options().ebook_types, &config.ebook_types, &files);

    if selected_audio_format.is_none() && selected_ebook_format.is_none() {
        bail!("Could not find any wanted formats in torrent");
    }
    let Some(mam_torrent) = mam.get_torrent_info(hash).await.context("get_mam_info")? else {
        if let Some(on_invalid_torrent) = &qbit.0.on_invalid_torrent {
            handle_invalid_torrent(qbit, on_invalid_torrent, hash).await?;
        }
        bail!("Could not find torrent on mam");
    };
    if existing_torrent.is_none()
        && let Some(old_torrent) = db
            .r_transaction()?
            .get()
            .secondary::<Torrent>(TorrentKey::mam_id, Some(mam_torrent.id))?
    {
        if old_torrent.id != hash {
            let (_guard, rw) = db.rw_async().await?;
            rw.remove(old_torrent.clone())?;
            rw.commit()?;
        }
        existing_torrent = Some(old_torrent);
    }
    let mut meta = match mam_torrent.as_meta() {
        Ok(meta) => meta,
        Err(err) => {
            if let MetaError::UnknownMediaType(_) = err {
                if let Some(on_invalid_torrent) = &qbit.0.on_invalid_torrent {
                    handle_invalid_torrent(qbit, on_invalid_torrent, hash).await?;
                }
                trace!("qbit updated");
            }
            return Err(err).context("as_meta");
        }
    };
    if let Some(existing_torrent) = &mut existing_torrent {
        existing_torrent.meta.ids.append(&mut meta.ids);
        mem::swap(&mut meta.ids, &mut existing_torrent.meta.ids);
    }

    link_torrent(
        &config,
        qbit.0,
        &db,
        hash,
        torrent,
        files,
        selected_audio_format,
        selected_ebook_format,
        library,
        existing_torrent.as_ref(),
        &meta,
    )
    .await
    .context("link_torrent")
    .map_err(|err| anyhow::Error::new(TorrentMetaError(meta, err)))
}

#[instrument(skip_all)]
pub async fn refresh_mam_metadata<M>(
    config: &Config,
    db: &Database<'_>,
    mam: &M,
    id: String,
) -> Result<(Torrent, MaMTorrent)>
where
    M: MaMApi + ?Sized,
{
    let Some(mut torrent): Option<Torrent> = db.r_transaction()?.get().primary(id)? else {
        bail!("Could not find torrent id");
    };
    let Some(mam_id) = torrent.meta.mam_id() else {
        bail!("Could not find mam id");
    };
    debug!("refreshing metadata for torrent {}", mam_id);
    let Some(mam_torrent) = mam
        .get_torrent_info_by_id(mam_id)
        .await
        .context("get_mam_info")?
    else {
        bail!("Could not find torrent \"{}\" on mam", torrent.meta.title);
    };
    let mut meta = mam_torrent.as_meta().context("as_meta")?;
    let mut ids = torrent.meta.ids.clone();
    ids.append(&mut meta.ids);
    meta.ids = ids;

    if torrent.meta != meta {
        update_torrent_meta(
            config,
            db,
            db.rw_async().await?,
            &mam_torrent,
            torrent.clone(),
            meta.clone(),
            true,
            false,
        )
        .await?;
        torrent.meta = meta;
    }
    Ok((torrent, mam_torrent))
}

#[instrument(skip_all)]
pub async fn relink(config: &Config, db: &Database<'_>, hash: String) -> Result<()> {
    for qbit_conf in &config.qbittorrent {
        let qbit = match qbit::Api::new_login_username_password(
            &qbit_conf.url,
            &qbit_conf.username,
            &qbit_conf.password,
        )
        .await
        {
            Ok(qbit) => qbit,
            Err(err) => {
                error!("Error logging in to qbit {}: {err}", qbit_conf.url);
                continue;
            }
        };
        let mut torrents = match qbit
            .torrents(Some(TorrentListParams {
                hashes: Some(vec![hash.clone()]),
                ..TorrentListParams::default()
            }))
            .await
        {
            Ok(torrents) => torrents,
            Err(err) => {
                error!("Error getting torrents from qbit {}: {err}", qbit_conf.url);
                continue;
            }
        };
        let Some(qbit_torrent) = torrents.pop() else {
            continue;
        };
        return relink_internal(config, qbit_conf, db, &qbit, qbit_torrent, hash).await;
    }
    bail!("Could not find torrent in qbit");
}

#[instrument(skip_all)]
pub async fn relink_internal<Q>(
    config: &Config,
    qbit_config: &QbitConfig,
    db: &Database<'_>,
    qbit: &Q,
    qbit_torrent: QbitTorrent,
    hash: String,
) -> Result<()>
where
    Q: QbitApi + ?Sized,
{
    let Some(library) = find_library(config, &qbit_torrent) else {
        bail!("Could not find matching library for torrent");
    };
    let files = qbit.files(&hash, None).await?;
    let selected_audio_format =
        select_format(&library.options().audio_types, &config.audio_types, &files);
    let selected_ebook_format =
        select_format(&library.options().ebook_types, &config.ebook_types, &files);

    if selected_audio_format.is_none() && selected_ebook_format.is_none() {
        bail!("Could not find any wanted formats in torrent");
    }
    let Some(torrent) = db.r_transaction()?.get().primary::<Torrent>(hash.clone())? else {
        bail!("Could not find torrent");
    };
    let library_path_changed = torrent.library_path
        != library_dir(
            config.exclude_narrator_in_library_dir,
            library,
            &torrent.meta,
        );
    remove_library_files(config, &torrent, library_path_changed).await?;
    link_torrent(
        config,
        qbit_config,
        db,
        &hash,
        &qbit_torrent,
        files,
        selected_audio_format,
        selected_ebook_format,
        library,
        Some(&torrent),
        &torrent.meta,
    )
    .await
    .context("link_torrent")
    .map_err(|err| anyhow::Error::new(TorrentMetaError(torrent.meta, err)))
}

#[instrument(skip_all)]
pub async fn refresh_metadata_relink<M>(
    config: &Config,
    db: &Database<'_>,
    mam: &M,
    hash: String,
) -> Result<()>
where
    M: MaMApi + ?Sized,
{
    for qbit_conf in &config.qbittorrent {
        let qbit = match qbit::Api::new_login_username_password(
            &qbit_conf.url,
            &qbit_conf.username,
            &qbit_conf.password,
        )
        .await
        {
            Ok(qbit) => qbit,
            Err(err) => {
                error!("Error logging in to qbit {}: {err}", qbit_conf.url);
                continue;
            }
        };
        let mut torrents = match qbit
            .torrents(Some(TorrentListParams {
                hashes: Some(vec![hash.clone()]),
                ..TorrentListParams::default()
            }))
            .await
        {
            Ok(torrents) => torrents,
            Err(err) => {
                error!("Error getting torrents from qbit {}: {err}", qbit_conf.url);
                continue;
            }
        };
        let Some(qbit_torrent) = torrents.pop() else {
            continue;
        };
        return refresh_metadata_relink_internal(
            config,
            qbit_conf,
            db,
            &qbit,
            mam,
            qbit_torrent,
            hash,
        )
        .await;
    }
    bail!("Could not find torrent in qbit");
}

#[instrument(skip_all)]
pub async fn refresh_metadata_relink_internal<Q, M>(
    config: &Config,
    qbit_config: &QbitConfig,
    db: &Database<'_>,
    qbit: &Q,
    mam: &M,
    qbit_torrent: QbitTorrent,
    hash: String,
) -> Result<()>
where
    Q: QbitApi + ?Sized,
    M: MaMApi + ?Sized,
{
    let Some(library) = find_library(config, &qbit_torrent) else {
        bail!("Could not find matching library for torrent");
    };
    let files = qbit.files(&hash, None).await?;
    let selected_audio_format =
        select_format(&library.options().audio_types, &config.audio_types, &files);
    let selected_ebook_format =
        select_format(&library.options().ebook_types, &config.ebook_types, &files);

    if selected_audio_format.is_none() && selected_ebook_format.is_none() {
        bail!("Could not find any wanted formats in torrent");
    }
    let (torrent, _mam_torrent) = refresh_mam_metadata(config, db, mam, hash.clone()).await?;
    let library_path_changed = torrent.library_path
        != library_dir(
            config.exclude_narrator_in_library_dir,
            library,
            &torrent.meta,
        );
    remove_library_files(config, &torrent, library_path_changed).await?;
    link_torrent(
        config,
        qbit_config,
        db,
        &hash,
        &qbit_torrent,
        files,
        selected_audio_format,
        selected_ebook_format,
        library,
        Some(&torrent),
        &torrent.meta,
    )
    .await
    .context("link_torrent")
    .map_err(|err| anyhow::Error::new(TorrentMetaError(torrent.meta, err)))
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
async fn link_torrent(
    config: &Config,
    qbit_config: &QbitConfig,
    db: &Database<'_>,
    hash: &str,
    torrent: &QbitTorrent,
    files: Vec<TorrentContent>,
    selected_audio_format: Option<String>,
    selected_ebook_format: Option<String>,
    library: &Library,
    existing_torrent: Option<&Torrent>,
    meta: &TorrentMeta,
) -> Result<()> {
    let mut library_files = vec![];

    let library_path = if library.options().method != LibraryLinkMethod::NoLink {
        let Some(mut dir) = library_dir(config.exclude_narrator_in_library_dir, library, meta)
        else {
            bail!("Torrent has no author");
        };
        if config.exclude_narrator_in_library_dir && !meta.narrators.is_empty() && dir.exists() {
            dir = library_dir(false, library, meta).unwrap();
        }
        let metadata = abs::create_metadata(meta);

        create_dir_all(&dir).await?;
        let plans = calculate_link_plans(
            qbit_config,
            torrent,
            &files,
            selected_audio_format.as_deref(),
            selected_ebook_format.as_deref(),
            &dir,
        );

        for plan in plans {
            let span = span!(Level::TRACE, "file", file = ?plan.relative_library_path);
            let _s = span.enter();
            if let Some(parent) = plan.library_path.parent() {
                create_dir_all(parent).await?;
            }
            library_files.push(plan.relative_library_path.clone());
            match library.options().method {
                LibraryLinkMethod::Hardlink => hard_link(
                    &plan.download_path,
                    &plan.library_path,
                    &plan.relative_library_path,
                )?,
                LibraryLinkMethod::HardlinkOrCopy => hard_link(
                    &plan.download_path,
                    &plan.library_path,
                    &plan.relative_library_path,
                )
                .or_else(|_| copy(&plan.download_path, &plan.library_path))?,
                LibraryLinkMethod::Copy => copy(&plan.download_path, &plan.library_path)?,
                LibraryLinkMethod::HardlinkOrSymlink => hard_link(
                    &plan.download_path,
                    &plan.library_path,
                    &plan.relative_library_path,
                )
                .or_else(|_| symlink(&plan.download_path, &plan.library_path))?,
                LibraryLinkMethod::Symlink => symlink(&plan.download_path, &plan.library_path)?,
                LibraryLinkMethod::NoLink => {}
            };
        }
        library_files.sort();

        let file = File::create(dir.join("metadata.json"))?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &metadata)?;
        writer.flush()?;
        Some(dir.clone())
    } else {
        None
    };

    {
        let (_guard, rw) = db.rw_async().await?;
        rw.upsert(Torrent {
            id: hash.to_owned(),
            id_is_hash: true,
            mam_id: meta.mam_id(),
            // abs_id: existing_torrent.and_then(|t| t.abs_id.clone()),
            // goodreads_id: existing_torrent.and_then(|t| t.goodreads_id),
            library_path: library_path.clone(),
            library_files,
            linker: library.options().name.clone(),
            category: if torrent.category.is_empty() {
                None
            } else {
                Some(torrent.category.clone())
            },
            selected_audio_format,
            selected_ebook_format,
            title_search: normalize_title(&meta.title),
            meta: meta.clone(),
            created_at: existing_torrent
                .map(|t| t.created_at)
                .unwrap_or_else(Timestamp::now),
            replaced_with: existing_torrent.and_then(|t| t.replaced_with.clone()),
            library_mismatch: None,
            client_status: existing_torrent.and_then(|t| t.client_status.clone()),
        })?;
        rw.commit()?;
    }

    if let Some(library_path) = library_path {
        write_event(
            db,
            Event::new(
                Some(hash.to_owned()),
                meta.mam_id(),
                EventType::Linked {
                    linker: library.options().name.clone(),
                    library_path,
                },
            ),
        )
        .await;
    }

    Ok(())
}

// map_path provided by crate::linker::common::map_path

pub fn find_library<'a>(config: &'a Config, torrent: &QbitTorrent) -> Option<&'a Library> {
    config
        .libraries
        .iter()
        .filter(|l| match l {
            Library::ByRipDir(_) => false,
            Library::ByDownloadDir(l) => {
                PathBuf::from(&torrent.save_path).starts_with(&l.download_dir)
            }
            Library::ByCategory(l) => torrent.category == l.category,
        })
        .find(|l| {
            let filters = l.tag_filters();
            if filters
                .deny_tags
                .iter()
                .any(|tag| torrent.tags.split(", ").any(|t| t == tag.as_str()))
            {
                return false;
            }
            if filters.allow_tags.is_empty() {
                return true;
            }
            filters
                .allow_tags
                .iter()
                .any(|tag| torrent.tags.split(", ").any(|t| t == tag.as_str()))
        })
}

// library_dir provided by crate::linker::common::library_dir

// select_format provided by crate::linker::common::select_format_from_contents as `select_format`

// hard_link, copy, symlink and file_size provided by crate::linker_common

// tests moved to linker_common

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::config::{
        Config, Library, LibraryByCategory, LibraryByDownloadDir, LibraryLinkMethod,
        LibraryOptions, LibraryTagFilters,
    };

    #[test]
    fn test_find_library_by_download_dir() {
        let cfg = Config {
            mam_id: "m".to_string(),
            web_host: "".to_string(),
            web_port: 0,
            min_ratio: 0.0,
            unsat_buffer: 0,
            wedge_buffer: 0,
            add_torrents_stopped: false,
            exclude_narrator_in_library_dir: false,
            search_interval: 0,
            link_interval: 0,
            import_interval: 0,
            ignore_torrents: vec![],
            audio_types: vec![],
            ebook_types: vec![],
            music_types: vec![],
            radio_types: vec![],
            search: crate::config::SearchConfig::default(),
            audiobookshelf: None,
            autograbs: vec![],
            snatchlist: vec![],
            goodreads_lists: vec![],
            notion_lists: vec![],
            tags: vec![],
            qbittorrent: vec![],
            libraries: vec![Library::ByDownloadDir(LibraryByDownloadDir {
                download_dir: PathBuf::from("/downloads"),
                options: LibraryOptions {
                    name: None,
                    library_dir: PathBuf::from("/library"),
                    method: LibraryLinkMethod::Hardlink,
                    audio_types: None,
                    ebook_types: None,
                },
                tag_filters: LibraryTagFilters {
                    allow_tags: vec![],
                    deny_tags: vec![],
                },
            })],
        };

        let qbit_torrent = qbit::models::Torrent {
            save_path: "/downloads/some/path".to_string(),
            category: "".to_string(),
            ..Default::default()
        };
        let lib = find_library(&cfg, &qbit_torrent);
        assert!(lib.is_some());
        match lib.unwrap() {
            Library::ByDownloadDir(_) => {}
            _ => panic!("Expected ByDownloadDir"),
        }
    }

    #[test]
    fn test_find_library_by_category() {
        let cfg = Config {
            mam_id: "m".to_string(),
            web_host: "".to_string(),
            web_port: 0,
            min_ratio: 0.0,
            unsat_buffer: 0,
            wedge_buffer: 0,
            add_torrents_stopped: false,
            exclude_narrator_in_library_dir: false,
            search_interval: 0,
            link_interval: 0,
            import_interval: 0,
            ignore_torrents: vec![],
            audio_types: vec![],
            ebook_types: vec![],
            music_types: vec![],
            radio_types: vec![],
            search: crate::config::SearchConfig::default(),
            audiobookshelf: None,
            autograbs: vec![],
            snatchlist: vec![],
            goodreads_lists: vec![],
            notion_lists: vec![],
            tags: vec![],
            qbittorrent: vec![],
            libraries: vec![Library::ByCategory(LibraryByCategory {
                category: "audiobooks".to_string(),
                options: LibraryOptions {
                    name: None,
                    library_dir: PathBuf::from("/lib2"),
                    method: LibraryLinkMethod::Hardlink,
                    audio_types: None,
                    ebook_types: None,
                },
                tag_filters: LibraryTagFilters {
                    allow_tags: vec![],
                    deny_tags: vec![],
                },
            })],
        };

        let qbit_torrent = qbit::models::Torrent {
            save_path: "/other".to_string(),
            category: "audiobooks".to_string(),
            ..Default::default()
        };
        let lib = find_library(&cfg, &qbit_torrent);
        assert!(lib.is_some());
        match lib.unwrap() {
            Library::ByCategory(l) => assert_eq!(l.category, "audiobooks"),
            _ => panic!("Expected ByCategory"),
        }
    }

    #[test]
    fn test_find_library_skips_rip_dir() {
        let cfg = Config {
            mam_id: "m".to_string(),
            web_host: "".to_string(),
            web_port: 0,
            min_ratio: 0.0,
            unsat_buffer: 0,
            wedge_buffer: 0,
            add_torrents_stopped: false,
            exclude_narrator_in_library_dir: false,
            search_interval: 0,
            link_interval: 0,
            import_interval: 0,
            ignore_torrents: vec![],
            audio_types: vec![],
            ebook_types: vec![],
            music_types: vec![],
            radio_types: vec![],
            search: crate::config::SearchConfig::default(),
            audiobookshelf: None,
            autograbs: vec![],
            snatchlist: vec![],
            goodreads_lists: vec![],
            notion_lists: vec![],
            tags: vec![],
            qbittorrent: vec![],
            libraries: vec![Library::ByRipDir(crate::config::LibraryByRipDir {
                rip_dir: PathBuf::from("/rip"),
                options: LibraryOptions {
                    name: None,
                    library_dir: PathBuf::from("/lib"),
                    method: LibraryLinkMethod::Hardlink,
                    audio_types: None,
                    ebook_types: None,
                },
                filter: crate::config::EditionFilter::default(),
            })],
        };

        let qbit_torrent = qbit::models::Torrent {
            save_path: "/rip/some".to_string(),
            category: "".to_string(),
            ..Default::default()
        };
        let lib = find_library(&cfg, &qbit_torrent);
        // ByRipDir is explicitly skipped by find_library so should return None
        assert!(lib.is_none());
    }

    #[test]
    fn test_calculate_library_file_path() {
        assert_eq!(
            calculate_library_file_path("Book Title/audio.m4b"),
            PathBuf::from("audio.m4b")
        );
        assert_eq!(
            calculate_library_file_path("Book Title/CD 1/01.mp3"),
            PathBuf::from("Disc 1/01.mp3")
        );
        assert_eq!(
            calculate_library_file_path("Book Title/Disc 2/05.mp3"),
            PathBuf::from("Disc 2/05.mp3")
        );
        assert_eq!(
            calculate_library_file_path("Book Title/Disk 10/05.mp3"),
            PathBuf::from("Disc 10/05.mp3")
        );
        assert_eq!(
            calculate_library_file_path("Book Title/CD1/01.mp3"),
            PathBuf::from("Disc 1/01.mp3")
        );
        assert_eq!(
            calculate_library_file_path("audio.m4b"),
            PathBuf::from("audio.m4b")
        );
        assert_eq!(
            calculate_library_file_path("Book Title/Some Other Folder/audio.m4b"),
            PathBuf::from("audio.m4b")
        );
        assert_eq!(
            calculate_library_file_path("Book Title/Disc 1/Subfolder/01.mp3"),
            PathBuf::from("Disc 1/01.mp3")
        );
    }

    #[test]
    fn test_calculate_library_file_path_filename_contains_disc() {
        // Ensure a filename that contains the word "Disc" isn't treated as a
        // disc directory. Only ancestor directory names should be considered.
        assert_eq!(
            calculate_library_file_path("Book Title/Some Folder/Disc 1 - track.mp3"),
            PathBuf::from("Disc 1 - track.mp3")
        );
    }

    #[test]
    fn test_calculate_link_plans() {
        use std::collections::BTreeMap;
        let qbit_config = QbitConfig {
            path_mapping: BTreeMap::from([(
                PathBuf::from("/downloads"),
                PathBuf::from("/data/downloads"),
            )]),
            url: "".to_string(),
            username: "".to_string(),
            password: "".to_string(),
            on_cleaned: None,
            on_invalid_torrent: None,
        };
        let torrent = qbit::models::Torrent {
            save_path: "/downloads".to_string(),
            ..Default::default()
        };
        let files = vec![
            TorrentContent {
                name: "Audiobook/audio.m4b".to_string(),
                ..Default::default()
            },
            TorrentContent {
                name: "Audiobook/cover.jpg".to_string(),
                ..Default::default()
            },
        ];
        let library_dir = PathBuf::from("/library");

        let plans = calculate_link_plans(
            &qbit_config,
            &torrent,
            &files,
            Some(".m4b"),
            None,
            &library_dir,
        );

        assert_eq!(plans.len(), 1);
        assert_eq!(
            plans[0].download_path,
            PathBuf::from("/data/downloads/Audiobook/audio.m4b")
        );
        assert_eq!(plans[0].library_path, PathBuf::from("/library/audio.m4b"));
        assert_eq!(plans[0].relative_library_path, PathBuf::from("audio.m4b"));
    }

    #[test]
    fn test_check_torrent_updates_category_change() {
        use std::collections::BTreeMap;
        let mut torrent = Torrent {
            id: "1".to_string(),
            id_is_hash: true,
            mam_id: None,
            library_path: None,
            library_files: vec![],
            linker: None,
            category: Some("old".to_string()),
            selected_audio_format: None,
            selected_ebook_format: None,
            title_search: "".to_string(),
            meta: TorrentMeta {
                ids: BTreeMap::new(),
                vip_status: None,
                cat: None,
                media_type: mlm_db::MediaType::Audiobook,
                main_cat: None,
                categories: vec![],
                tags: vec![],
                language: None,
                flags: None,
                filetypes: vec![],
                num_files: 0,
                size: mlm_db::Size::from_bytes(0),
                title: "".to_string(),
                edition: None,
                description: "".to_string(),
                authors: vec![],
                narrators: vec![],
                series: vec![],
                source: mlm_db::MetadataSource::Mam,
                uploaded_at: mlm_db::Timestamp::now(),
            },
            created_at: mlm_db::Timestamp::now(),
            replaced_with: None,
            library_mismatch: None,
            client_status: None,
        };
        let qbit_torrent = qbit::models::Torrent {
            category: "new".to_string(),
            ..Default::default()
        };
        let cfg = Config {
            mam_id: "m".to_string(),
            web_host: "".to_string(),
            web_port: 0,
            min_ratio: 0.0,
            unsat_buffer: 0,
            wedge_buffer: 0,
            add_torrents_stopped: false,
            exclude_narrator_in_library_dir: false,
            search_interval: 0,
            link_interval: 0,
            import_interval: 0,
            ignore_torrents: vec![],
            audio_types: vec![],
            ebook_types: vec![],
            music_types: vec![],
            radio_types: vec![],
            search: crate::config::SearchConfig::default(),
            audiobookshelf: None,
            autograbs: vec![],
            snatchlist: vec![],
            goodreads_lists: vec![],
            notion_lists: vec![],
            tags: vec![],
            qbittorrent: vec![],
            libraries: vec![],
        };

        let update = check_torrent_updates(&mut torrent, &qbit_torrent, None, &cfg, &[]);
        assert!(update.changed);
        assert_eq!(torrent.category, Some("new".to_string()));
    }

    #[test]
    fn test_check_torrent_updates_linker_change() {
        let mut torrent = Torrent {
            linker: Some("old_linker".to_string()),
            ..mock_torrent()
        };
        let qbit_torrent = qbit::models::Torrent::default();
        let library = Library::ByCategory(LibraryByCategory {
            category: "audiobooks".to_string(),
            options: LibraryOptions {
                name: Some("new_linker".to_string()),
                library_dir: PathBuf::from("/lib"),
                method: LibraryLinkMethod::Hardlink,
                audio_types: None,
                ebook_types: None,
            },
            tag_filters: LibraryTagFilters::default(),
        });
        let cfg = mock_config_with_library(library.clone());

        let update = check_torrent_updates(&mut torrent, &qbit_torrent, Some(&library), &cfg, &[]);
        assert!(update.changed);
        assert_eq!(torrent.linker, Some("new_linker".to_string()));
    }

    #[test]
    fn test_check_torrent_updates_removed_from_tracker() {
        let mut torrent = mock_torrent();
        let qbit_torrent = qbit::models::Torrent {
            hash: "hash".to_string(),
            ..Default::default()
        };
        let trackers = vec![qbit::models::Tracker {
            msg: "torrent not registered with this tracker".to_string(),
            ..Default::default()
        }];
        let cfg = mock_config();

        let update = check_torrent_updates(&mut torrent, &qbit_torrent, None, &cfg, &trackers);
        assert!(update.changed);
        assert_eq!(
            torrent.client_status,
            Some(ClientStatus::RemovedFromTracker)
        );
        assert_eq!(update.events.len(), 1);
        assert_eq!(update.events[0].event, EventType::RemovedFromTracker);
    }

    #[test]
    fn test_check_torrent_updates_library_mismatch() {
        let mut torrent = mock_torrent();
        torrent.library_path = Some(PathBuf::from("/old_library/Author/Title"));
        torrent.meta.authors = vec!["Author".to_string()];
        torrent.meta.title = "Title".to_string();

        let library = Library::ByDownloadDir(LibraryByDownloadDir {
            download_dir: PathBuf::from("/downloads"),
            options: LibraryOptions {
                name: None,
                library_dir: PathBuf::from("/new_library"),
                method: LibraryLinkMethod::Hardlink,
                audio_types: None,
                ebook_types: None,
            },
            tag_filters: LibraryTagFilters::default(),
        });
        let cfg = mock_config_with_library(library.clone());
        let qbit_torrent = qbit::models::Torrent::default();

        let update = check_torrent_updates(&mut torrent, &qbit_torrent, Some(&library), &cfg, &[]);
        assert!(update.changed);
        assert_eq!(
            torrent.library_mismatch,
            Some(LibraryMismatch::NewLibraryDir(PathBuf::from(
                "/new_library"
            )))
        );

        // Test NewPath (same library dir, different author/title logic)
        torrent.library_mismatch = None;
        torrent.library_path = Some(PathBuf::from("/new_library/OldAuthor/Title"));
        let update = check_torrent_updates(&mut torrent, &qbit_torrent, Some(&library), &cfg, &[]);
        assert!(update.changed);
        if let Some(LibraryMismatch::NewPath(p)) = &torrent.library_mismatch {
            assert!(p.ends_with("Author/Title"));
        } else {
            panic!(
                "Expected NewPath mismatch, got {:?}",
                torrent.library_mismatch
            );
        }
    }

    #[test]
    fn test_check_torrent_updates_exclude_narrator() {
        let mut torrent = mock_torrent();
        torrent.meta.authors = vec!["Author".to_string()];
        torrent.meta.narrators = vec!["Narrator".to_string()];
        torrent.meta.title = "Title".to_string();

        // Path with narrator
        let path_with_narrator = PathBuf::from("/library/Author/Title {Narrator}");
        // Path without narrator
        let path_without_narrator = PathBuf::from("/library/Author/Title");

        let library = Library::ByDownloadDir(LibraryByDownloadDir {
            download_dir: PathBuf::from("/downloads"),
            options: LibraryOptions {
                name: None,
                library_dir: PathBuf::from("/library"),
                method: LibraryLinkMethod::Hardlink,
                audio_types: None,
                ebook_types: None,
            },
            tag_filters: LibraryTagFilters::default(),
        });

        // Config: exclude_narrator = true
        let mut cfg = mock_config_with_library(library.clone());
        cfg.exclude_narrator_in_library_dir = true;

        let qbit_torrent = qbit::models::Torrent::default();

        // If current path matches "with narrator" but config says exclude, it might be okay if it was deliberate or it might trigger mismatch.
        // The code has this logic:
        /*
            let dir = library_dir(config.exclude_narrator_in_library_dir, library, &torrent.meta);
            let mut is_wrong = Some(library_path) != dir.as_ref();
            ...
            if is_wrong {
                // Try another attempt at matching with exclude_narrator flipped
                let dir_2 = library_dir(!config.exclude_narrator_in_library_dir, library, &torrent.meta);
                if Some(library_path) == dir_2.as_ref() {
                    is_wrong = false
                }
            }
        */
        // This means it accepts BOTH paths as "not wrong" if they match either state of the toggle, UNLESS it's already in mismatch state?
        // Wait, if it matches dir_2, is_wrong becomes false, so it WON'T set library_mismatch.

        torrent.library_path = Some(path_with_narrator.clone());
        let update = check_torrent_updates(&mut torrent, &qbit_torrent, Some(&library), &cfg, &[]);
        assert!(
            !update.changed,
            "Should accept existing path with narrator even if config says exclude"
        );

        torrent.library_path = Some(path_without_narrator.clone());
        let update = check_torrent_updates(&mut torrent, &qbit_torrent, Some(&library), &cfg, &[]);
        assert!(
            !update.changed,
            "Should accept existing path without narrator"
        );

        // What if it matches neither?
        torrent.library_path = Some(PathBuf::from("/library/Wrong/Path"));
        let update = check_torrent_updates(&mut torrent, &qbit_torrent, Some(&library), &cfg, &[]);
        assert!(update.changed);
        assert!(matches!(
            torrent.library_mismatch,
            Some(LibraryMismatch::NewPath(_))
        ));
    }

    #[tokio::test]
    async fn test_handle_invalid_torrent() {
        struct MockQbit {
            category: std::sync::Mutex<Option<String>>,
            tags: std::sync::Mutex<Vec<String>>,
        }
        impl QbitApi for MockQbit {
            async fn torrents(
                &self,
                _: Option<TorrentListParams>,
            ) -> Result<Vec<qbit::models::Torrent>> {
                Ok(vec![])
            }
            async fn trackers(&self, _: &str) -> Result<Vec<qbit::models::Tracker>> {
                Ok(vec![])
            }
            async fn files(
                &self,
                _: &str,
                _: Option<Vec<i64>>,
            ) -> Result<Vec<qbit::models::TorrentContent>> {
                Ok(vec![])
            }
            async fn set_category(&self, _: Option<Vec<&str>>, category: &str) -> Result<()> {
                *self.category.lock().unwrap() = Some(category.to_string());
                Ok(())
            }
            async fn add_tags(&self, _: Option<Vec<&str>>, tags: Vec<&str>) -> Result<()> {
                self.tags
                    .lock()
                    .unwrap()
                    .extend(tags.iter().map(|t| t.to_string()));
                Ok(())
            }
            async fn create_category(&self, _: &str, _: &str) -> Result<()> {
                Ok(())
            }
            async fn categories(
                &self,
            ) -> Result<std::collections::HashMap<String, qbit::models::Category>> {
                Ok(std::collections::HashMap::new())
            }
        }

        let qbit = MockQbit {
            category: std::sync::Mutex::new(None),
            tags: std::sync::Mutex::new(vec![]),
        };

        let qbit_conf = QbitConfig {
            url: "http://localhost:8080".to_string(),
            ..Default::default()
        };

        let update = crate::config::QbitUpdate {
            category: Some("invalid".to_string()),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };

        handle_invalid_torrent((&qbit_conf, &qbit), &update, "hash")
            .await
            .unwrap();

        assert_eq!(*qbit.category.lock().unwrap(), Some("invalid".to_string()));
        assert_eq!(
            *qbit.tags.lock().unwrap(),
            vec!["tag1".to_string(), "tag2".to_string()]
        );
    }

    fn mock_torrent() -> Torrent {
        use std::collections::BTreeMap;
        Torrent {
            id: "1".to_string(),
            id_is_hash: true,
            mam_id: None,
            library_path: None,
            library_files: vec![],
            linker: None,
            category: None,
            selected_audio_format: None,
            selected_ebook_format: None,
            title_search: "".to_string(),
            meta: TorrentMeta {
                ids: BTreeMap::new(),
                vip_status: None,
                cat: None,
                media_type: mlm_db::MediaType::Audiobook,
                main_cat: None,
                categories: vec![],
                tags: vec![],
                language: None,
                flags: None,
                filetypes: vec![],
                num_files: 0,
                size: mlm_db::Size::from_bytes(0),
                title: "".to_string(),
                edition: None,
                description: "".to_string(),
                authors: vec![],
                narrators: vec![],
                series: vec![],
                source: mlm_db::MetadataSource::Mam,
                uploaded_at: mlm_db::Timestamp::now(),
            },
            created_at: mlm_db::Timestamp::now(),
            replaced_with: None,
            library_mismatch: None,
            client_status: None,
        }
    }

    fn mock_config() -> Config {
        Config {
            mam_id: "m".to_string(),
            web_host: "".to_string(),
            web_port: 0,
            min_ratio: 0.0,
            unsat_buffer: 0,
            wedge_buffer: 0,
            add_torrents_stopped: false,
            exclude_narrator_in_library_dir: false,
            search_interval: 0,
            link_interval: 0,
            import_interval: 0,
            ignore_torrents: vec![],
            audio_types: vec![],
            ebook_types: vec![],
            music_types: vec![],
            radio_types: vec![],
            search: crate::config::SearchConfig::default(),
            audiobookshelf: None,
            autograbs: vec![],
            snatchlist: vec![],
            goodreads_lists: vec![],
            notion_lists: vec![],
            tags: vec![],
            qbittorrent: vec![],
            libraries: vec![],
        }
    }

    fn mock_config_with_library(library: Library) -> Config {
        let mut cfg = mock_config();
        cfg.libraries.push(library);
        cfg
    }
}
