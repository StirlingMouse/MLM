use std::fs;

use anyhow::Result;
use mlm_db::{Torrent, TorrentKey};
use native_db::Database;
use tracing::trace;

use crate::config::Config;
use crate::linker::file_size;

pub fn find_matches(db: &Database<'_>, torrent: &Torrent) -> Result<Vec<Torrent>> {
    let r = db.r_transaction()?;
    let torrents = r.scan().secondary::<Torrent>(TorrentKey::title_search)?;
    let matches = torrents
        .all()?
        .filter_map(|t| t.ok())
        .filter(|t| t.id != torrent.id && t.matches(torrent))
        .collect();
    Ok(matches)
}

pub fn rank_torrents(config: &Config, batch: Vec<Torrent>) -> Vec<Torrent> {
    if batch.len() <= 1 {
        return batch;
    }

    let mut ranked = batch
        .into_iter()
        .map(|torrent| {
            let preferred_types = config.preferred_types(&torrent.meta.media_type);
            let preference = preferred_types
                .iter()
                .position(|t| torrent.meta.filetypes.contains(t))
                .unwrap_or(usize::MAX);
            (torrent, preference)
        })
        .collect::<Vec<_>>();
    ranked.sort_by_key(|(_, preference)| *preference);

    if ranked[0].1 == ranked[1].1 {
        let mut with_size = ranked
            .into_iter()
            .map(|(torrent, preference)| {
                let mut size = 0;
                if let Some(library_path) = &torrent.library_path {
                    for file in &torrent.library_files {
                        let path: std::path::PathBuf = library_path.join(file);
                        size += fs::metadata(path).map_or(0, |s| file_size(&s));
                    }
                }
                if size == 0 {
                    size = torrent.meta.size.bytes();
                }
                (torrent, preference, size)
            })
            .collect::<Vec<_>>();
        with_size.sort_by(|a, b| a.1.cmp(&b.1).then(b.2.cmp(&a.2)));
        trace!("ranked batch by size: {:?}", with_size);
        ranked = with_size
            .into_iter()
            .map(|(torrent, preference, _)| (torrent, preference))
            .collect();
    }
    ranked.into_iter().map(|(t, _)| t).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlm_db::{Language, MainCat, MediaType, MetadataSource, Size, Timestamp, TorrentMeta};
    use std::collections::BTreeMap;

    fn create_test_torrent(
        id: &str,
        title: &str,
        filetypes: Vec<String>,
        size_bytes: u64,
    ) -> Torrent {
        let meta = TorrentMeta {
            title: title.to_string(),
            filetypes,
            size: Size::from_bytes(size_bytes),
            media_type: MediaType::Audiobook,
            main_cat: Some(MainCat::Fiction),
            source: MetadataSource::Mam,
            uploaded_at: Timestamp::now(),
            authors: vec!["Author".to_string()],
            language: Some(Language::English),
            ids: BTreeMap::new(),
            vip_status: None,
            cat: None,
            categories: vec![],
            tags: vec![],
            flags: None,
            num_files: 1,
            edition: None,
            description: "".to_string(),
            narrators: vec![],
            series: vec![],
        };
        Torrent {
            id: id.to_string(),
            id_is_hash: false,
            mam_id: None,
            library_path: None,
            library_files: vec![],
            linker: None,
            category: None,
            selected_audio_format: None,
            selected_ebook_format: None,
            title_search: mlm_parse::normalize_title(title),
            meta,
            created_at: Timestamp::now(),
            replaced_with: None,
            library_mismatch: None,
            client_status: None,
        }
    }

    fn create_test_config() -> Config {
        Config {
            mam_id: "test".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_rank_torrents_preference() {
        let config = create_test_config();

        let t1 = create_test_torrent("1", "Title", vec!["mp3".to_string()], 100);
        let t2 = create_test_torrent("2", "Title", vec!["m4b".to_string()], 100);

        let batch = vec![t1.clone(), t2.clone()];
        let ranked = rank_torrents(&config, batch);

        assert_eq!(ranked[0].id, "2"); // m4b is preferred over mp3
        assert_eq!(ranked[1].id, "1");
    }

    #[test]
    fn test_rank_torrents_size_tie_break() {
        let config = create_test_config();

        let t1 = create_test_torrent("1", "Title", vec!["m4b".to_string()], 100);
        let t2 = create_test_torrent("2", "Title", vec!["m4b".to_string()], 200);

        let batch = vec![t1.clone(), t2.clone()];
        let ranked = rank_torrents(&config, batch);

        assert_eq!(ranked[0].id, "2"); // Larger size wins tie
        assert_eq!(ranked[1].id, "1");
    }

    #[tokio::test]
    async fn test_find_matches() -> Result<()> {
        let tmp_dir =
            std::env::temp_dir().join(format!("mlm_test_duplicates_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir)?;
        let db_path = tmp_dir.join("test.db");

        let db = native_db::Builder::new().create(&mlm_db::MODELS, &db_path)?;
        mlm_db::migrate(&db)?;

        let t1 = create_test_torrent("1", "My Book", vec!["m4b".to_string()], 100);
        let t2 = create_test_torrent("2", "My Book", vec!["mp3".to_string()], 150);
        let t3 = create_test_torrent("3", "Other Book", vec!["m4b".to_string()], 100);

        {
            let rw = db.rw_transaction()?;
            rw.insert(t1.clone())?;
            rw.insert(t2.clone())?;
            rw.insert(t3.clone())?;
            rw.commit()?;
        }

        let matches = find_matches(&db, &t1)?;
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, "2");

        drop(db);
        let _ = fs::remove_dir_all(tmp_dir);
        Ok(())
    }
}
