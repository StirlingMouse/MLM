use crate::traits::Provider;
use anyhow::Result;
use async_trait::async_trait;
use mlm_db::TorrentMeta;

pub struct FakeProvider {
    pub id_str: String,
    pub meta: Option<TorrentMeta>,
}

impl FakeProvider {
    pub fn new(id: &str, meta: Option<TorrentMeta>) -> Self {
        Self {
            id_str: id.to_string(),
            meta,
        }
    }
}

#[async_trait]
impl Provider for FakeProvider {
    fn id(&self) -> &str {
        &self.id_str
    }

    async fn fetch(&self, _query: &TorrentMeta) -> Result<TorrentMeta> {
        match &self.meta {
            Some(m) => Ok(m.clone()),
            None => Err(anyhow::anyhow!("not found")),
        }
    }
}
