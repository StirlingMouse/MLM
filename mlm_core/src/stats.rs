use anyhow::Result;
use std::{collections::BTreeMap, sync::Arc};
use time::{OffsetDateTime, UtcDateTime};
use tokio::sync::{
    Mutex,
    watch::{self, Receiver, Sender},
};

#[derive(Default)]
pub struct StatsValues {
    pub autograbber_run_at: BTreeMap<usize, OffsetDateTime>,
    pub autograbber_result: BTreeMap<usize, Result<()>>,
    pub import_run_at: BTreeMap<usize, OffsetDateTime>,
    pub import_result: BTreeMap<usize, Result<()>>,
    pub folder_linker_run_at: Option<OffsetDateTime>,
    pub folder_linker_result: Option<Result<()>>,
    pub torrent_linker_run_at: Option<OffsetDateTime>,
    pub torrent_linker_result: Option<Result<()>>,
    pub cleaner_run_at: Option<OffsetDateTime>,
    pub cleaner_result: Option<Result<()>>,
    pub downloader_run_at: Option<OffsetDateTime>,
    pub downloader_result: Option<Result<()>>,
    pub audiobookshelf_run_at: Option<OffsetDateTime>,
    pub audiobookshelf_result: Option<Result<()>>,
}

#[derive(Clone)]
pub struct Events {
    pub event: (
        Sender<Option<mlm_db::Event>>,
        Receiver<Option<mlm_db::Event>>,
    ),
}

impl Events {
    pub fn new() -> Self {
        Self {
            event: watch::channel(None),
        }
    }
}

impl Default for Events {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Stats {
    pub values: Arc<Mutex<StatsValues>>,
    values_updated: (Sender<UtcDateTime>, Receiver<UtcDateTime>),
}

impl Stats {
    pub fn new() -> Self {
        Self {
            values: Arc::new(Mutex::new(StatsValues::default())),
            values_updated: watch::channel(UtcDateTime::now()),
        }
    }

    pub async fn update(&self, f: impl FnOnce(&mut StatsValues)) {
        let mut data = self.values.lock().await;
        f(&mut data);
        let _ = self.values_updated.0.send(UtcDateTime::now());
    }

    pub fn updates(&self) -> Receiver<UtcDateTime> {
        self.values_updated.1.clone()
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}
