use std::sync::Arc;

use mlm_db::TorrentMeta;
use mlm_meta::Provider;
use mlm_meta::providers::Hardcover;

mod helper {
    use anyhow::Result;
    use async_trait::async_trait;
    use mlm_meta::http::HttpClient;

    pub struct MockClient {
        resps: std::sync::Mutex<Vec<String>>,
    }

    impl MockClient {
        pub fn new(resp: &str) -> Self {
            Self {
                resps: std::sync::Mutex::new(vec![resp.to_string()]),
            }
        }
    }

    #[async_trait]
    impl HttpClient for MockClient {
        async fn get(&self, _url: &str) -> Result<String> {
            Ok(String::new())
        }

        async fn post(
            &self,
            _url: &str,
            _body: Option<&str>,
            _headers: &[(&str, &str)],
        ) -> Result<String> {
            let mut guard = self.resps.lock().unwrap();
            if guard.is_empty() {
                return Ok(String::new());
            }
            Ok(guard.remove(0))
        }
    }
}

#[tokio::test]
async fn hardcover_selects_best_candidate() {
    use helper::MockClient;

    let data = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "title": "The Great Adventure", "author_names": ["Alice Author"], "description": "A" } },
        { "document": { "title": "Great Adventure", "author_names": ["Bob Smith"], "description": "B" } }
    ] } } } }"#;

    let client = Arc::new(MockClient::new(data));
    let prov = Hardcover::with_client("http://example/graphql", client, None);

    let query_meta = TorrentMeta {
        title: "Great Adventure".to_string(),
        authors: vec!["Bob Smith".to_string()],
        ..Default::default()
    };

    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should select best candidate");
    assert!(m.authors.iter().any(|a| a.to_lowercase().contains("bob")));
    assert!(m.title.to_lowercase().contains("great"));
}

#[tokio::test]
async fn hardcover_parses_tags_and_isbn() {
    use helper::MockClient;

    let data = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "title": "Unique Book", "author_names": ["Unique Author"], "description": "desc", "tags": ["Tropes"], "genres": ["Romance"], "isbns": ["9781234567897"] } }
    ] } } } }"#;

    let client = Arc::new(MockClient::new(data));
    let prov = Hardcover::with_client("http://example/graphql", client, None);

    let query_meta = TorrentMeta {
        title: "Unique Book".to_string(),
        ..Default::default()
    };

    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should parse tags and isbn");
    assert!(m.tags.iter().any(|t| t == "tropes"));
    assert!(m.tags.iter().any(|t| t == "romance"));
    assert_eq!(m.ids.get("isbn").map(|s| s.as_str()), Some("9781234567897"));
}

#[tokio::test]
async fn hardcover_empty_results_returns_err() {
    use helper::MockClient;

    let data = r#"{ "data": { "search": { "results": { "hits": [] } } } }"#;
    let client = Arc::new(MockClient::new(data));
    let prov = Hardcover::with_client("http://example/graphql", client, None);

    let query_meta = TorrentMeta {
        title: "Does Not Exist".to_string(),
        ..Default::default()
    };
    let res = prov.fetch(&query_meta).await;
    assert!(res.is_err(), "expected error for empty results");
}

#[tokio::test]
async fn hardcover_handles_malformed_fields_gracefully() {
    use helper::MockClient;

    let data = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "title": "Any Title", "description": "only desc", "tags": null, "genres": 123 } }
    ] } } } }"#;
    let client = Arc::new(MockClient::new(data));
    let prov = Hardcover::with_client("http://example/graphql", client, None);

    let query_meta = TorrentMeta {
        title: "Any Title".to_string(),
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should handle malformed fields");
    assert_eq!(m.title, "Any Title");
    assert_eq!(m.description, "only desc");
    assert!(m.tags.is_empty());
    assert!(!m.ids.contains_key("isbn"));
}

#[tokio::test]
async fn hardcover_uses_first_isbn_when_multiple_present() {
    use helper::MockClient;

    let data = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "title": "Multi ISBN", "author_names": ["A"], "isbns": ["FIRSTISBN","SECONDISBN"] } }
    ] } } } }"#;
    let client = Arc::new(MockClient::new(data));
    let prov = Hardcover::with_client("http://example/graphql", client, None);

    let query_meta = TorrentMeta {
        title: "Multi ISBN".to_string(),
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should parse multiple isbns");
    assert_eq!(m.ids.get("isbn").map(|s| s.as_str()), Some("FIRSTISBN"));
}

#[tokio::test]
async fn hardcover_tie_breaker_prefers_first_result() {
    use helper::MockClient;

    let data = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "title": "Tie Book", "author_names": ["Author One"], "description": "first" } },
        { "document": { "title": "Tie Book", "author_names": ["Author One"], "description": "second" } }
    ] } } } }"#;

    let client = Arc::new(MockClient::new(data));
    let prov = Hardcover::with_client("http://example/graphql", client, None);

    let query_meta = TorrentMeta {
        title: "Tie Book".to_string(),
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should return first result on tie");
    assert!(m.description == "first");
}

#[tokio::test]
async fn hardcover_handles_minor_typos() {
    use helper::MockClient;

    let data = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "title": "Great Adventure", "author_names": ["Bob Smith"], "description": "B" } }
    ] } } } }"#;

    let client = Arc::new(MockClient::new(data));
    let prov = Hardcover::with_client("http://example/graphql", client, None);

    let query_meta = TorrentMeta {
        title: "Gret Adventure".to_string(),
        authors: vec!["Bob Smith".to_string()],
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should match despite typo");
    assert!(m.title.to_lowercase().contains("great adventure"));
}

#[tokio::test]
async fn hardcover_parses_isbn_from_search_results() {
    use helper::MockClient;

    let search = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "id": 123, "title": "Detailed Book", "author_names": ["Detail Author"], "description": "short desc", "isbns": ["9781111111111"], "series_names": ["Series A"] } }
    ] } } } }"#;

    let client = Arc::new(MockClient::new(search));
    let prov = Hardcover::with_client("http://example/graphql", client, None);

    let query_meta = TorrentMeta {
        title: "Detailed Book".to_string(),
        ..Default::default()
    };
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should parse search results");

    assert_eq!(m.ids.get("isbn").map(|s| s.as_str()), Some("9781111111111"));
    assert!(m.series.iter().any(|s| s.name == "Series A"));
    assert_eq!(m.description, "short desc");
}

#[tokio::test]
async fn hardcover_title_only_fallback_still_scores_with_author() {
    use helper::MockClient;

    // Query for "Boss of the Year" by "Nicole French"
    // Results include a similar title by a different author
    // The fallback to title-only should NOT match because author doesn't match
    let data = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "title": "Not the Boss of the Year", "author_names": ["J.S. Cooper"], "description": "wrong author" } },
        { "document": { "title": "Boss of the Year", "author_names": ["Nicole French"], "description": "correct" } }
    ] } } } }"#;

    let client = Arc::new(MockClient::new(data));
    let prov = Hardcover::with_client("http://example/graphql", client, None);

    let query_meta = TorrentMeta {
        title: "Boss of the Year".to_string(),
        authors: vec!["Nicole French".to_string()],
        ..Default::default()
    };

    // Should NOT match "Not the Boss of the Year" by J.S. Cooper
    // Should either match the correct one OR return error
    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should find correct match");
    assert!(
        m.title.to_lowercase().contains("boss of the year"),
        "title should contain 'Boss of the Year'"
    );
    assert!(
        m.authors
            .iter()
            .any(|a| a.to_lowercase().contains("nicole")),
        "author should be Nicole French, got: {:?}",
        m.authors
    );
}
