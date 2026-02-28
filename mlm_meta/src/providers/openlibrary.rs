use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{debug, instrument};
use url::Url;

use crate::http::ReqwestClient;
use crate::providers::{MetadataProvider, search_with_fallback};
use crate::traits::Provider;
use crate::{helpers, http::HttpClient};
use mlm_db::TorrentMeta;

pub struct OpenLibrary {
    pub client: Arc<dyn HttpClient>,
}

impl OpenLibrary {
    pub fn new() -> Self {
        Self {
            client: Arc::new(ReqwestClient::new()),
        }
    }

    pub fn with_client(client: Arc<dyn HttpClient>) -> Self {
        Self { client }
    }

    #[instrument(skip_all, fields(url = %url))]
    async fn fetch_json(&self, url: &str) -> Result<String> {
        debug!("fetching Open Library JSON");
        self.client.get(url).await
    }
}

impl Default for OpenLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataProvider for OpenLibrary {
    type SearchResult = serde_json::Value;

    fn id(&self) -> &str {
        "openlibrary"
    }

    async fn search(&self, query: &helpers::SearchQuery) -> Result<Vec<Self::SearchResult>> {
        let base = Url::parse("https://openlibrary.org").unwrap();
        let qstr = query.to_combined_string();

        let mut search_url = base.join("/search.json").unwrap();
        if let Some(ref author) = query.author {
            if !author.is_empty() && !qstr.is_empty() {
                search_url.query_pairs_mut().append_pair("q", &qstr);
            }
        } else if !qstr.is_empty() {
            search_url.query_pairs_mut().append_pair("q", &qstr);
        }

        let url = search_url.to_string();
        debug!(query = %qstr, url = %url, "searching Open Library");

        let body = self.fetch_json(&url).await.context("fetch search json")?;
        let v: serde_json::Value = serde_json::from_str(&body).context("parse search json")?;

        let docs = v.get("docs").and_then(|d| d.as_array()).cloned();
        debug!(
            count = docs.as_ref().map(|a| a.len()).unwrap_or(0),
            "Open Library search results"
        );
        Ok(docs.unwrap_or_default())
    }

    fn result_title<'a>(&self, result: &'a Self::SearchResult) -> Option<&'a str> {
        result.get("title").and_then(|t| t.as_str())
    }

    fn result_authors(&self, result: &Self::SearchResult) -> Vec<String> {
        result
            .get("author_name")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| a.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn result_to_meta(&self, result: &Self::SearchResult) -> Result<TorrentMeta> {
        let title = result
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let authors = self.result_authors(result);

        let first_publish_year = result
            .get("first_publish_year")
            .and_then(|y| y.as_i64())
            .map(|y| y.to_string());

        let edition_count = result
            .get("edition_count")
            .and_then(|e| e.as_i64())
            .map(|e| e as u32);

        let subjects: Vec<String> = result
            .get("subject")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str())
                    .filter(|s| s.len() > 2 && s.len() < 50)
                    .take(20)
                    .map(|s| s.to_lowercase())
                    .collect()
            })
            .unwrap_or_default();

        let mut tm = TorrentMeta {
            title: title.clone(),
            description: String::new(),
            authors: authors.clone(),
            ..Default::default()
        };

        if let Some(year) = first_publish_year {
            tm.description
                .push_str(&format!("First published: {}\n", year));
        }
        if let Some(count) = edition_count {
            tm.description.push_str(&format!("{} editions\n", count));
        }

        tm.tags = subjects;

        if let Some(isbns) = result.get("isbn").and_then(|i| i.as_array()) {
            for isbn in isbns.iter().take(3) {
                if let Some(isbn_str) = isbn.as_str() {
                    tm.ids
                        .insert(mlm_db::ids::ISBN.to_string(), isbn_str.to_string());
                    break;
                }
            }
        }

        debug!(title = %tm.title, authors = ?tm.authors, tags_count = tm.tags.len(), "returning Open Library metadata");
        Ok(tm)
    }
}

#[async_trait]
impl Provider for OpenLibrary {
    fn id(&self) -> &str {
        MetadataProvider::id(self)
    }

    async fn fetch(&self, query: &TorrentMeta) -> Result<TorrentMeta> {
        let (meta, _score) = search_with_fallback(self, &query.title, &query.authors).await?;
        Ok(meta)
    }
}
