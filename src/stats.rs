use anyhow::Result;
use time::OffsetDateTime;

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
}
