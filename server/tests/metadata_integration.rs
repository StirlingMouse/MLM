mod common;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use mlm_db::{Event, EventKey, EventType, TorrentMeta as MetadataQuery};

use async_trait::async_trait;
use common::{TestDb, mock_config};
use mlm::metadata::MetadataService;
use mlm::stats::Context;
use url::Url;

// Simple mock fetcher that resolves plan files from the repo for tests.
struct MockFetcher;

#[async_trait]
impl mlm_meta::http::HttpClient for MockFetcher {
    async fn get(&self, url: &str) -> anyhow::Result<String> {
        let u = Url::parse(url).map_err(|e| anyhow::anyhow!(e))?;
        let rel = if u.host_str().unwrap_or_default().contains("romance.io") {
            if u.path().starts_with("/json/search_books") {
                "plan/romanceio/search_books.json"
            } else if u.path().starts_with("/json/search_authors") {
                "plan/romanceio/search_authors.json"
            } else if u.path().starts_with("/search") {
                "plan/romanceio/search.html"
            } else {
                "plan/romanceio/book.html"
            }
        } else {
            return Err(anyhow::anyhow!("unexpected host in test fetch"));
        };

        // Walk upwards from CWD to find the plan file (same approach as mlm_meta tests).
        let mut dir = std::env::current_dir()?;
        loop {
            let candidate = dir.join(rel);
            if candidate.exists() {
                let s = std::fs::read_to_string(candidate)?;
                return Ok(s);
            }
            if !dir.pop() {
                break;
            }
        }
        Err(anyhow::anyhow!("could not find plan file"))
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
    let providers = cfg.metadata_providers.clone();
    // convert provider config to server metadata provider settings
    let provider_settings: Vec<mlm::metadata::ProviderSetting> = providers
        .iter()
        .map(|p| match p {
            mlm::config::ProviderConfig::Hardcover(c) => {
                mlm::metadata::ProviderSetting::Hardcover {
                    enabled: c.enabled,
                    timeout_secs: c.timeout_secs,
                    api_key: c.api_key.clone(),
                }
            }
            mlm::config::ProviderConfig::RomanceIo(c) => {
                mlm::metadata::ProviderSetting::RomanceIo {
                    enabled: c.enabled,
                    timeout_secs: c.timeout_secs,
                }
            }
        })
        .collect();
    let metadata =
        MetadataService::from_settings(&provider_settings, std::time::Duration::from_secs(5));
    let metadata = Arc::new(metadata);

    let ctx = Context {
        config: Arc::new(tokio::sync::Mutex::new(Arc::new(cfg))),
        db: test_db.db.clone(),
        mam: Arc::new(Err(anyhow::anyhow!("no mam"))),
        stats: mlm::stats::Stats::new(),
        metadata: metadata.clone(),
        triggers: mlm::stats::Triggers {
            search_tx: std::collections::BTreeMap::new(),
            import_tx: std::collections::BTreeMap::new(),
            torrent_linker_tx: tokio::sync::watch::channel(()).0,
            folder_linker_tx: tokio::sync::watch::channel(()).0,
            downloader_tx: tokio::sync::watch::channel(()).0,
            audiobookshelf_tx: tokio::sync::watch::channel(()).0,
        },
    };

    // Use a title known to the plan/romanceio mock. Inject the test fetcher
    // implementation into the RomanceIo provider so we don't make network
    // requests during tests.
    // Replace the RomanceIo provider in the metadata service with one that
    // uses the MockFetcher.
    let mock_fetcher = std::sync::Arc::new(MockFetcher);
    // Rebuild a metadata service with a RomanceIo using the mock fetcher.
    let rom = mlm_meta::providers::RomanceIo::with_client(mock_fetcher.clone());
    let svc = mlm::metadata::MetadataService::new(
        vec![(std::sync::Arc::new(rom), std::time::Duration::from_secs(5))],
        std::time::Duration::from_secs(5),
    );
    let metadata = Arc::new(svc);

    let ctx = Context {
        metadata: metadata.clone(),
        ..ctx
    };

    // Use a title known to the plan/romanceio mock
    let mut q: MetadataQuery = Default::default();
    q.title = "Of Ink and Alchemy".to_string();
    let meta = metadata.fetch_and_persist(&ctx, q).await?;

    // Expect meta to contain some categories/tags from plan/romanceio/book.html
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
