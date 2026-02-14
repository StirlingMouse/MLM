use anyhow::Result;
use mlm_meta::http::HttpClient;
use std::sync::Arc;

fn resolve_plan_file(rel: &str) -> std::io::Result<std::path::PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join(rel);
        if candidate.exists() {
            return Ok(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("could not find {}", rel),
    ))
}

pub struct MockClient;

#[async_trait::async_trait]
impl HttpClient for MockClient {
    async fn get(&self, url: &str) -> Result<String> {
        let u = url::Url::parse(url).map_err(|e| anyhow::anyhow!(e))?;
        let rel = if u.host_str().is_some_and(|h| h.contains("romance.io")) {
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

        let p = resolve_plan_file(rel).map_err(|e| anyhow::anyhow!(e))?;
        let s = std::fs::read_to_string(p).map_err(|e| anyhow::anyhow!(e))?;
        Ok(s)
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
    Arc::new(MockClient)
}
