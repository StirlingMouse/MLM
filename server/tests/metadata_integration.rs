mod common;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use mlm_db::{Event, EventKey, EventType, TorrentMeta as MetadataQuery};

use async_trait::async_trait;
use common::{TestDb, mock_config};
use mlm_core::metadata::MetadataService;
use mlm_core::{Context, Events, SsrBackend, Stats, Triggers};
use url::Url;

// Simple mock fetcher that returns inline mock data for tests.
struct MockFetcher;

#[async_trait]
impl mlm_meta::http::HttpClient for MockFetcher {
    async fn get(&self, url: &str) -> anyhow::Result<String> {
        let u = Url::parse(url).map_err(|e| anyhow::anyhow!(e))?;
        if !u.host_str().is_some_and(|h| h.contains("romance.io")) {
            return Err(anyhow::anyhow!("unexpected host in test fetch"));
        }
        if u.path().starts_with("/json/search_books") {
            return Ok(r#"{
  "success": true,
  "books": [
    {
      "_id":"68b95a390bc0cee156edaf2b",
      "info":{"title":"Of Ink and Alchemy"},
      "authors":[{"name":"Sloane St. James"}],
      "url":"/books/68b95a390bc0cee156edaf2b/of-ink-and-alchemy-sloane-st-james"
    }
  ]
}"#
            .to_string());
        }
        if u.path().starts_with("/json/search_authors") {
            return Ok(r#"{ "success": true, "authors": [] }"#.to_string());
        }
        if u.path().starts_with("/search") {
            return Ok("<html><body>search</body></html>".to_string());
        }

        Ok(r#"
<html><head>
<script type="application/ld+json">
{
  "@graph": [{
    "name": "Of Ink and Alchemy",
    "author": [{"name":"Sloane St. James"}],
    "description": "A dark contemporary romance with friends to lovers."
  }]
}
</script>
</head><body>
<ul id="valid-topics-list">
  <li><a class="topic">Contemporary</a></li>
  <li><a class="topic">Dark Romance</a></li>
  <li><a class="topic">Age Difference</a></li>
  <li><a class="topic">Friends to Lovers</a></li>
</ul>
</body></html>
"#
        .to_string())
    }

    async fn post(
        &self,
        _url: &str,
        _body: Option<&str>,
        _headers: &[(&str, &str)],
    ) -> anyhow::Result<String> {
        Err(anyhow::anyhow!("post not implemented in mock fetcher"))
    }
}

#[tokio::test]
async fn test_metadata_fetch_and_persist_romanceio() -> Result<()> {
    let test_db = TestDb::new()?;

    // minimal config/context
    let temp = tempfile::tempdir()?;
    let rip = temp.path().join("rip");
    let lib = temp.path().join("library");
    std::fs::create_dir_all(&rip)?;
    std::fs::create_dir_all(&lib)?;
    let cfg = mock_config(rip, lib);

    let _default_timeout = StdDuration::from_secs(5);

    // Use a title known to the plan/romanceio mock. Inject the test fetcher
    // implementation into the RomanceIo provider so we don't make network
    // requests during tests.
    let mock_fetcher = Arc::new(MockFetcher);
    // Rebuild a metadata service with a RomanceIo using the mock fetcher.
    let rom = mlm_meta::providers::RomanceIo::with_client(mock_fetcher.clone());
    let svc = MetadataService::new(
        vec![(Arc::new(rom), StdDuration::from_secs(5))],
        StdDuration::from_secs(5),
    );
    let metadata = Arc::new(tokio::sync::Mutex::new(svc));

    let ctx = Context {
        backend: Some(Arc::new(SsrBackend {
            db: test_db.db.clone(),
            mam: Arc::new(Err(anyhow::anyhow!("no mam"))),
            metadata: metadata.clone(),
        })),
        config: Arc::new(tokio::sync::Mutex::new(Arc::new(cfg))),
        stats: Stats::new(),
        events: Events::new(),
        triggers: Triggers::default(),
    };

    // Use a title known to the plan/romanceio mock
    let q = MetadataQuery {
        title: "Of Ink and Alchemy".to_string(),
        ..Default::default()
    };
    let meta = metadata.lock().await.fetch_and_persist(&ctx, q).await?;

    // Expect meta to contain some categories/tags
    assert!(
        meta.title.to_lowercase().contains("ink")
            || !meta.categories.is_empty()
            || !meta.tags.is_empty()
    );

    // Ensure an Event::Updated was inserted
    let r = test_db.db.r_transaction()?;
    let events = r.scan().secondary::<Event>(EventKey::created_at)?;
    let events = events.all()?;
    let mut found = false;
    for ev in events {
        let ev = ev?;
        if let EventType::Updated { source, .. } = ev.event
            && source.0 == mlm_db::MetadataSource::Match
            && source.1 == "romanceio"
        {
            found = true;
            break;
        }
    }
    assert!(found, "Expected Event::Updated from romanceio provider");

    Ok(())
}

/// Test MaM provider with mock MaM server.
/// This test starts the mock MaM server and uses it to test MaM provider functionality.
#[tokio::test]
async fn test_mam_provider_fetch_with_mock_server() -> Result<()> {
    let test_db = TestDb::new()?;

    let temp = tempfile::tempdir()?;
    let rip = temp.path().join("rip");
    let lib = temp.path().join("library");
    std::fs::create_dir_all(&rip)?;
    std::fs::create_dir_all(&lib)?;
    let cfg = mock_config(rip, lib);

    // Start mock MaM server
    let mock_port = 14997u16;
    let mock_url = format!("http://127.0.0.1:{}", mock_port);

    // Spawn mock server
    let mock_bin = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("mock_server");
    let mock_bin = if mock_bin.exists() {
        mock_bin
    } else {
        // Fallback: look in target/debug
        std::path::PathBuf::from("target/debug/mock_server")
    };

    let mut mock_server = std::process::Command::new(&mock_bin)
        .env("MOCK_PORT", mock_port.to_string())
        .env("MLM_MAM_BASE_URL", &mock_url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    // Wait for mock server to start (synchronous sleep in test context)
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Check if mock server started successfully
    let mock_running = mock_server.try_wait().map(|o| o.is_none()).unwrap_or(false);
    if !mock_running {
        eprintln!("Warning: mock_server may not have started, continuing anyway...");
    }

    // Set env var for MaM to use mock server
    unsafe { std::env::set_var("MLM_MAM_BASE_URL", &mock_url) };

    // Create MaM instance pointing to mock server
    let mam_result = mlm_mam::api::MaM::new("test-user", test_db.db.clone()).await;
    let mam: Arc<mlm_mam::api::MaM<'static>> = match mam_result {
        Ok(m) => Arc::new(m),
        Err(e) => {
            let _ = mock_server.kill();
            unsafe { std::env::remove_var("MLM_MAM_BASE_URL") };
            return Err(anyhow::anyhow!("Failed to create MaM instance: {}", e));
        }
    };

    // Create metadata service with MaM provider
    let mut svc = MetadataService::new(vec![], StdDuration::from_secs(5));
    svc.register_mam(mam.clone(), StdDuration::from_secs(5));
    let metadata = Arc::new(tokio::sync::Mutex::new(svc));

    let ctx = Context {
        backend: Some(Arc::new(SsrBackend {
            db: test_db.db.clone(),
            mam: Arc::new(Ok(mam.clone())),
            metadata: metadata.clone(),
        })),
        config: Arc::new(tokio::sync::Mutex::new(Arc::new(cfg))),
        stats: Stats::new(),
        events: Events::new(),
        triggers: Triggers::default(),
    };

    // Test 1: MaM provider should be registered
    let providers = metadata.lock().await.enabled_providers();
    assert!(
        providers.contains(&"mam".to_string()),
        "MaM provider should be registered"
    );

    // Test 2: Search by title should work (mock server returns results for "test book")
    let q = MetadataQuery {
        title: "Test Book".to_string(),
        authors: vec!["Test Author".to_string()],
        ..Default::default()
    };
    let result = metadata.lock().await.fetch_provider(&ctx, q, "mam").await;
    assert!(
        result.is_ok(),
        "MaM search should succeed: {:?}",
        result.err()
    );
    let meta = result.unwrap();
    assert!(
        !meta.title.is_empty(),
        "MaM should return a result with title"
    );

    // Test 3: Fetch by MaM ID should work
    let q_with_id = MetadataQuery {
        ids: std::collections::BTreeMap::from([(
            mlm_db::ids::MAM.to_string(),
            "12345".to_string(),
        )]),
        title: "Test".to_string(),
        ..Default::default()
    };
    let result = metadata
        .lock()
        .await
        .fetch_provider(&ctx, q_with_id, "mam")
        .await;
    assert!(
        result.is_ok(),
        "MaM fetch by ID should succeed: {:?}",
        result.err()
    );
    let meta = result.unwrap();
    // Verify the result came from the direct-ID lookup (mock returns "Updated Mock Search Result Title")
    assert_eq!(
        meta.title, "Updated Mock Search Result Title",
        "MaM direct ID lookup should return the mock torrent title for ID 12345"
    );

    // Cleanup
    let _ = mock_server.kill();
    unsafe { std::env::remove_var("MLM_MAM_BASE_URL") };

    Ok(())
}

/// Test that MaM provider returns proper error for unknown provider id.
#[tokio::test]
async fn test_unknown_provider_error() -> Result<()> {
    let test_db = TestDb::new()?;

    let temp = tempfile::tempdir()?;
    let rip = temp.path().join("rip");
    let lib = temp.path().join("library");
    std::fs::create_dir_all(&rip)?;
    std::fs::create_dir_all(&lib)?;
    let cfg = mock_config(rip, lib);

    // Create a metadata service with no providers
    let metadata = Arc::new(tokio::sync::Mutex::new(MetadataService::new(
        vec![],
        StdDuration::from_secs(5),
    )));
    let ctx = Context {
        backend: Some(Arc::new(SsrBackend {
            db: test_db.db.clone(),
            mam: Arc::new(Err(anyhow::anyhow!("no mam"))),
            metadata: metadata.clone(),
        })),
        config: Arc::new(tokio::sync::Mutex::new(Arc::new(cfg))),
        stats: Stats::new(),
        events: Events::new(),
        triggers: Triggers::default(),
    };

    // Query with unknown provider should fail gracefully
    let q = MetadataQuery {
        title: "Test Title".to_string(),
        ..Default::default()
    };
    let result = metadata
        .lock()
        .await
        .fetch_provider(&ctx, q, "nonexistent")
        .await;
    assert!(result.is_err(), "Should fail for unknown provider");
    assert_eq!(
        result.unwrap_err().to_string(),
        "unknown provider id: nonexistent"
    );

    Ok(())
}
