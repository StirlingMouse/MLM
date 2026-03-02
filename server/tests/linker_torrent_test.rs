mod common;

use anyhow::Result;
use common::{MockFs, TestDb, mock_config};
use mlm_core::config::{
    Library, LibraryByDownloadDir, LibraryLinkMethod, LibraryOptions, LibraryTagFilters, QbitConfig,
};
use mlm_core::linker::torrent::{MaMApi, link_torrents_to_library};
use mlm_core::qbittorrent::QbitApi;
use mlm_db::DatabaseExt as _;
use mlm_mam::search::MaMTorrent;
use qbit::models::{Torrent as QbitTorrent, TorrentContent, Tracker};
use qbit::parameters::TorrentListParams;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

struct MockQbit {
    torrents: Vec<QbitTorrent>,
    files: HashMap<String, Vec<TorrentContent>>,
}

impl QbitApi for MockQbit {
    async fn torrents(&self, _params: Option<TorrentListParams>) -> Result<Vec<QbitTorrent>> {
        Ok(self.torrents.clone())
    }
    async fn trackers(&self, _hash: &str) -> Result<Vec<Tracker>> {
        Ok(vec![])
    }
    async fn files(&self, hash: &str, _params: Option<Vec<i64>>) -> Result<Vec<TorrentContent>> {
        Ok(self.files.get(hash).cloned().unwrap_or_default())
    }
    async fn set_category(&self, _hashes: Option<Vec<&str>>, _category: &str) -> Result<()> {
        Ok(())
    }
    async fn add_tags(&self, _hashes: Option<Vec<&str>>, _tags: Vec<&str>) -> Result<()> {
        Ok(())
    }
    async fn create_category(&self, _category: &str, _save_path: &str) -> Result<()> {
        Ok(())
    }
    async fn categories(&self) -> Result<HashMap<String, qbit::models::Category>> {
        Ok(HashMap::new())
    }
}

struct MockMaM {
    torrents: HashMap<String, MaMTorrent>,
}

impl MaMApi for MockMaM {
    async fn get_torrent_info(&self, hash: &str) -> Result<Option<MaMTorrent>> {
        Ok(self.torrents.get(hash).cloned())
    }
    async fn get_torrent_info_by_id(&self, id: u64) -> Result<Option<MaMTorrent>> {
        Ok(self.torrents.values().find(|t| t.id == id).cloned())
    }
}

#[allow(clippy::too_many_arguments)]
/// Helper to build a MaMTorrent with sensible defaults for tests.
fn make_mam_torrent(
    id: u64,
    title: &str,
    mediatype: u8,
    maincat: u8,
    category: u64,
    catname: &str,
    language: u8,
    lang_code: &str,
    numfiles: u64,
    filetype: &str,
) -> MaMTorrent {
    MaMTorrent {
        id,
        title: title.to_string(),
        added: "2024-01-01 12:00:00".to_string(),
        size: format!("{} B", 100),
        mediatype,
        maincat,
        catname: catname.to_string(),
        category,
        language,
        lang_code: lang_code.to_string(),
        numfiles,
        filetype: filetype.to_string(),
        ..Default::default()
    }
}

#[tokio::test]
async fn test_link_torrent_audiobook() -> anyhow::Result<()> {
    // Setup test DB and filesystem
    let db = TestDb::new()?;
    let fs = MockFs::new()?;

    // Create a mock torrent directory in rip folder
    let torrent_hash = "1234567890abcdef1234567890abcdef12345678";
    let torrent_name = "Test Audiobook";
    let torrent_dir = fs.rip_dir.join(torrent_name);
    std::fs::create_dir_all(&torrent_dir)?;
    std::fs::write(torrent_dir.join("audio.m4b"), "fake audio content")?;

    let mut config = mock_config(fs.rip_dir.clone(), fs.library_dir.clone());
    config.qbittorrent.push(QbitConfig {
        url: "http://localhost:8080".to_string(),
        username: "admin".to_string(),
        password: "adminadmin".to_string(),
        path_mapping: BTreeMap::new(),
        on_cleaned: None,
        on_invalid_torrent: None,
    });
    config.libraries = vec![Library::ByDownloadDir(LibraryByDownloadDir {
        download_dir: fs.rip_dir.clone(),
        options: LibraryOptions {
            name: Some("test_library".to_string()),
            library_dir: fs.library_dir.clone(),
            method: LibraryLinkMethod::Hardlink,
            audio_types: None,
            ebook_types: None,
        },
        tag_filters: LibraryTagFilters::default(),
    })];
    let config = Arc::new(config);
    let events = mlm_core::Events::new();

    // Setup mock Qbit
    let qbit_torrent = QbitTorrent {
        hash: torrent_hash.to_string(),
        name: torrent_name.to_string(),
        save_path: fs.rip_dir.to_string_lossy().to_string(),
        progress: 1.0,
        ..Default::default()
    };
    let qbit_content = TorrentContent {
        name: format!("{}/audio.m4b", torrent_name),
        size: 100,
        ..Default::default()
    };
    let mock_qbit = MockQbit {
        torrents: vec![qbit_torrent],
        files: HashMap::from([(torrent_hash.to_string(), vec![qbit_content])]),
    };

    // Setup mock MaM
    let mut mam_torrent = make_mam_torrent(
        1,
        "Test Title",
        1,
        1,
        42,
        "General Fiction",
        1,
        "en",
        1,
        "m4b",
    );
    mam_torrent.author_info.insert(1, "Test Author".to_string());

    let mock_mam = MockMaM {
        torrents: HashMap::from([(torrent_hash.to_string(), mam_torrent)]),
    };

    // Run the linker
    let qbit_config = config.qbittorrent.first().unwrap();
    link_torrents_to_library(
        config.clone(),
        db.db.clone(),
        (qbit_config, &mock_qbit),
        &mock_mam,
        &events,
    )
    .await?;

    // Verify files in library
    let library_book_dir = fs.library_dir.join("Test Author").join("Test Title");
    assert!(
        library_book_dir.exists(),
        "Library book directory should exist at {:?}",
        library_book_dir
    );
    assert!(
        library_book_dir.join("audio.m4b").exists(),
        "Audio file should exist in library"
    );
    assert!(
        library_book_dir.join("metadata.json").exists(),
        "Metadata file should exist in library"
    );

    // Verify DB entry
    let r = db.db.r_transaction()?;
    let torrent: Option<mlm_db::Torrent> = r.get().primary(torrent_hash.to_string())?;
    assert!(torrent.is_some(), "Torrent should be in DB");
    let torrent = torrent.unwrap();
    assert_eq!(torrent.meta.title, "Test Title");
    assert_eq!(torrent.meta.authors, vec!["Test Author"]);

    Ok(())
}

#[tokio::test]
async fn test_skip_incomplete_torrent() -> anyhow::Result<()> {
    let db = TestDb::new()?;
    let fs = MockFs::new()?;
    let mut config = mock_config(fs.rip_dir.clone(), fs.library_dir.clone());
    config.qbittorrent.push(QbitConfig {
        url: "http://localhost:8080".to_string(),
        username: "admin".to_string(),
        password: "adminadmin".to_string(),
        path_mapping: BTreeMap::new(),
        on_cleaned: None,
        on_invalid_torrent: None,
    });
    let config = Arc::new(config);
    let events = mlm_core::Events::new();

    let mock_qbit = MockQbit {
        torrents: vec![QbitTorrent {
            hash: "incomplete".to_string(),
            progress: 0.5,
            ..Default::default()
        }],
        files: HashMap::new(),
    };
    let mock_mam = MockMaM {
        torrents: HashMap::new(),
    };

    let qbit_config = config.qbittorrent.first().unwrap();
    link_torrents_to_library(
        config.clone(),
        db.db.clone(),
        (qbit_config, &mock_qbit),
        &mock_mam,
        &events,
    )
    .await?;

    let r = db.db.r_transaction()?;
    let torrents: Vec<mlm_db::Torrent> = r
        .scan()
        .primary::<mlm_db::Torrent>()?
        .all()?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    assert!(torrents.is_empty(), "Incomplete torrent should be skipped");
    Ok(())
}

#[tokio::test]
async fn test_remove_selected_torrent() -> anyhow::Result<()> {
    let db = TestDb::new()?;
    let fs = MockFs::new()?;

    let torrent_hash = "1234567890abcdef1234567890abcdef12345678";

    // Add SelectedTorrent to DB
    {
        let (_guard, rw) = db.db.rw_async().await?;
        rw.insert(mlm_db::SelectedTorrent {
            mam_id: 1,
            hash: Some(torrent_hash.to_string()),
            dl_link: "http://example.com/dl".to_string(),
            unsat_buffer: None,
            wedge_buffer: None,
            cost: mlm_db::TorrentCost::GlobalFreeleech,
            category: None,
            tags: vec![],
            title_search: "test title".to_string(),
            meta: mlm_db::TorrentMeta {
                ids: BTreeMap::from([(mlm_db::ids::MAM.to_string(), "1".to_string())]),
                media_type: mlm_db::MediaType::Audiobook,
                title: "Test Title".to_string(),
                authors: vec!["Test Author".to_string()],
                source: mlm_db::MetadataSource::Mam,
                uploaded_at: mlm_db::Timestamp::now(),
                ..Default::default()
            },
            grabber: None,
            created_at: mlm_db::Timestamp::now(),
            started_at: None,
            removed_at: None,
        })?;
        rw.commit()?;
    }

    let mut config = mock_config(fs.rip_dir.clone(), fs.library_dir.clone());
    config.qbittorrent.push(QbitConfig {
        url: "".to_string(),
        username: "".to_string(),
        password: "".to_string(),
        path_mapping: BTreeMap::new(),
        on_cleaned: None,
        on_invalid_torrent: None,
    });
    config.libraries = vec![Library::ByDownloadDir(LibraryByDownloadDir {
        download_dir: fs.rip_dir.clone(),
        options: LibraryOptions {
            name: Some("test".to_string()),
            library_dir: fs.library_dir.clone(),
            method: LibraryLinkMethod::NoLink,
            audio_types: None,
            ebook_types: None,
        },
        tag_filters: LibraryTagFilters::default(),
    })];
    let config = Arc::new(config);
    let events = mlm_core::Events::new();

    let mock_qbit = MockQbit {
        torrents: vec![QbitTorrent {
            hash: torrent_hash.to_string(),
            progress: 1.0,
            save_path: fs.rip_dir.to_string_lossy().to_string(),
            ..Default::default()
        }],
        files: HashMap::from([(torrent_hash.to_string(), vec![])]),
    };
    let mock_mam = MockMaM {
        torrents: HashMap::new(),
    };

    let qbit_config = config.qbittorrent.first().unwrap();
    let _ = link_torrents_to_library(
        config.clone(),
        db.db.clone(),
        (qbit_config, &mock_qbit),
        &mock_mam,
        &events,
    )
    .await;

    let r = db.db.r_transaction()?;
    let selected: Option<mlm_db::SelectedTorrent> = r.get().primary(1u64)?;
    assert!(
        selected.is_none(),
        "SelectedTorrent should be removed from DB"
    );
    Ok(())
}

#[tokio::test]
async fn test_link_torrent_ebook() -> anyhow::Result<()> {
    let db = TestDb::new()?;
    let fs = MockFs::new()?;

    let torrent_hash = "abcdef1234567890abcdef1234567890abcdef12";
    let torrent_name = "Test Ebook";
    let torrent_dir = fs.rip_dir.join(torrent_name);
    std::fs::create_dir_all(&torrent_dir)?;
    std::fs::write(torrent_dir.join("book.epub"), "fake epub content")?;

    let mut config = mock_config(fs.rip_dir.clone(), fs.library_dir.clone());
    config.qbittorrent.push(QbitConfig {
        url: "http://localhost:8080".to_string(),
        username: "admin".to_string(),
        password: "adminadmin".to_string(),
        path_mapping: BTreeMap::new(),
        on_cleaned: None,
        on_invalid_torrent: None,
    });
    config.libraries = vec![Library::ByDownloadDir(LibraryByDownloadDir {
        download_dir: fs.rip_dir.clone(),
        options: LibraryOptions {
            name: Some("test_library".to_string()),
            library_dir: fs.library_dir.clone(),
            method: LibraryLinkMethod::Hardlink,
            audio_types: None,
            ebook_types: None,
        },
        tag_filters: LibraryTagFilters::default(),
    })];
    let config = Arc::new(config);
    let events = mlm_core::Events::new();

    let qbit_torrent = QbitTorrent {
        hash: torrent_hash.to_string(),
        name: torrent_name.to_string(),
        save_path: fs.rip_dir.to_string_lossy().to_string(),
        progress: 1.0,
        ..Default::default()
    };
    let qbit_content = TorrentContent {
        name: format!("{}/book.epub", torrent_name),
        size: 200,
        ..Default::default()
    };
    let mock_qbit = MockQbit {
        torrents: vec![qbit_torrent],
        files: HashMap::from([(torrent_hash.to_string(), vec![qbit_content])]),
    };

    let mut mam_torrent = make_mam_torrent(
        2,
        "Ebook Title",
        2,
        2,
        64,
        "General Fiction",
        1,
        "en",
        1,
        "epub",
    );
    mam_torrent
        .author_info
        .insert(2, "Ebook Author".to_string());

    let mock_mam = MockMaM {
        torrents: HashMap::from([(torrent_hash.to_string(), mam_torrent)]),
    };

    let qbit_config = config.qbittorrent.first().unwrap();
    link_torrents_to_library(
        config.clone(),
        db.db.clone(),
        (qbit_config, &mock_qbit),
        &mock_mam,
        &events,
    )
    .await?;

    let library_book_dir = fs.library_dir.join("Ebook Author").join("Ebook Title");
    assert!(library_book_dir.exists());
    assert!(library_book_dir.join("book.epub").exists());
    assert!(library_book_dir.join("metadata.json").exists());

    Ok(())
}

#[tokio::test]
async fn test_relink() -> anyhow::Result<()> {
    let db = TestDb::new()?;
    let fs = MockFs::new()?;

    let torrent_hash = "relink_hash";
    let torrent_name = "Relink Torrent";
    let torrent_dir = fs.rip_dir.join(torrent_name);
    std::fs::create_dir_all(&torrent_dir)?;
    std::fs::write(torrent_dir.join("audio.m4b"), "audio")?;

    let mut config = mock_config(fs.rip_dir.clone(), fs.library_dir.clone());
    let qbit_config = QbitConfig {
        url: "".to_string(),
        username: "".to_string(),
        password: "".to_string(),
        path_mapping: BTreeMap::new(),
        on_cleaned: None,
        on_invalid_torrent: None,
    };
    config.qbittorrent.push(qbit_config.clone());
    config.libraries = vec![Library::ByDownloadDir(LibraryByDownloadDir {
        download_dir: fs.rip_dir.clone(),
        options: LibraryOptions {
            name: Some("test".to_string()),
            library_dir: fs.library_dir.clone(),
            method: LibraryLinkMethod::Hardlink,
            audio_types: None,
            ebook_types: None,
        },
        tag_filters: LibraryTagFilters::default(),
    })];
    let config = Arc::new(config);
    let events = mlm_core::Events::new();

    let old_library_path = fs.library_dir.join("Old Author").join("Title");
    std::fs::create_dir_all(&old_library_path)?;
    std::fs::write(old_library_path.join("audio.m4b"), "old audio")?;

    {
        let (_guard, rw) = db.db.rw_async().await?;
        rw.insert(mlm_db::Torrent {
            id: torrent_hash.to_string(),
            id_is_hash: true,
            mam_id: Some(1),
            library_path: Some(old_library_path.clone()),
            library_files: vec![std::path::PathBuf::from("audio.m4b")],
            linker: Some("test".to_string()),
            category: None,
            selected_audio_format: Some(".m4b".to_string()),
            selected_ebook_format: None,
            title_search: "title".to_string(),
            meta: mlm_db::TorrentMeta {
                ids: BTreeMap::from([(mlm_db::ids::MAM.to_string(), "1".to_string())]),
                title: "Title".to_string(),
                authors: vec!["New Author".to_string()],
                media_type: mlm_db::MediaType::Audiobook,
                source: mlm_db::MetadataSource::Mam,
                uploaded_at: mlm_db::Timestamp::now(),
                ..Default::default()
            },
            created_at: mlm_db::Timestamp::now(),
            replaced_with: None,
            library_mismatch: None,
            client_status: None,
        })?;
        rw.commit()?;
    }

    let mock_qbit = MockQbit {
        torrents: vec![],
        files: HashMap::from([(
            torrent_hash.to_string(),
            vec![TorrentContent {
                name: format!("{}/audio.m4b", torrent_name),
                ..Default::default()
            }],
        )]),
    };

    let qbit_torrent = QbitTorrent {
        hash: torrent_hash.to_string(),
        save_path: fs.rip_dir.to_string_lossy().to_string(),
        ..Default::default()
    };

    mlm_core::linker::torrent::relink_internal(
        &config,
        &qbit_config,
        &db.db,
        &mock_qbit,
        qbit_torrent,
        torrent_hash.to_string(),
        &events,
    )
    .await?;

    assert!(!old_library_path.exists());
    let new_library_path = fs.library_dir.join("New Author").join("Title");
    assert!(new_library_path.exists());
    assert!(new_library_path.join("audio.m4b").exists());

    let r = db.db.r_transaction()?;
    let torrent: mlm_db::Torrent = r.get().primary(torrent_hash.to_string())?.unwrap();
    assert_eq!(torrent.library_path, Some(new_library_path));

    Ok(())
}

#[tokio::test]
async fn test_refresh_metadata_relink() -> anyhow::Result<()> {
    let db = TestDb::new()?;
    let fs = MockFs::new()?;

    let torrent_hash = "refresh_relink_hash";
    let torrent_name = "Refresh Relink Torrent";
    let torrent_dir = fs.rip_dir.join(torrent_name);
    std::fs::create_dir_all(&torrent_dir)?;
    std::fs::write(torrent_dir.join("audio.m4b"), "audio")?;

    let mut config = mock_config(fs.rip_dir.clone(), fs.library_dir.clone());
    let qbit_config = QbitConfig {
        url: "".to_string(),
        username: "".to_string(),
        password: "".to_string(),
        path_mapping: BTreeMap::new(),
        on_cleaned: None,
        on_invalid_torrent: None,
    };
    config.qbittorrent.push(qbit_config.clone());
    config.libraries = vec![Library::ByDownloadDir(LibraryByDownloadDir {
        download_dir: fs.rip_dir.clone(),
        options: LibraryOptions {
            name: Some("test".to_string()),
            library_dir: fs.library_dir.clone(),
            method: LibraryLinkMethod::Hardlink,
            audio_types: None,
            ebook_types: None,
        },
        tag_filters: LibraryTagFilters::default(),
    })];
    let config = Arc::new(config);
    let events = mlm_core::Events::new();

    {
        let (_guard, rw) = db.db.rw_async().await?;
        rw.insert(mlm_db::Torrent {
            id: torrent_hash.to_string(),
            id_is_hash: true,
            mam_id: Some(2),
            library_path: Some(fs.library_dir.join("Old Author").join("Title")),
            library_files: vec![std::path::PathBuf::from("audio.m4b")],
            linker: Some("test".to_string()),
            category: None,
            selected_audio_format: Some(".m4b".to_string()),
            selected_ebook_format: None,
            title_search: "title".to_string(),
            meta: mlm_db::TorrentMeta {
                ids: BTreeMap::from([(mlm_db::ids::MAM.to_string(), "2".to_string())]),
                title: "Title".to_string(),
                authors: vec!["Old Author".to_string()],
                media_type: mlm_db::MediaType::Audiobook,
                source: mlm_db::MetadataSource::Mam,
                uploaded_at: mlm_db::Timestamp::now(),
                ..Default::default()
            },
            created_at: mlm_db::Timestamp::now(),
            replaced_with: None,
            library_mismatch: None,
            client_status: None,
        })?;
        rw.commit()?;
    }

    let mock_qbit = MockQbit {
        torrents: vec![],
        files: HashMap::from([(
            torrent_hash.to_string(),
            vec![TorrentContent {
                name: format!("{}/audio.m4b", torrent_name),
                ..Default::default()
            }],
        )]),
    };

    let mut mam_torrent =
        make_mam_torrent(2, "Title", 1, 1, 42, "General Fiction", 1, "en", 1, "m4b");
    mam_torrent
        .author_info
        .insert(2, "Refreshed Author".to_string());

    let mock_mam = MockMaM {
        torrents: HashMap::from([(torrent_hash.to_string(), mam_torrent)]),
    };

    let qbit_torrent = QbitTorrent {
        hash: torrent_hash.to_string(),
        save_path: fs.rip_dir.to_string_lossy().to_string(),
        ..Default::default()
    };

    mlm_core::linker::torrent::refresh_metadata_relink_internal(
        &config,
        &qbit_config,
        &db.db,
        &mock_qbit,
        &mock_mam,
        qbit_torrent,
        torrent_hash.to_string(),
        &events,
    )
    .await?;

    let new_library_path = fs.library_dir.join("Refreshed Author").join("Title");
    assert!(new_library_path.exists());
    assert!(new_library_path.join("audio.m4b").exists());

    let r = db.db.r_transaction()?;
    let torrent: mlm_db::Torrent = r.get().primary(torrent_hash.to_string())?.unwrap();
    assert_eq!(torrent.meta.authors, vec!["Refreshed Author"]);
    assert_eq!(torrent.library_path, Some(new_library_path));

    Ok(())
}
