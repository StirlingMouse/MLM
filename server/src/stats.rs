use std::{collections::BTreeMap, sync::Arc};

use anyhow::Result;
use mlm_db::Event;
use mlm_mam::api::MaM;
use native_db::Database;
use time::{OffsetDateTime, UtcDateTime};
use tokio::sync::{
    Mutex,
    watch::{self, Receiver, Sender},
};

use crate::config::Config;

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
pub struct Events {
    pub event: (Sender<Option<Event>>, Receiver<Option<Event>>),
}

#[derive(Clone)]
pub struct Triggers {
    pub search_tx: BTreeMap<usize, Sender<()>>,
    pub import_tx: BTreeMap<usize, Sender<()>>,
    pub torrent_linker_tx: Sender<()>,
    pub folder_linker_tx: Sender<()>,
    pub downloader_tx: Sender<()>,
    pub audiobookshelf_tx: Sender<()>,
}

#[derive(Clone)]
pub struct Context {
    pub config: Arc<Mutex<Arc<Config>>>,
    pub db: Arc<Database<'static>>,
    pub mam: Arc<Result<Arc<MaM<'static>>>>,
    pub stats: Stats,
    // pub events: Events,
    pub triggers: Triggers,
}

impl Context {
    pub async fn config(&self) -> Arc<Config> {
        self.config.lock().await.clone()
    }

    pub fn mam(&self) -> Result<Arc<MaM<'static>>> {
        let Ok(mam) = self.mam.as_ref() else {
            return Err(anyhow::Error::msg("mam_id error"));
        };

        Ok(mam.clone())
    }
}
