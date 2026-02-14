use anyhow::Result;
use async_trait::async_trait;
use mlm_db::TorrentMeta;

/// Implementations should populate and return a `TorrentMeta` containing as
/// much normalized metadata as possible.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Short stable id for the provider, e.g. "goodreads"
    fn id(&self) -> &str;

    /// Fetch metadata for the given `TorrentMeta` query. Return Ok(TorrentMeta)
    /// on success.
    async fn fetch(&self, query: &TorrentMeta) -> Result<TorrentMeta>;
}
