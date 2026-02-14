mod common;

use common::{MockFs, MockTorrentBuilder, TestDb, mock_config};
use mlm_core::cleaner::run_library_cleaner;
use mlm_db::{DatabaseExt, Torrent};
use std::sync::Arc;

#[tokio::test]
async fn test_run_library_cleaner() -> anyhow::Result<()> {
    let test_db = TestDb::new()?;
    let mock_fs = MockFs::new()?;
    let config = Arc::new(mock_config(
        mock_fs.rip_dir.clone(),
        mock_fs.library_dir.clone(),
    ));

    // Create two versions of the same book
    let lib_path1 = mock_fs.library_dir.join("Author 1").join("Book 1 (v1)");
    let lib_path2 = mock_fs.library_dir.join("Author 1").join("Book 1 (v2)");
    std::fs::create_dir_all(&lib_path1)?;
    std::fs::create_dir_all(&lib_path2)?;
    std::fs::write(lib_path1.join("file.m4b"), "v1")?;
    std::fs::write(lib_path2.join("file.m4b"), "v2 is longer than v1")?;

    let mut t1 = MockTorrentBuilder::new("ID1", "Book 1")
        .with_library_path(lib_path1.clone())
        .with_size(100)
        .with_author("Author 1")
        .with_language(mlm_db::Language::English)
        .build();
    t1.library_files = vec!["file.m4b".into()];

    let mut t2 = MockTorrentBuilder::new("ID2", "Book 1")
        .with_library_path(lib_path2.clone())
        .with_size(200) // Better version because it's larger
        .with_author("Author 1")
        .with_language(mlm_db::Language::English)
        .build();
    t2.library_files = vec!["file.m4b".into()];

    {
        let (_guard, rw) = test_db.db.rw_async().await?;
        rw.insert(t1)?;
        rw.insert(t2)?;
        rw.commit()?;
    }

    run_library_cleaner(config.clone(), test_db.db.clone()).await?;

    let r = test_db.db.r_transaction()?;
    let t1_after: Torrent = r.get().primary("ID1".to_string())?.unwrap();
    let t2_after: Torrent = r.get().primary("ID2".to_string())?.unwrap();

    // t1 should be replaced_with t2
    assert!(t1_after.replaced_with.is_some(), "t1 should be replaced");
    assert_eq!(t1_after.replaced_with.unwrap().0, "ID2");
    assert!(
        t1_after.library_path.is_none(),
        "t1 library path should be cleared"
    );

    // t2 should still be there
    assert!(
        t2_after.replaced_with.is_none(),
        "t2 should not be replaced"
    );
    assert!(
        t2_after.library_path.is_some(),
        "t2 library path should still be set"
    );

    // Files for t1 should be deleted
    assert!(!lib_path1.exists(), "t1 files should be deleted");
    // Files for t2 should still exist
    assert!(lib_path2.exists(), "t2 files should still exist");

    Ok(())
}
