mod common;

use common::{MockFs, TestDb, mock_config};
use mlm_core::linker::folder::link_folders_to_library;
use mlm_db::{DatabaseExt, Torrent};
use std::{fs, sync::Arc};

#[tokio::test]
async fn test_link_folders_to_library() -> anyhow::Result<()> {
    let test_db = TestDb::new()?;
    let mock_fs = MockFs::new()?;
    let config = Arc::new(mock_config(
        mock_fs.rip_dir.clone(),
        mock_fs.library_dir.clone(),
    ));
    let events = mlm_core::Events::new();

    mock_fs.create_libation_folder("B00TEST1", "Test Book 1", vec!["Author 1"])?;

    link_folders_to_library(config.clone(), test_db.db.clone(), &events).await?;

    let r = test_db.db.r_transaction()?;
    let torrent: Option<Torrent> = r.get().primary("B00TEST1".to_string())?;
    assert!(torrent.is_some());
    let torrent = torrent.unwrap();
    assert_eq!(torrent.meta.title, "Test Book 1");
    assert_eq!(torrent.meta.authors, vec!["Author 1"]);
    assert!(torrent.library_path.is_some());

    // Check if files were created in library
    let expected_dir = mock_fs.library_dir.join("Author 1").join("Test Book 1");
    assert!(expected_dir.exists());
    assert!(expected_dir.join("B00TEST1.m4b").exists());
    assert!(expected_dir.join("metadata.json").exists());

    Ok(())
}

#[tokio::test]
async fn test_link_folders_to_library_duplicate_skipping() -> anyhow::Result<()> {
    let test_db = TestDb::new()?;
    let mock_fs = MockFs::new()?;
    let config = Arc::new(mock_config(
        mock_fs.rip_dir.clone(),
        mock_fs.library_dir.clone(),
    ));
    let events = mlm_core::Events::new();

    // Create a better version already in the DB
    let existing = common::MockTorrentBuilder::new("MAM123", "Test Book 1")
        .with_mam_id(123)
        .with_size(1000) // 1000 bytes
        .with_author("Author 1")
        .with_language(mlm_db::Language::English)
        .build();

    {
        let (_guard, rw) = test_db.db.rw_async().await?;
        rw.insert(existing)?;
        rw.commit()?;
    }

    // Create a libation folder with the same title but smaller size (worse version)
    // Libation folder files will have small size "fake audio data" = 15 bytes
    mock_fs.create_libation_folder("B00TEST1", "Test Book 1", vec!["Author 1"])?;

    link_folders_to_library(config.clone(), test_db.db.clone(), &events).await?;

    let r = test_db.db.r_transaction()?;
    let torrent: Option<Torrent> = r.get().primary("B00TEST1".to_string())?;
    // It should NOT be in the DB because it was skipped as a duplicate of a better version
    assert!(torrent.is_none());

    Ok(())
}

#[tokio::test]
async fn test_link_folders_to_library_filter_size_too_small() -> anyhow::Result<()> {
    let test_db = TestDb::new()?;
    let mock_fs = MockFs::new()?;
    let mut config = mock_config(mock_fs.rip_dir.clone(), mock_fs.library_dir.clone());

    if let mlm_core::config::Library::ByRipDir(ref mut l) = config.libraries[0] {
        l.filter.min_size = mlm_db::Size::from_bytes(100); // Libation folder is 15 bytes
    }
    let config = Arc::new(config);
    let events = mlm_core::Events::new();

    mock_fs.create_libation_folder("B00TEST1", "Test Book 1", vec!["Author 1"])?;

    link_folders_to_library(config.clone(), test_db.db.clone(), &events).await?;

    let r = test_db.db.r_transaction()?;
    let torrent: Option<Torrent> = r.get().primary("B00TEST1".to_string())?;
    assert!(
        torrent.is_none(),
        "Should have been skipped due to size filter"
    );

    Ok(())
}

#[tokio::test]
async fn test_link_folders_to_library_filter_media_type_mismatch() -> anyhow::Result<()> {
    let test_db = TestDb::new()?;
    let mock_fs = MockFs::new()?;
    let mut config = mock_config(mock_fs.rip_dir.clone(), mock_fs.library_dir.clone());

    if let mlm_core::config::Library::ByRipDir(ref mut l) = config.libraries[0] {
        l.filter.media_type = vec![mlm_db::MediaType::Ebook]; // Libation is Audiobook
    }
    let config = Arc::new(config);
    let events = mlm_core::Events::new();

    mock_fs.create_libation_folder("B00TEST1", "Test Book 1", vec!["Author 1"])?;

    link_folders_to_library(config.clone(), test_db.db.clone(), &events).await?;

    let r = test_db.db.r_transaction()?;
    let torrent: Option<Torrent> = r.get().primary("B00TEST1".to_string())?;
    assert!(
        torrent.is_none(),
        "Should have been skipped due to media type filter"
    );

    Ok(())
}

#[tokio::test]
async fn test_link_folders_to_library_filter_language_mismatch() -> anyhow::Result<()> {
    let test_db = TestDb::new()?;
    let mock_fs = MockFs::new()?;
    let mut config = mock_config(mock_fs.rip_dir.clone(), mock_fs.library_dir.clone());

    if let mlm_core::config::Library::ByRipDir(ref mut l) = config.libraries[0] {
        l.filter.languages = vec![mlm_db::Language::German]; // Libation is English
    }
    let config = Arc::new(config);
    let events = mlm_core::Events::new();

    mock_fs.create_libation_folder("B00TEST1", "Test Book 1", vec!["Author 1"])?;

    link_folders_to_library(config.clone(), test_db.db.clone(), &events).await?;

    let r = test_db.db.r_transaction()?;
    let torrent: Option<Torrent> = r.get().primary("B00TEST1".to_string())?;
    assert!(
        torrent.is_none(),
        "Should have been skipped due to language filter"
    );

    Ok(())
}

#[tokio::test]
async fn test_link_folders_to_library_libation_missing_subtitle() -> anyhow::Result<()> {
    let test_db = TestDb::new()?;
    let mock_fs = MockFs::new()?;
    let config = Arc::new(mock_config(
        mock_fs.rip_dir.clone(),
        mock_fs.library_dir.clone(),
    ));
    let events = mlm_core::Events::new();

    let folder = mock_fs.rip_dir.join("1977386733");
    fs::create_dir_all(&folder)?;
    let libation_meta = serde_json::json!({
        "asin": "1977386733",
        "title": "The Blueprint",
        "authors": [{"name": "S.E. Harmon"}],
        "narrators": [{"name": "Alexander Cendese"}],
        "series": [],
        "language": "english",
        "format_type": "unabridged",
        "publisher_summary": "Test summary",
        "merchandising_summary": "Test merchandising summary",
        "category_ladders": [],
        "is_adult_product": false,
        "issue_date": "2018-06-30",
        "publication_datetime": "2018-06-30T07:00:00Z",
        "publication_name": "Rules of Possession",
        "publisher_name": "Tantor Media",
        "release_date": "2018-06-30",
        "runtime_length_min": 504
    });
    fs::write(
        folder.join("The Blueprint [1977386733].metadata.json"),
        serde_json::to_string(&libation_meta)?,
    )?;
    fs::write(
        folder.join("The Blueprint [1977386733].m4b"),
        "fake audio data",
    )?;

    link_folders_to_library(config.clone(), test_db.db.clone(), &events).await?;

    let r = test_db.db.r_transaction()?;
    let torrent: Option<Torrent> = r.get().primary("1977386733".to_string())?;
    assert!(torrent.is_some());
    let torrent = torrent.unwrap();
    assert_eq!(torrent.meta.title, "The Blueprint");
    let library_path = torrent.library_path.unwrap();
    assert!(library_path.join("The Blueprint [1977386733].m4b").exists());

    Ok(())
}

#[tokio::test]
async fn test_link_folders_to_library_libation_missing_publication_name() -> anyhow::Result<()> {
    let test_db = TestDb::new()?;
    let mock_fs = MockFs::new()?;
    let config = Arc::new(mock_config(
        mock_fs.rip_dir.clone(),
        mock_fs.library_dir.clone(),
    ));
    let events = mlm_core::Events::new();

    let folder = mock_fs.rip_dir.join("B0DZ3R4CCN");
    fs::create_dir_all(&folder)?;
    let libation_meta = serde_json::json!({
        "asin": "B0DZ3R4CCN",
        "title": "Boraleashe",
        "subtitle": "Lord of the North Wind",
        "authors": [{"name": "A.E. Via"}],
        "narrators": [{"name": "Troy Duran"}],
        "series": [],
        "language": "english",
        "format_type": "unabridged",
        "publisher_summary": "Test summary",
        "merchandising_summary": "Test merchandising summary",
        "category_ladders": [],
        "is_adult_product": false,
        "issue_date": "2025-03-18",
        "publication_datetime": "2025-03-18T07:00:00Z",
        "publisher_name": "Tantor Media",
        "release_date": "2025-03-18",
        "runtime_length_min": 283
    });
    fs::write(
        folder.join("Boraleashe [B0DZ3R4CCN].metadata.json"),
        serde_json::to_string(&libation_meta)?,
    )?;
    fs::write(
        folder.join("Boraleashe [B0DZ3R4CCN].m4b"),
        "fake audio data",
    )?;

    link_folders_to_library(config.clone(), test_db.db.clone(), &events).await?;

    let r = test_db.db.r_transaction()?;
    let torrent: Option<Torrent> = r.get().primary("B0DZ3R4CCN".to_string())?;
    assert!(torrent.is_some());
    let torrent = torrent.unwrap();
    assert_eq!(torrent.meta.title, "Boraleashe: Lord of the North Wind");

    Ok(())
}

#[tokio::test]
async fn test_link_folders_to_library_nextory_wrapped_metadata() -> anyhow::Result<()> {
    let test_db = TestDb::new()?;
    let mock_fs = MockFs::new()?;
    let config = Arc::new(mock_config(
        mock_fs.rip_dir.clone(),
        mock_fs.library_dir.clone(),
    ));
    let events = mlm_core::Events::new();

    mock_fs.create_nextory_folder("nextory_wrapped", true)?;

    link_folders_to_library(config.clone(), test_db.db.clone(), &events).await?;

    let r = test_db.db.r_transaction()?;
    let torrent: Option<Torrent> = r.get().primary("nextory_424242".to_string())?;
    assert!(torrent.is_some());
    let torrent = torrent.unwrap();
    assert_eq!(torrent.meta.title, "Fake Dollar");
    assert_eq!(torrent.meta.authors, vec!["Fake Author"]);
    assert_eq!(torrent.meta.narrators, vec!["Fake Narrator"]);
    assert_eq!(
        torrent.meta.ids.get(mlm_db::ids::ISBN),
        Some(&"9780000000001".to_string())
    );
    assert_eq!(torrent.meta.language, Some(mlm_db::Language::Swedish));
    assert_eq!(torrent.meta.series[0].name, "Fake");
    assert!(torrent.library_path.is_some());

    let expected_dir = mock_fs
        .library_dir
        .join("Fake Author")
        .join("Fake")
        .join("Fake #2 - Fake Dollar {Fake Narrator}");
    assert!(expected_dir.exists());
    assert!(expected_dir.join("Fake Dollar - Fake Author.m4a").exists());
    assert!(expected_dir.join("metadata.json").exists());

    Ok(())
}

#[tokio::test]
async fn test_link_folders_to_library_nextory_raw_metadata() -> anyhow::Result<()> {
    let test_db = TestDb::new()?;
    let mock_fs = MockFs::new()?;
    let config = Arc::new(mock_config(
        mock_fs.rip_dir.clone(),
        mock_fs.library_dir.clone(),
    ));
    let events = mlm_core::Events::new();

    mock_fs.create_nextory_folder("nextory_raw_only", false)?;

    link_folders_to_library(config.clone(), test_db.db.clone(), &events).await?;

    let r = test_db.db.r_transaction()?;
    let torrent: Option<Torrent> = r.get().primary("nextory_424242".to_string())?;
    assert!(torrent.is_some());
    let torrent = torrent.unwrap();
    assert_eq!(torrent.meta.title, "Fake Dollar");
    assert_eq!(torrent.meta.authors, vec!["Fake Author"]);
    assert_eq!(torrent.selected_audio_format, Some(".m4a".to_string()),);
    assert!(torrent.library_path.is_some());

    Ok(())
}

#[tokio::test]
async fn test_link_folders_to_library_libation_series_subtitle_does_not_overwrite_book_title()
-> anyhow::Result<()> {
    let test_db = TestDb::new()?;
    let mock_fs = MockFs::new()?;
    let config = Arc::new(mock_config(
        mock_fs.rip_dir.clone(),
        mock_fs.library_dir.clone(),
    ));
    let events = mlm_core::Events::new();

    let cases = [
        (
            "B0D8LFR3R2",
            "The Order: Kingdom of Fallen Ash",
            "The Order Series, Book 1",
            "Kingdom of Fallen Ash",
            "1",
        ),
        (
            "B0DSQW6S4W",
            "The Order: Labyrinth of Twisted Games",
            "The Order Series, Book 2",
            "Labyrinth of Twisted Games",
            "2",
        ),
        (
            "B0FNYKHDXG",
            "The Order: Rise of the New Empire",
            "The Order Series, Book 4",
            "Rise of the New Empire",
            "4",
        ),
    ];

    for (asin, title, subtitle, expected_title, expected_series_num) in cases {
        let folder = mock_fs.rip_dir.join(format!("The Order [{asin}]"));
        fs::create_dir_all(&folder)?;
        let libation_meta = serde_json::json!({
            "asin": asin,
            "title": title,
            "subtitle": subtitle,
            "authors": [{"name": "Katerina St Clair"}],
            "narrators": [{"name": "Isabelle Turner"}],
            "series": [],
            "language": "english",
            "format_type": "unabridged",
            "publisher_summary": "Test summary",
            "merchandising_summary": "Test merchandising summary",
            "category_ladders": [],
            "is_adult_product": false,
            "issue_date": "2025-01-01",
            "publication_datetime": "2025-01-01T00:00:00Z",
            "publication_name": "The Order Series",
            "publisher_name": "Podium Audio",
            "release_date": "2025-01-01",
            "runtime_length_min": 60
        });
        fs::write(
            folder.join(format!("The Order [{asin}].metadata.json")),
            serde_json::to_string(&libation_meta)?,
        )?;
        fs::write(
            folder.join(format!("The Order [{asin}].m4b")),
            "fake audio data",
        )?;

        link_folders_to_library(config.clone(), test_db.db.clone(), &events).await?;

        let r = test_db.db.r_transaction()?;
        let torrent: Option<Torrent> = r.get().primary(asin.to_string())?;
        assert!(torrent.is_some());
        let torrent = torrent.unwrap();

        assert_eq!(torrent.meta.title, expected_title);
        assert_eq!(torrent.meta.series.len(), 1);
        assert_eq!(torrent.meta.series[0].name, "The Order");
        assert_eq!(
            torrent.meta.series[0].entries.to_string(),
            expected_series_num
        );
    }

    Ok(())
}
