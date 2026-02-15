use crate::config::Config;
use crate::stats::{Events, Stats};
use std::sync::Arc;
use tokio::sync::Mutex;

pub trait Backend: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}

#[derive(Clone)]
pub struct Context {
    pub config: Arc<Mutex<Arc<Config>>>,
    pub stats: Stats,
    pub events: Events,
    pub backend: Option<Arc<dyn Backend>>,
    pub triggers: Triggers,
}

#[derive(Clone, Default)]
pub struct Triggers {
    pub search_tx: std::collections::BTreeMap<usize, tokio::sync::watch::Sender<()>>,
    pub import_tx: std::collections::BTreeMap<usize, tokio::sync::watch::Sender<()>>,
    pub torrent_linker_tx: Option<tokio::sync::watch::Sender<()>>,
    pub folder_linker_tx: Option<tokio::sync::watch::Sender<()>>,
    pub downloader_tx: Option<tokio::sync::watch::Sender<()>>,
    pub audiobookshelf_tx: Option<tokio::sync::watch::Sender<()>>,
}

impl Context {
    pub async fn config(&self) -> Arc<Config> {
        self.config.lock().await.clone()
    }
}
