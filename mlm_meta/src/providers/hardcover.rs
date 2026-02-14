use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{debug, instrument};

use crate::providers::{MetadataProvider, search_with_fallback};
use crate::traits::Provider;
use crate::{helpers, http::HttpClient};
use mlm_db::TorrentMeta;
use mlm_parse::parse_edition;

use std::sync::Arc;

const DEFAULT_ENDPOINT: &str = "https://api.hardcover.app/v1/graphql";

pub struct Hardcover {
    endpoint: String,
    client: Arc<dyn HttpClient>,
    api_key: Option<String>,
}

impl Hardcover {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            endpoint: DEFAULT_ENDPOINT.to_string(),
            client: Arc::new(crate::http::ReqwestClient::new()),
            api_key,
        }
    }

    pub fn with_client(
        endpoint: &str,
        client: Arc<dyn HttpClient>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            client,
            api_key,
        }
    }

    #[instrument(skip_all, fields(query = %query))]
    async fn post_graphql(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let body_v = serde_json::json!({ "query": query, "variables": variables });
        let body = serde_json::to_string(&body_v)?;
        debug!(url = %self.endpoint, "posting GraphQL request");

        let headers = if let Some(ref key) = self.api_key {
            vec![
                ("content-type", "application/json"),
                ("authorization", key.as_str()),
            ]
        } else {
            vec![("content-type", "application/json")]
        };

        let s = self
            .client
            .post(&self.endpoint, Some(&body), &headers)
            .await
            .context("post graphql")?;
        let v: serde_json::Value = serde_json::from_str(&s).context("parse graphql json")?;
        Ok(v)
    }

    fn parse_results(&self, v: &serde_json::Value) -> Vec<serde_json::Value> {
        let hits = v
            .get("data")
            .and_then(|d| d.get("search"))
            .and_then(|s| s.get("results"))
            .and_then(|r| r.get("hits"))
            .and_then(|h| h.as_array())
            .cloned()
            .unwrap_or_default();

        hits.iter()
            .filter_map(|hit| hit.get("document").cloned())
            .collect()
    }
}

impl Default for Hardcover {
    fn default() -> Self {
        Self::new(None)
    }
}

impl MetadataProvider for Hardcover {
    type SearchResult = serde_json::Value;

    fn id(&self) -> &str {
        "hardcover"
    }

    async fn search(&self, query: &helpers::SearchQuery) -> Result<Vec<Self::SearchResult>> {
        let gql = r#"
        query Search($q: String!, $type: String!, $per_page: Int, $page: Int) {
            search(query: $q, query_type: $type, per_page: $per_page, page: $page) {
                results
            }
        }
        "#;

        let qstr = query.to_combined_string();
        let vars = serde_json::json!({"q": qstr, "type": "Book", "per_page": 10, "page": 1});
        debug!(query = %qstr, "searching hardcover");
        let v = self.post_graphql(gql, vars).await?;
        let results = self.parse_results(&v);
        debug!(count = results.len(), "hardcover search results");
        Ok(results)
    }

    fn result_title<'a>(&self, result: &'a Self::SearchResult) -> Option<&'a str> {
        result.get("title")?.as_str()
    }

    fn result_authors(&self, result: &Self::SearchResult) -> Vec<String> {
        result
            .get("author_names")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
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
        let authors: Vec<String> = self.result_authors(result);
        let description = result
            .get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        let mut tm = TorrentMeta {
            title: title.clone(),
            description: description.clone().unwrap_or_default(),
            authors: authors.clone(),
            ..Default::default()
        };

        // tags/genres
        let mut tags = Vec::new();
        if let Some(tarr) = result.get("tags").and_then(|t| t.as_array()) {
            for t in tarr {
                if let Some(s) = t.as_str() {
                    let s = s.trim().to_lowercase();
                    if !s.is_empty() && !tags.contains(&s) {
                        tags.push(s);
                    }
                }
            }
        }
        if let Some(genres) = result.get("genres").and_then(|g| g.as_array()) {
            for g in genres {
                if let Some(s) = g.as_str() {
                    let s = s.trim().to_lowercase();
                    if !s.is_empty() && !tags.contains(&s) {
                        tags.push(s);
                    }
                }
            }
        }
        tm.tags = tags;

        // ISBNs
        if let Some(isbns_arr) = result.get("isbns").and_then(|i| i.as_array())
            && let Some(first) = isbns_arr.iter().filter_map(|v| v.as_str()).next()
        {
            let s = first.trim().to_string();
            if !s.is_empty() {
                tm.ids.insert(mlm_db::ids::ISBN.to_string(), s);
            }
        }

        // edition
        if let Some(ed_str) = result
            .get("edition")
            .and_then(|v| v.as_str())
            .or(result.get("edition_string").and_then(|v| v.as_str()))
        {
            let (_t, ed_parsed) = parse_edition(&tm.title, ed_str);
            if ed_parsed.is_some() {
                tm.edition = ed_parsed;
            }
        }

        // series
        if let Some(series_arr) = result.get("series_names").and_then(|v| v.as_array()) {
            for s in series_arr {
                if let Some(name) = s.as_str() {
                    tm.series.push(mlm_db::Series {
                        name: name.to_string(),
                        entries: mlm_db::SeriesEntries::new(vec![]),
                    });
                } else if let Some(obj) = s.as_object()
                    && let Some(name) = obj.get("name").and_then(|v| v.as_str())
                {
                    if let Some(idx) = obj.get("index").and_then(|v| v.as_f64()) {
                        let entry = mlm_db::SeriesEntry::Num(idx as f32);
                        tm.series.push(mlm_db::Series {
                            name: name.to_string(),
                            entries: mlm_db::SeriesEntries::new(vec![entry]),
                        });
                    } else {
                        tm.series.push(mlm_db::Series {
                            name: name.to_string(),
                            entries: mlm_db::SeriesEntries::new(vec![]),
                        });
                    }
                }
            }
        }

        debug!(title = %tm.title, authors = ?tm.authors, tags_count = tm.tags.len(), "returning hardcover metadata");
        Ok(tm)
    }
}

#[async_trait]
impl Provider for Hardcover {
    fn id(&self) -> &str {
        MetadataProvider::id(self)
    }

    async fn fetch(&self, query: &TorrentMeta) -> Result<TorrentMeta> {
        let (meta, _score) = search_with_fallback(self, &query.title, &query.authors).await?;
        Ok(meta)
    }
}
