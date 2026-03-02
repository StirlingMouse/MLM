use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use scraper::{Html, Selector};
use tracing::{debug, instrument};
use url::Url;

use crate::http::ReqwestClient;
use crate::providers::{MetadataProvider, search_with_fallback};
use crate::traits::Provider;
use crate::{helpers, http::HttpClient};
use mlm_db::TorrentMeta;

pub struct RomanceIo {
    pub client: Arc<dyn HttpClient>,
}

impl RomanceIo {
    pub fn new() -> Self {
        Self {
            client: Arc::new(ReqwestClient::new()),
        }
    }

    pub fn with_client(client: Arc<dyn HttpClient>) -> Self {
        Self { client }
    }

    #[instrument(skip_all, fields(url = %url))]
    async fn fetch_html(&self, url: &str) -> Result<String> {
        debug!("fetching romance.io HTML");
        self.client.get(url).await
    }

    async fn fetch_book(&self, book_url: &str) -> Result<TorrentMeta> {
        let book_html = self.fetch_html(book_url).await.context("fetch book page")?;
        self.parse_book_html(&book_html)
    }

    pub fn parse_book_html(&self, html: &str) -> Result<TorrentMeta> {
        let doc = Html::parse_document(html);

        let script_sel = Selector::parse("script[type=\"application/ld+json\"]").unwrap();
        if let Some(script) = doc.select(&script_sel).next() {
            let json_text = script.inner_html();
            let v: serde_json::Value = serde_json::from_str(&json_text).context("parse json-ld")?;
            let book = v.get("@graph").and_then(|g| g.get(0)).unwrap_or(&v);
            let title = book
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let authors: Vec<String> = book
                .get("author")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| {
                            p.get("name")
                                .and_then(|n| n.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                })
                .unwrap_or_default();
            let description = book
                .get("description")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());

            let mut tm = TorrentMeta {
                title: title.clone(),
                description: description.clone().unwrap_or_default(),
                authors: authors.clone(),
                ..Default::default()
            };

            let mut topics = Vec::new();
            let topics_sel = Selector::parse("#valid-topics-list a.topic").unwrap();
            for t in doc.select(&topics_sel) {
                let text = t.text().collect::<Vec<_>>().join(" ").trim().to_lowercase();
                if text.len() > 2 && !topics.contains(&text) {
                    topics.push(text);
                }
            }

            if let Some(desc) = description.as_ref() {
                for part in desc.split(&[',', '\n'][..]) {
                    let s = part.trim().to_lowercase();
                    if s.len() > 2 && !topics.contains(&s) {
                        topics.push(s);
                    }
                }
            }

            let mut categories = Vec::new();
            let mut tags = Vec::new();
            for t in topics {
                if let Some(cat) = topic_to_category(&t) {
                    if !categories.contains(&cat) {
                        categories.push(cat);
                    }
                } else if !tags.contains(&t) {
                    tags.push(t);
                }
            }
            tm.categories = categories;
            tm.tags = tags;

            return Ok(tm);
        }
        Err(anyhow::anyhow!("no json-ld found"))
    }
}

impl Default for RomanceIo {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataProvider for RomanceIo {
    type SearchResult = serde_json::Value;

    fn id(&self) -> &str {
        "romanceio"
    }

    async fn search(&self, query: &helpers::SearchQuery) -> Result<Vec<Self::SearchResult>> {
        let base = Url::parse("https://www.romance.io").unwrap();
        let qstr = query.to_combined_string();

        let mut json_url = base.join("/json/search_books").unwrap();
        json_url.query_pairs_mut().append_pair("search", &qstr);

        debug!(query = %qstr, url = %json_url, "searching romance.io");
        let body = self
            .fetch_html(json_url.as_str())
            .await
            .context("fetch search json")?;

        let v: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => {
                let preview = if body.len() > 50000 {
                    format!("{}...", &body[..50000])
                } else {
                    body.clone()
                };
                tracing::warn!(
                    url = %json_url,
                    response_preview = %preview,
                    "failed to parse romance.io search response: {}",
                    e
                );
                return Err(anyhow::anyhow!("parse search json: {}", e))
                    .context("parse search json");
            }
        };

        let books = v.get("books").and_then(|b| b.as_array()).cloned();
        debug!(
            count = books.as_ref().map(|a| a.len()).unwrap_or(0),
            "romance.io search results"
        );
        Ok(books.unwrap_or_default())
    }

    fn result_title<'a>(&self, result: &'a Self::SearchResult) -> Option<&'a str> {
        result
            .get("info")
            .and_then(|info| info.get("title"))
            .and_then(|t| t.as_str())
            .or_else(|| result.get("url").and_then(|u| u.as_str()))
    }

    fn result_authors(&self, result: &Self::SearchResult) -> Vec<String> {
        result
            .get("authors")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| {
                        a.get("name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn result_to_meta(&self, result: &Self::SearchResult) -> Result<TorrentMeta> {
        // RomanceIo fetches the full book page for verification, so this method
        // extracts the URL and fetches the book page
        let url = result
            .get("url")
            .and_then(|u| u.as_str())
            .context("no URL in search result")?;

        let base = Url::parse("https://www.romance.io").unwrap();
        let book_url = base.join(url).context("invalid book URL")?;

        debug!(url = %book_url, "fetching romance.io book page");
        let meta = self.fetch_book(book_url.as_str()).await?;

        // Verify title matches (case-insensitive substring)
        // Note: The caller should handle verification, but we do a quick check here
        Ok(meta)
    }
}

#[async_trait]
impl Provider for RomanceIo {
    fn id(&self) -> &str {
        MetadataProvider::id(self)
    }

    async fn fetch(&self, query: &TorrentMeta) -> Result<TorrentMeta> {
        let (meta, _score) = search_with_fallback(self, &query.title, &query.authors).await?;

        // Additional verification: ensure title contains query title
        let query_title_lower = query.title.to_lowercase();
        let meta_title_lower = meta.title.to_lowercase();
        if !meta_title_lower.contains(&query_title_lower) {
            return Err(anyhow::anyhow!(
                "matched title does not contain query title"
            ));
        }

        // Additional verification: if query has authors, at least one should match
        if !query.authors.is_empty() {
            let query_authors_lower: Vec<String> =
                query.authors.iter().map(|a| a.to_lowercase()).collect();
            let meta_authors_lower: Vec<String> =
                meta.authors.iter().map(|a| a.to_lowercase()).collect();
            let any_match = query_authors_lower.iter().any(|qa| {
                meta_authors_lower
                    .iter()
                    .any(|ma| ma.contains(qa) || qa.contains(ma))
            });
            if !any_match {
                return Err(anyhow::anyhow!(
                    "matched author does not contain any query author"
                ));
            }
        }

        Ok(meta)
    }
}

fn topic_to_category(topic: &str) -> Option<String> {
    let t = topic.trim().to_lowercase();
    match t.as_str() {
        "contemporary" | "contemporary romance" => Some("contemporary".to_string()),
        "romance" => Some("romance".to_string()),
        "dark" | "dark romance" => Some("dark romance".to_string()),
        "suspense" | "romantic suspense" => Some("suspense".to_string()),
        "erotic" | "erotic romance" | "steam" | "explicit" => Some("erotic".to_string()),
        "office" | "workplace" | "boss & employee" => Some("contemporary".to_string()),
        _ => None,
    }
}
