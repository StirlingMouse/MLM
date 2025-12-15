use std::{collections::BTreeMap, sync::Arc};

use anyhow::Result;
use time::{OffsetDateTime, UtcDateTime};
use tokio::sync::{
    Mutex,
    watch::{self, Receiver, Sender},
};

#[derive(Default)]
pub struct StatsValues {
    pub autograbber_run_at: BTreeMap<usize, OffsetDateTime>,
    pub autograbber_result: BTreeMap<usize, Result<()>>,
    pub linker_run_at: Option<OffsetDateTime>,
    pub linker_result: Option<Result<()>>,
    pub cleaner_run_at: Option<OffsetDateTime>,
    pub cleaner_result: Option<Result<()>>,
    pub goodreads_run_at: Option<OffsetDateTime>,
    pub goodreads_result: Option<Result<()>>,
    pub downloader_run_at: Option<OffsetDateTime>,
    pub downloader_result: Option<Result<()>>,
    pub audiobookshelf_run_at: Option<OffsetDateTime>,
    pub audiobookshelf_result: Option<Result<()>>,
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
        self.values_updated.0.send(UtcDateTime::now()).unwrap();
    }

    pub fn updates(&self) -> Receiver<UtcDateTime> {
        self.values_updated.1.clone()
    }
}

#[derive(Clone)]
pub struct Triggers {
    pub search_tx: BTreeMap<usize, Sender<()>>,
    pub linker_tx: Sender<()>,
    pub goodreads_tx: Sender<()>,
    pub downloader_tx: Sender<()>,
    pub audiobookshelf_tx: Sender<()>,
}
