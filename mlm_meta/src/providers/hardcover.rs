use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{debug, instrument, warn};

use crate::providers::MetadataProvider;
use crate::traits::Provider;
use crate::{helpers, http::HttpClient, map_tag_to_category};
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

    fn result_id(result: &serde_json::Value) -> Option<i64> {
        result.get("id").and_then(|v| v.as_i64())
    }

    #[instrument(skip_all, fields(book_id = book_id))]
    async fn fetch_book_by_id(&self, book_id: i64) -> Result<serde_json::Value> {
        let gql = r#"
        query BookById($id: Int!) {
            books_by_pk(id: $id) {
                id
                title
                subtitle
                headline
                description
                pages
                images {
                    height
                    ratio
                    url
                }
                contributions {
                    author {
                        name
                    }
                    contributable_type
                    contribution
                }
                book_series {
                    position
                    details
                    series {
                        name
                    }
                }
                taggings(distinct_on: tag_id) {
                    id
                    spoiler
                    taggable_type
                    tag {
                        tag
                        tag_category {
                            category
                        }
                    }
                }
                editions {
                    language {
                        language
                    }
                    asin
                    isbn_10
                    isbn_13
                    edition_format
                    contributions {
                        contribution
                        author {
                            name
                        }
                    }
                }
            }
        }
        "#;

        let vars = serde_json::json!({ "id": book_id });
        let v = self.post_graphql(gql, vars).await?;
        let book = v
            .get("data")
            .and_then(|d| d.get("books_by_pk"))
            .cloned()
            .context("missing books_by_pk in hardcover response")?;
        Ok(book)
    }

    fn parse_series_entries(position: Option<f64>, details: Option<&str>) -> mlm_db::SeriesEntries {
        if let Some(pos) = position {
            return mlm_db::SeriesEntries::new(vec![mlm_db::SeriesEntry::Num(pos as f32)]);
        }

        if let Some(details) = details {
            let cleaned = details.trim();
            if let Ok(num) = cleaned.parse::<f32>() {
                return mlm_db::SeriesEntries::new(vec![mlm_db::SeriesEntry::Num(num)]);
            }
        }

        mlm_db::SeriesEntries::new(vec![])
    }

    fn normalize_identifier(value: &str) -> String {
        value
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_uppercase()
    }

    fn normalize_name(value: &str) -> String {
        value.trim().to_ascii_lowercase()
    }

    fn parse_contributions(
        contributions: Option<&Vec<serde_json::Value>>,
    ) -> (Vec<String>, Vec<String>) {
        let mut authors = Vec::new();
        let mut narrators = Vec::new();

        if let Some(contribs) = contributions {
            for c in contribs {
                let name = c
                    .get("author")
                    .and_then(|a| a.get("name"))
                    .and_then(|n| n.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string);

                if let Some(name) = name {
                    let contribution = c
                        .get("contribution")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();

                    if contribution.contains("narrat") {
                        if !narrators.contains(&name) {
                            narrators.push(name);
                        }
                    } else if (contribution.is_empty() || contribution.contains("author"))
                        && !authors.contains(&name)
                    {
                        authors.push(name);
                    }
                }
            }
        }

        (authors, narrators)
    }

    fn edition_language(edition: &serde_json::Value) -> Option<mlm_db::Language> {
        edition
            .get("language")
            .and_then(|l| l.get("language"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<mlm_db::Language>().ok())
    }

    fn edition_isbn(edition: &serde_json::Value) -> Option<String> {
        edition
            .get("isbn_13")
            .and_then(|v| v.as_str())
            .or_else(|| edition.get("isbn_10").and_then(|v| v.as_str()))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
    }

    fn edition_asin(edition: &serde_json::Value) -> Option<String> {
        edition
            .get("asin")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
    }

    fn edition_format(edition: &serde_json::Value) -> Option<String> {
        edition
            .get("edition_format")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
    }

    fn is_audiobook_format(fmt: &str) -> bool {
        let f = fmt.to_ascii_lowercase();
        f.contains("audio") || f.contains("audible") || f.contains("hör")
    }

    fn is_ebook_format(fmt: &str) -> bool {
        let f = fmt.to_ascii_lowercase();
        f.contains("ebook") || f.contains("e-book") || f.contains("epub") || f.contains("kindle")
    }

    fn score_edition(
        edition: &serde_json::Value,
        result: &serde_json::Value,
        query: Option<&TorrentMeta>,
    ) -> i32 {
        let mut score = 0_i32;

        let query_isbn = query
            .and_then(|q| q.ids.get(mlm_db::ids::ISBN))
            .map(|s| Self::normalize_identifier(s));
        let search_isbn = result
            .get("isbns")
            .and_then(|i| i.as_array())
            .and_then(|arr| arr.iter().filter_map(|v| v.as_str()).next())
            .map(Self::normalize_identifier);

        if let Some(query_isbn) = query_isbn
            && let Some(edition_isbn) = Self::edition_isbn(edition)
            && Self::normalize_identifier(&edition_isbn) == query_isbn
        {
            score += 140;
        } else if let Some(search_isbn) = search_isbn
            && let Some(edition_isbn) = Self::edition_isbn(edition)
            && Self::normalize_identifier(&edition_isbn) == search_isbn
        {
            score += 20;
        }

        if let Some(query_asin) = query
            .and_then(|q| q.ids.get(mlm_db::ids::ASIN))
            .map(|s| Self::normalize_identifier(s))
            && let Some(edition_asin) = Self::edition_asin(edition)
            && Self::normalize_identifier(&edition_asin) == query_asin
        {
            score += 140;
        }

        if let Some(query_lang) = query.and_then(|q| q.language)
            && let Some(edition_lang) = Self::edition_language(edition)
        {
            if edition_lang == query_lang {
                score += 30;
            } else {
                score -= 10;
            }
        }

        if let Some(query) = query {
            let format = Self::edition_format(edition);
            match query.media_type {
                mlm_db::MediaType::Audiobook | mlm_db::MediaType::PeriodicalAudiobook => {
                    if let Some(format) = format {
                        if Self::is_audiobook_format(&format) {
                            score += 25;
                        } else {
                            score -= 8;
                        }
                    }
                }
                mlm_db::MediaType::Ebook | mlm_db::MediaType::PeriodicalEbook => {
                    if let Some(format) = format {
                        if Self::is_ebook_format(&format) {
                            score += 25;
                        } else if Self::is_audiobook_format(&format) {
                            score -= 8;
                        }
                    }
                }
                _ => {}
            }
        }

        let query_authors = query
            .map(|q| {
                q.authors
                    .iter()
                    .map(|a| Self::normalize_name(a))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !query_authors.is_empty() {
            let (edition_authors, _) =
                Self::parse_contributions(edition.get("contributions").and_then(|c| c.as_array()));
            let edition_author_names = edition_authors
                .iter()
                .map(|a| Self::normalize_name(a))
                .collect::<Vec<_>>();

            if edition_author_names
                .iter()
                .any(|a| query_authors.iter().any(|q| q == a))
            {
                score += 20;
            }
        }

        score
    }

    fn select_best_edition<'a>(
        editions: &'a [serde_json::Value],
        result: &serde_json::Value,
        query: Option<&TorrentMeta>,
    ) -> Option<&'a serde_json::Value> {
        editions
            .iter()
            .enumerate()
            .max_by_key(|(idx, edition)| {
                (Self::score_edition(edition, result, query), -(*idx as i32))
            })
            .map(|(_, edition)| edition)
    }

    fn score_result(
        &self,
        result: &serde_json::Value,
        scoring_query: &helpers::SearchQuery,
    ) -> f64 {
        let q_title = Some(scoring_query.title.clone());
        let q_auths = scoring_query.author.iter().cloned().collect::<Vec<_>>();
        crate::helpers::score_candidate(
            self.result_title(result),
            &self.result_authors(result),
            &q_title,
            &q_auths,
        )
    }

    fn select_best_result(
        &self,
        results: &[serde_json::Value],
        scoring_query: &helpers::SearchQuery,
        threshold: f64,
    ) -> Option<(usize, f64)> {
        let mut best_idx = None;
        let mut best_score = -1.0_f64;
        for (i, item) in results.iter().enumerate() {
            let score = self.score_result(item, scoring_query);
            if score > best_score {
                best_score = score;
                best_idx = Some(i);
            }
        }

        if best_score >= threshold {
            best_idx.map(|idx| (idx, best_score))
        } else {
            None
        }
    }

    async fn search_best_result(
        &self,
        title: &str,
        authors: &[String],
    ) -> Result<(serde_json::Value, f64)> {
        if title.trim().is_empty() {
            return Err(anyhow::anyhow!("title is required for search"));
        }

        let threshold = self.min_score_threshold();
        let q_with_author = helpers::query_with_author(title, authors);
        let q_title_only = helpers::query_title_only(title);

        let tried_with_author = if q_with_author.author.is_some() {
            match self.search(&q_with_author).await {
                Ok(results) => {
                    if !results.is_empty()
                        && let Some((idx, score)) =
                            self.select_best_result(&results, &q_with_author, threshold)
                    {
                        return Ok((results[idx].clone(), score));
                    }
                }
                Err(e) => warn!("hardcover search with author failed: {e}"),
            }
            true
        } else {
            false
        };

        if (!tried_with_author || !authors.is_empty()) && !q_title_only.title.is_empty() {
            match self.search(&q_title_only).await {
                Ok(results) => {
                    if !results.is_empty()
                        && let Some((idx, score)) =
                            self.select_best_result(&results, &q_with_author, threshold)
                    {
                        return Ok((results[idx].clone(), score));
                    }
                }
                Err(e) => warn!("hardcover title-only search failed: {e}"),
            }
        }

        Err(anyhow::anyhow!("no result above score threshold"))
    }

    async fn result_to_meta_with_query(
        &self,
        result: &serde_json::Value,
        query: Option<&TorrentMeta>,
    ) -> Result<TorrentMeta> {
        let id = Self::result_id(result).context("missing hardcover result id")?;
        let book = self.fetch_book_by_id(id).await?;

        let title = book
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let subtitle = book
            .get("subtitle")
            .and_then(|s| s.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        let headline = book
            .get("headline")
            .and_then(|h| h.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        let body = book
            .get("description")
            .and_then(|d| d.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        let mut description_parts = Vec::new();
        if let Some(h) = headline {
            description_parts.push(h);
        }
        if let Some(b) = body {
            description_parts.push(b);
        }
        let description = description_parts.join("\n\n");

        let (book_authors, book_narrators) =
            Self::parse_contributions(book.get("contributions").and_then(|c| c.as_array()));

        let selected_edition = book
            .get("editions")
            .and_then(|e| e.as_array())
            .and_then(|editions| Self::select_best_edition(editions, result, query));

        let (authors, narrators) = if let Some(edition) = selected_edition {
            let (edition_authors, edition_narrators) =
                Self::parse_contributions(edition.get("contributions").and_then(|c| c.as_array()));

            let authors = if edition_authors.is_empty() {
                book_authors.clone()
            } else {
                edition_authors
            };
            let narrators = if edition_narrators.is_empty() {
                book_narrators.clone()
            } else {
                edition_narrators
            };

            (authors, narrators)
        } else {
            (book_authors, book_narrators)
        };

        let mut tm = TorrentMeta {
            title: subtitle
                .map(|s| format!("{title}: {s}"))
                .unwrap_or_else(|| title.clone()),
            description,
            authors,
            narrators,
            media_type: query.map(|q| q.media_type).unwrap_or_default(),
            language: query.and_then(|q| q.language),
            ..Default::default()
        };

        let mut tags = Vec::new();
        let mut categories = Vec::new();
        if let Some(taggings) = book.get("taggings").and_then(|t| t.as_array()) {
            for tagging in taggings {
                let tag_text = tagging
                    .get("tag")
                    .and_then(|t| t.get("tag"))
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty());

                let Some(tag_text) = tag_text else {
                    continue;
                };

                let normalized_tag = tag_text.to_ascii_lowercase();
                if !normalized_tag.is_empty() && !tags.contains(&normalized_tag) {
                    tags.push(normalized_tag);
                }

                let tag_category = tagging
                    .get("tag")
                    .and_then(|t| t.get("tag_category"))
                    .and_then(|tc| tc.get("category"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_ascii_lowercase());

                if matches!(tag_category.as_deref(), Some("genre" | "mood")) {
                    for category in map_tag_to_category(tag_text) {
                        if !categories.contains(&category) {
                            categories.push(category);
                        }
                    }
                }
            }
        }
        tm.tags = tags;
        tm.categories = categories;
        tm.ids.insert("hardcover".to_string(), id.to_string());

        if let Some(edition) = selected_edition {
            if let Some(lang) = Self::edition_language(edition) {
                tm.language = Some(lang);
            }
            if let Some(asin) = Self::edition_asin(edition) {
                tm.ids.insert(mlm_db::ids::ASIN.to_string(), asin);
            }
            if let Some(isbn) = Self::edition_isbn(edition) {
                tm.ids.insert(mlm_db::ids::ISBN.to_string(), isbn);
            }
            if let Some(format) = Self::edition_format(edition) {
                let lower = format.to_ascii_lowercase();
                if Self::is_audiobook_format(&lower) {
                    tm.media_type = mlm_db::MediaType::Audiobook;
                } else if Self::is_ebook_format(&lower) {
                    tm.media_type = mlm_db::MediaType::Ebook;
                }

                let (_t, ed_parsed) = parse_edition(&tm.title, &format);
                if ed_parsed.is_some() {
                    tm.edition = ed_parsed;
                }
            }
        }

        // Fallback ISBN support from search payload when edition doesn't provide one.
        if !tm.ids.contains_key(mlm_db::ids::ISBN)
            && let Some(isbns_arr) = result.get("isbns").and_then(|i| i.as_array())
            && let Some(first) = isbns_arr.iter().filter_map(|v| v.as_str()).next()
        {
            let s = first.trim().to_string();
            if !s.is_empty() {
                tm.ids.insert(mlm_db::ids::ISBN.to_string(), s);
            }
        }

        // Legacy edition fallback from selected search document.
        if tm.edition.is_none()
            && let Some(ed_str) = result
                .get("edition")
                .and_then(|v| v.as_str())
                .or(result.get("edition_string").and_then(|v| v.as_str()))
        {
            let (_t, ed_parsed) = parse_edition(&tm.title, ed_str);
            if ed_parsed.is_some() {
                tm.edition = ed_parsed;
            }
        }

        if let Some(series_arr) = book.get("book_series").and_then(|v| v.as_array()) {
            for s in series_arr {
                let name = s
                    .get("series")
                    .and_then(|series| series.get("name"))
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|n| !n.is_empty());

                if let Some(name) = name {
                    let position = s.get("position").and_then(|v| v.as_f64());
                    let details = s.get("details").and_then(|v| v.as_str());
                    tm.series.push(mlm_db::Series {
                        name: name.to_string(),
                        entries: Self::parse_series_entries(position, details),
                    });
                }
            }
        }

        debug!(
            title = %tm.title,
            authors = ?tm.authors,
            language = ?tm.language,
            tags_count = tm.tags.len(),
            categories_count = tm.categories.len(),
            "returning hardcover metadata"
        );
        Ok(tm)
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
        self.result_to_meta_with_query(result, None).await
    }
}

#[async_trait]
impl Provider for Hardcover {
    fn id(&self) -> &str {
        MetadataProvider::id(self)
    }

    async fn fetch(&self, query: &TorrentMeta) -> Result<TorrentMeta> {
        let (best_result, _score) = self
            .search_best_result(&query.title, &query.authors)
            .await?;
        let meta = self
            .result_to_meta_with_query(&best_result, Some(query))
            .await?;
        Ok(meta)
    }
}
