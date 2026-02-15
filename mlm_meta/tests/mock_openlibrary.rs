use anyhow::Result;
use mlm_meta::http::HttpClient;
use std::sync::Arc;

pub struct MockOpenLibraryClient;

#[async_trait::async_trait]
impl HttpClient for MockOpenLibraryClient {
    async fn get(&self, url: &str) -> Result<String> {
        let u = url::Url::parse(url).map_err(|e| anyhow::anyhow!(e))?;
        if !u.host_str().is_some_and(|h| h.contains("openlibrary.org")) {
            return Err(anyhow::anyhow!("unexpected host in test fetch"));
        }
        if !u.path().starts_with("/search.json") {
            return Err(anyhow::anyhow!("unexpected path: {}", u.path()));
        }

        Ok(r#"{
  "numFound": 1,
  "docs": [
    {
      "title": "The Lord of the Rings",
      "author_name": ["J.R.R. Tolkien"],
      "isbn": ["9780261102385", "0261102389"],
      "subject": ["Fantasy fiction", "Middle Earth", "Epic fantasy"],
      "first_publish_year": 1954,
      "edition_count": 120
    }
  ]
}"#
        .to_string())
    }

    async fn post(
        &self,
        _url: &str,
        _body: Option<&str>,
        _headers: &[(&str, &str)],
    ) -> Result<String> {
        Err(anyhow::anyhow!("post not implemented in mock"))
    }
}

pub fn boxed() -> Arc<dyn HttpClient> {
    Arc::new(MockOpenLibraryClient)
}
