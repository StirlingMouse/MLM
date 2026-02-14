pub mod fake;
pub mod hardcover;
pub mod romanceio;

pub use fake::FakeProvider;
pub use hardcover::Hardcover;
pub use romanceio::RomanceIo;

use crate::helpers::SearchQuery;
use anyhow::Result;
use mlm_db::TorrentMeta;

/// Metadata provider trait for searching and fetching book metadata.
/// Implement this trait to add a new provider.
#[allow(async_fn_in_trait)]
pub trait MetadataProvider: Send + Sync {
    /// Provider's search result type (e.g., serde_json::Value for JSON APIs)
    type SearchResult;

    /// Unique identifier for this provider (e.g., "hardcover", "romanceio")
    fn id(&self) -> &str;

    /// Minimum score threshold for accepting a match. Default 0.5.
    fn min_score_threshold(&self) -> f64 {
        0.5
    }

    /// Perform a search query. Receives title and optional author.
    async fn search(&self, query: &SearchQuery) -> Result<Vec<Self::SearchResult>>;

    /// Extract title from a search result
    fn result_title<'a>(&self, result: &'a Self::SearchResult) -> Option<&'a str>;

    /// Extract authors from a search result
    fn result_authors(&self, result: &Self::SearchResult) -> Vec<String>;

    /// Convert a search result to TorrentMeta. May fetch additional data (e.g., romanceio).
    async fn result_to_meta(&self, result: &Self::SearchResult) -> Result<TorrentMeta>;
}

/// `search_query` - the query sent to the provider (may have no author for title-only fallback)
/// `scoring_query` - the query used for scoring (always includes author if provided)
fn select_best<P: MetadataProvider>(
    provider: &P,
    results: &[P::SearchResult],
    _search_query: &SearchQuery,
    scoring_query: &SearchQuery,
    threshold: f64,
) -> Result<Option<(usize, f64)>> {
    let q_title = Some(scoring_query.title.clone());
    let q_auths = scoring_query.author.iter().cloned().collect::<Vec<_>>();

    let mut best_idx: Option<usize> = None;
    let mut best_score = -1.0f64;

    for (i, item) in results.iter().enumerate() {
        let title = provider.result_title(item);
        let authors = provider.result_authors(item);

        let score = crate::helpers::score_candidate(title, &authors, &q_title, &q_auths);

        if score > best_score {
            best_score = score;
            best_idx = Some(i);
        }
    }

    if best_score >= threshold {
        Ok(best_idx.map(|idx| (idx, best_score)))
    } else {
        Ok(None)
    }
}

/// Run a search with fallback: try title+author first, then title-only if needed.
/// Returns the matched metadata and score if found above threshold.
pub async fn search_with_fallback<P: MetadataProvider>(
    provider: &P,
    title: &str,
    authors: &[String],
) -> Result<(TorrentMeta, f64)> {
    if title.trim().is_empty() {
        return Err(anyhow::anyhow!("title is required for search"));
    }

    let threshold = provider.min_score_threshold();

    // Build queries
    let q_with_author = crate::helpers::query_with_author(title, authors);
    let q_title_only = crate::helpers::query_title_only(title);

    // If we have authors, try with author first
    let tried_with_author = if q_with_author.author.is_some() {
        match provider.search(&q_with_author).await {
            Ok(results) => {
                if !results.is_empty()
                    && let Some((idx, score)) = select_best(
                        provider,
                        &results,
                        &q_with_author,
                        &q_with_author,
                        threshold,
                    )?
                {
                    let meta = provider.result_to_meta(&results[idx]).await?;
                    return Ok((meta, score));
                }
            }
            Err(e) => {
                tracing::warn!("search with author failed: {}", e);
            }
        }
        true
    } else {
        false
    };

    // If authors was provided but didn't yield results above threshold, try title-only
    // Or if no authors were provided, do title-only search
    if (!tried_with_author || !authors.is_empty()) && !q_title_only.title.is_empty() {
        match provider.search(&q_title_only).await {
            Ok(results) => {
                if !results.is_empty()
                    && let Some((idx, score)) =
                        select_best(provider, &results, &q_title_only, &q_with_author, threshold)?
                {
                    let meta = provider.result_to_meta(&results[idx]).await?;
                    return Ok((meta, score));
                }
            }
            Err(e) => {
                tracing::warn!("title-only search failed: {}", e);
            }
        }
    }

    Err(anyhow::anyhow!("no result above score threshold"))
}
