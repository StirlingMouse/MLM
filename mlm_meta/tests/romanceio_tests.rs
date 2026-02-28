use mlm_db::TorrentMeta;
use mlm_meta::Provider;
use mlm_meta::http::HttpClient;
use mlm_meta::providers::RomanceIo;

mod mock_fetcher;

#[tokio::test]
async fn romanceio_parses_book() {
    let prov = RomanceIo::with_client(mock_fetcher::boxed());
    let query_meta = TorrentMeta {
        title: "Of Ink and Alchemy".to_string(),
        ..Default::default()
    };
    let m = prov.fetch(&query_meta).await.expect("should parse book");
    assert!(m.title.contains("Of Ink and Alchemy"));
    assert!(m.authors.iter().any(|a| a.contains("Sloane")));
    assert!(!m.description.is_empty());
}

#[tokio::test]
async fn romanceio_matches_title_and_author() {
    let prov = RomanceIo::with_client(mock_fetcher::boxed());
    let query_meta = TorrentMeta {
        title: "Of Ink and Alchemy".to_string(),
        authors: vec!["Sloane St. James".to_string()],
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should match title+author");
    assert!(m.title.to_lowercase().contains("of ink and alchemy"));
    assert!(
        m.authors
            .iter()
            .any(|a| a.to_lowercase().contains("sloane"))
    );
}

#[tokio::test]
async fn romanceio_rejects_title_with_nonmatching_author() {
    let prov = RomanceIo::with_client(mock_fetcher::boxed());
    let query_meta = TorrentMeta {
        title: "Of Ink and Alchemy".to_string(),
        authors: vec!["Some Other Author".to_string()],
        ..Default::default()
    };
    let res = prov.fetch(&query_meta).await;
    assert!(res.is_err(), "expected no result for non-matching author");
}

#[tokio::test]
async fn romanceio_rejects_different_title_same_author() {
    let prov = RomanceIo::with_client(mock_fetcher::boxed());
    let query_meta = TorrentMeta {
        title: "A Title That Does Not Exist".to_string(),
        authors: vec!["Sloane St. James".to_string()],
        ..Default::default()
    };
    let res = prov.fetch(&query_meta).await;
    assert!(
        res.is_err(),
        "expected no result for different title even if author matches"
    );
}

#[tokio::test]
async fn romanceio_finds_late_result_in_json_array() {
    use anyhow::Result;
    use std::sync::Arc;

    struct CustomClient;

    #[async_trait::async_trait]
    impl HttpClient for CustomClient {
        async fn get(&self, url: &str) -> Result<String> {
            if url.contains("/json/search_books") {
                let data = r#"{
                    "success": true,
                    "books": [
                        {"_id":"x1","info":{"title":"Unrelated Book"},"url":"/books/x1/unrelated"},
                        {"_id":"x2","info":{"title":"Another Irrelevant"},"url":"/books/x2/irrelevant"},
                        {"_id":"68b95a390bc0cee156edaf2b","info":{"title":"Of Ink and Alchemy"},"authors":[{"name":"Sloane St. James"}],"url":"/books/68b95a390bc0cee156edaf2b/of-ink-and-alchemy-sloane-st-james"}
                    ]
                }"#;
                return Ok(data.to_string());
            }
            if url.contains("/books/68b95a390bc0cee156edaf2b") {
                let mut dir = std::env::current_dir()?;
                loop {
                    let candidate = dir.join("plan/romanceio/book.html");
                    if candidate.exists() {
                        return Ok(std::fs::read_to_string(candidate)?);
                    }
                    if !dir.pop() {
                        break;
                    }
                }
                return Err(anyhow::anyhow!("plan file not found"));
            }
            Err(anyhow::anyhow!("unexpected url"))
        }

        async fn post(
            &self,
            _url: &str,
            _body: Option<&str>,
            _headers: &[(&str, &str)],
        ) -> Result<String> {
            Err(anyhow::anyhow!("post not implemented"))
        }
    }

    let prov = RomanceIo::with_client(Arc::new(CustomClient));
    let query_meta = TorrentMeta {
        title: "Of Ink and Alchemy".to_string(),
        authors: vec!["Sloane St. James".to_string()],
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should find late result");
    assert!(m.title.to_lowercase().contains("of ink and alchemy"));
}
