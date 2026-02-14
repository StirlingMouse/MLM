use mlm_db::TorrentMeta;
use mlm_meta::Provider;
use mlm_meta::http::HttpClient;
use mlm_meta::providers::OpenLibrary;

mod mock_openlibrary;

#[tokio::test]
async fn openlibrary_parses_search_results() {
    let prov = OpenLibrary::with_client(mock_openlibrary::boxed());
    let query_meta = TorrentMeta {
        title: "The Lord of the Rings".to_string(),
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should fetch metadata");
    assert!(m.title.contains("Lord of the Rings"));
    assert!(!m.authors.is_empty());
}

#[tokio::test]
async fn openlibrary_matches_title_and_author() {
    let prov = OpenLibrary::with_client(mock_openlibrary::boxed());
    let query_meta = TorrentMeta {
        title: "The Lord of the Rings".to_string(),
        authors: vec!["J.R.R. Tolkien".to_string()],
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should match title+author");
    assert!(m.title.to_lowercase().contains("lord of the rings"));
    assert!(
        m.authors
            .iter()
            .any(|a| a.to_lowercase().contains("tolkien"))
    );
}

#[tokio::test]
async fn openlibrary_extracts_isbn() {
    let prov = OpenLibrary::with_client(mock_openlibrary::boxed());
    let query_meta = TorrentMeta {
        title: "The Lord of the Rings".to_string(),
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should fetch metadata");
    assert!(
        m.ids.values().any(|v| v.starts_with("978")),
        "should have ISBN"
    );
}

#[tokio::test]
async fn openlibrary_extracts_subjects_as_tags() {
    let prov = OpenLibrary::with_client(mock_openlibrary::boxed());
    let query_meta = TorrentMeta {
        title: "The Lord of the Rings".to_string(),
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should fetch metadata");
    assert!(!m.tags.is_empty(), "should have subject tags");
}

#[tokio::test]
async fn openlibrary_title_only_search() {
    let prov = OpenLibrary::with_client(mock_openlibrary::boxed());
    let query_meta = TorrentMeta {
        title: "The Lord of the Rings".to_string(),
        authors: vec![],
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should find result with title only");
    assert!(m.title.to_lowercase().contains("lord of the rings"));
}

#[tokio::test]
async fn openlibrary_no_results() {
    use std::sync::Arc;

    struct EmptyClient;

    #[async_trait::async_trait]
    impl HttpClient for EmptyClient {
        async fn get(&self, _url: &str) -> anyhow::Result<String> {
            Ok(r#"{"numFound": 0, "docs": []}"#.to_string())
        }

        async fn post(
            &self,
            _url: &str,
            _body: Option<&str>,
            _headers: &[(&str, &str)],
        ) -> anyhow::Result<String> {
            anyhow::bail!("post not implemented")
        }
    }

    let prov = OpenLibrary::with_client(Arc::new(EmptyClient));
    let query_meta = TorrentMeta {
        title: "Nonexistent Title XYZ123".to_string(),
        ..Default::default()
    };
    let res = prov.fetch(&query_meta).await;
    assert!(res.is_err(), "expected no results for nonexistent title");
}
