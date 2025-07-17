use anyhow::Result;
use time::OffsetDateTime;
use tokio::sync::watch::Sender;

#[derive(Default)]
pub struct Stats {
    pub autograbber_run_at: Option<OffsetDateTime>,
    pub autograbber_result: Option<Result<()>>,
    pub linker_run_at: Option<OffsetDateTime>,
    pub linker_result: Option<Result<()>>,
    pub cleaner_run_at: Option<OffsetDateTime>,
    pub cleaner_result: Option<Result<()>>,
    pub goodreads_run_at: Option<OffsetDateTime>,
    pub goodreads_result: Option<Result<()>>,
    pub downloader_run_at: Option<OffsetDateTime>,
    pub downloader_result: Option<Result<()>>,
}

#[derive(Clone)]
pub struct Triggers {
    pub search_tx: Sender<()>,
    pub linker_tx: Sender<()>,
    pub goodreads_tx: Sender<()>,
    pub downloader_tx: Sender<()>,
}
