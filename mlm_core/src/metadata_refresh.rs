use anyhow::{Context as _, Result, bail};
use mlm_db::{Database, Torrent, TorrentKey};
use tracing::{info, warn};

use crate::{
    Config, Events,
    linker::torrent::{MaMApi, refresh_mam_metadata},
};

fn should_refresh_mam_description(torrent: &Torrent) -> bool {
    torrent.mam_id.is_some() && torrent.meta.description.trim().is_empty()
}

#[cfg(test)]
fn collect_refresh_candidate_ids<'a>(
    torrents: impl IntoIterator<Item = &'a Torrent>,
    limit: usize,
) -> Vec<String> {
    torrents
        .into_iter()
        .filter(|torrent| should_refresh_mam_description(torrent))
        .map(|torrent| torrent.id.clone())
        .take(limit)
        .collect()
}

pub(crate) fn missing_description_candidate_ids(
    db: &Database<'_>,
    limit: usize,
) -> Result<Vec<String>> {
    if limit == 0 {
        return Ok(vec![]);
    }

    let r = db.r_transaction()?;
    let mut candidates = Vec::with_capacity(limit);
    for torrent in r.scan().secondary::<Torrent>(TorrentKey::mam_id)?.all()? {
        let torrent = torrent?;
        if should_refresh_mam_description(&torrent) {
            candidates.push(torrent.id);
            if candidates.len() == limit {
                break;
            }
        }
    }

    Ok(candidates)
}

pub async fn refresh_missing_mam_descriptions<M>(
    config: &Config,
    db: &Database<'_>,
    mam: &M,
    limit: usize,
    events: &Events,
) -> Result<()>
where
    M: MaMApi + ?Sized,
{
    let candidate_ids = missing_description_candidate_ids(db, limit)
        .context("finding torrents with mam_id and missing descriptions")?;

    if candidate_ids.is_empty() {
        info!("No torrents with mam_id and missing descriptions need refreshing");
        return Ok(());
    }

    info!(
        "Refreshing MaM metadata for {} torrents with missing descriptions",
        candidate_ids.len()
    );

    let mut refreshed = 0usize;
    let mut failures = Vec::new();
    for torrent_id in candidate_ids {
        match refresh_mam_metadata(config, db, mam, torrent_id.clone(), events).await {
            Ok(_) => refreshed += 1,
            Err(err) => {
                warn!("Failed to refresh metadata for torrent {torrent_id}: {err:#}");
                failures.push(format!("{torrent_id}: {err:#}"));
            }
        }
    }

    if failures.is_empty() {
        info!("Refreshed MaM metadata for {refreshed} torrents");
        Ok(())
    } else {
        bail!(
            "refreshed {refreshed} torrents, but {} refreshes failed:\n{}",
            failures.len(),
            failures.join("\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use mlm_db::{MetadataSource, Timestamp, TorrentMeta};

    use super::*;

    fn torrent(id: &str, mam_id: Option<u64>, description: &str) -> Torrent {
        Torrent {
            id: id.to_string(),
            id_is_hash: true,
            mam_id,
            library_path: None,
            library_files: vec![],
            linker: None,
            category: None,
            selected_audio_format: None,
            selected_ebook_format: None,
            title_search: id.to_string(),
            meta: TorrentMeta {
                ids: BTreeMap::new(),
                title: id.to_string(),
                description: description.to_string(),
                source: MetadataSource::Mam,
                uploaded_at: Some(Timestamp::now()),
                ..Default::default()
            },
            created_at: Timestamp::now(),
            replaced_with: None,
            library_mismatch: None,
            client_status: None,
        }
    }

    #[test]
    fn candidate_selection_filters_missing_descriptions_and_respects_limit() {
        let torrents = vec![
            torrent("first", Some(1), ""),
            torrent("second", Some(2), "   "),
            torrent("third", Some(3), "already present"),
            torrent("fourth", None, ""),
        ];

        assert_eq!(
            collect_refresh_candidate_ids(&torrents, 10),
            vec!["first".to_string(), "second".to_string()]
        );
        assert_eq!(
            collect_refresh_candidate_ids(&torrents, 1),
            vec!["first".to_string()]
        );
    }
}
