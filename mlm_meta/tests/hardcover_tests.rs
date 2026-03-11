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

        pub fn new_many(resps: &[&str]) -> Self {
            Self {
                resps: std::sync::Mutex::new(resps.iter().map(|s| s.to_string()).collect()),
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

    let search = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "id": 10, "title": "The Great Adventure", "author_names": ["Alice Author"], "description": "A" } },
        { "document": { "id": 11, "title": "Great Adventure", "author_names": ["Bob Smith"], "description": "B" } }
    ] } } } }"#;
    let detail = r#"{ "data": { "books_by_pk": {
        "id": 11,
        "title": "Great Adventure",
        "subtitle": null,
        "headline": null,
        "description": "B",
        "contributions": [{ "author": { "name": "Bob Smith" }, "contribution": null }],
        "book_series": [],
        "taggings": []
    } } }"#;

    let client = Arc::new(MockClient::new_many(&[search, detail]));
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

    let search = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "id": 20, "title": "Unique Book", "author_names": ["Unique Author"], "description": "desc", "isbns": ["9781234567897"] } }
    ] } } } }"#;
    let detail = r#"{ "data": { "books_by_pk": {
        "id": 20,
        "title": "Unique Book",
        "subtitle": null,
        "headline": null,
        "description": "desc",
        "contributions": [{ "author": { "name": "Unique Author" }, "contribution": null }],
        "book_series": [],
        "taggings": [
            { "id": 1, "tag": { "tag": "Tropes", "tag_category": { "category": "Tag" } } },
            { "id": 2, "tag": { "tag": "Romance", "tag_category": { "category": "Genre" } } }
        ]
    } } }"#;

    let client = Arc::new(MockClient::new_many(&[search, detail]));
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
    assert!(m.categories.contains(&mlm_db::Category::Romance));
    assert!(!m.categories.contains(&mlm_db::Category::CharacterDriven));
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

    let search = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "id": 30, "title": "Any Title", "description": "only desc" } }
    ] } } } }"#;
    let detail = r#"{ "data": { "books_by_pk": {
        "id": 30,
        "title": "Any Title",
        "subtitle": null,
        "headline": null,
        "description": "only desc",
        "contributions": [],
        "book_series": [],
        "taggings": []
    } } }"#;
    let client = Arc::new(MockClient::new_many(&[search, detail]));
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

    let search = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "id": 40, "title": "Multi ISBN", "author_names": ["A"], "isbns": ["FIRSTISBN","SECONDISBN"] } }
    ] } } } }"#;
    let detail = r#"{ "data": { "books_by_pk": {
        "id": 40,
        "title": "Multi ISBN",
        "subtitle": null,
        "headline": null,
        "description": "",
        "contributions": [{ "author": { "name": "A" }, "contribution": null }],
        "book_series": [],
        "taggings": []
    } } }"#;
    let client = Arc::new(MockClient::new_many(&[search, detail]));
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

    let search = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "id": 50, "title": "Tie Book", "author_names": ["Author One"], "description": "first" } },
        { "document": { "id": 51, "title": "Tie Book", "author_names": ["Author One"], "description": "second" } }
    ] } } } }"#;
    let detail = r#"{ "data": { "books_by_pk": {
        "id": 50,
        "title": "Tie Book",
        "subtitle": null,
        "headline": null,
        "description": "first",
        "contributions": [{ "author": { "name": "Author One" }, "contribution": null }],
        "book_series": [],
        "taggings": []
    } } }"#;

    let client = Arc::new(MockClient::new_many(&[search, detail]));
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

    let search = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "id": 60, "title": "Great Adventure", "author_names": ["Bob Smith"], "description": "B" } }
    ] } } } }"#;
    let detail = r#"{ "data": { "books_by_pk": {
        "id": 60,
        "title": "Great Adventure",
        "subtitle": null,
        "headline": null,
        "description": "B",
        "contributions": [{ "author": { "name": "Bob Smith" }, "contribution": null }],
        "book_series": [],
        "taggings": []
    } } }"#;

    let client = Arc::new(MockClient::new_many(&[search, detail]));
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
    let detail = r#"{ "data": { "books_by_pk": {
        "id": 123,
        "title": "Detailed Book",
        "subtitle": null,
        "headline": null,
        "description": "short desc",
        "contributions": [{ "author": { "name": "Detail Author" }, "contribution": null }],
        "book_series": [{ "position": 1, "details": "1", "series": { "name": "Series A" } }],
        "taggings": []
    } } }"#;

    let client = Arc::new(MockClient::new_many(&[search, detail]));
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
    let search = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "id": 70, "title": "Not the Boss of the Year", "author_names": ["J.S. Cooper"], "description": "wrong author" } },
        { "document": { "id": 71, "title": "Boss of the Year", "author_names": ["Nicole French"], "description": "correct" } }
    ] } } } }"#;
    let detail = r#"{ "data": { "books_by_pk": {
        "id": 71,
        "title": "Boss of the Year",
        "subtitle": null,
        "headline": null,
        "description": "correct",
        "contributions": [{ "author": { "name": "Nicole French" }, "contribution": null }],
        "book_series": [],
        "taggings": []
    } } }"#;

    let client = Arc::new(MockClient::new_many(&[search, detail]));
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

#[tokio::test]
async fn hardcover_prefers_best_matching_edition_and_edition_specific_fields() {
    use helper::MockClient;

    let search = r#"{ "data": { "search": { "results": { "hits": [
        { "document": { "id": 1421303, "title": "Quicksilver", "author_names": ["Callie Hart"], "isbns": ["9781399745420"] } }
    ] } } } }"#;

    let detail = r#"{ "data": { "books_by_pk": {
        "id": 1421303,
        "title": "Quicksilver",
        "subtitle": null,
        "headline": null,
        "description": "Book description",
        "contributions": [{ "author": { "name": "Callie Hart" }, "contribution": null }],
        "book_series": [],
        "taggings": [],
        "editions": [
          {
            "language": { "language": "English" },
            "asin": null,
            "isbn_10": "1399745425",
            "isbn_13": "9781399745420",
            "edition_format": "Paperback",
            "contributions": [{ "contribution": null, "author": { "name": "Callie Hart" } }]
          },
          {
            "language": { "language": "English" },
            "asin": "B0DBJBFHGT",
            "isbn_10": null,
            "isbn_13": null,
            "edition_format": "Audible",
            "contributions": [
              { "contribution": null, "author": { "name": "Callie Hart" } },
              { "contribution": "Narrator", "author": { "name": "Stella Bloom" } }
            ]
          }
        ]
    } } }"#;

    let client = Arc::new(MockClient::new_many(&[search, detail]));
    let prov = Hardcover::with_client("http://example/graphql", client, None);

    let query_meta = TorrentMeta {
        title: "Quicksilver".to_string(),
        authors: vec!["Callie Hart".to_string()],
        media_type: mlm_db::MediaType::Audiobook,
        language: Some(mlm_db::Language::English),
        ..Default::default()
    };

    let m = prov
        .fetch(&query_meta)
        .await
        .expect("should choose audible edition");

    assert_eq!(
        m.ids.get(mlm_db::ids::ASIN).map(|s| s.as_str()),
        Some("B0DBJBFHGT")
    );
    assert_eq!(m.media_type, mlm_db::MediaType::Audiobook);
    assert_eq!(m.language, Some(mlm_db::Language::English));
    assert!(m.narrators.iter().any(|n| n == "Stella Bloom"));
}
