use anyhow::Result;
use async_trait::async_trait;
use mlm_db::TorrentMeta;
use mlm_mam::api::MaM;

use crate::traits::Provider;

/// MaM metadata provider that can:
/// 1. Directly fetch by MaM ID if present in query.ids
/// 2. Search by title+author as fallback
pub struct MamProvider {
    mam: std::sync::Arc<MaM<'static>>,
}

impl MamProvider {
    pub fn new(mam: std::sync::Arc<MaM<'static>>) -> Self {
        Self { mam }
    }
}

#[async_trait]
impl Provider for MamProvider {
    fn id(&self) -> &str {
        "mam"
    }

    async fn fetch(&self, query: &TorrentMeta) -> Result<TorrentMeta> {
        // Priority 1: Direct ID lookup if mam_id is in query
        if let Some(mam_id) = query.mam_id() {
            tracing::debug!("MaM provider: attempting direct lookup for id {}", mam_id);
            if let Some(mam_torrent) = self.mam.get_torrent_info_by_id(mam_id).await? {
                let meta = mam_torrent.as_meta()?;
                tracing::debug!("MaM provider: direct lookup succeeded");
                return Ok(meta);
            }
            tracing::debug!(
                "MaM provider: direct lookup returned no result, falling back to search"
            );
        } else {
            tracing::debug!("MaM provider: no mam_id in query, using search");
        }

        // Priority 2: Search by title+author
        let search_text = if query.authors.is_empty() {
            query.title.clone()
        } else {
            let author_str = query
                .authors
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            format!("{} {}", query.title, author_str)
        };

        if search_text.trim().is_empty() {
            anyhow::bail!("MaM provider: title is required for search");
        }

        let results = self
            .mam
            .search(&mlm_mam::search::SearchQuery {
                perpage: 5,
                tor: mlm_mam::search::Tor {
                    text: search_text,
                    ..Default::default()
                },
                ..Default::default()
            })
            .await?;

        // Take the first result if available
        if let Some(first) = results.data.into_iter().next() {
            let mut torrent = first;
            torrent.fix();
            let meta = torrent.as_meta()?;
            tracing::debug!("MaM provider: search succeeded");
            return Ok(meta);
        }

        anyhow::bail!("MaM provider: no results found")
    }
}
