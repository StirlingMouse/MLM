pub mod audiobookshelf;
pub mod autograbber;
pub mod cleaner;
pub mod config;
pub mod config_impl;
pub mod context;
pub mod exporter;
pub mod linker;
pub mod lists;
pub mod logging;
pub mod metadata;
pub mod qbittorrent;
pub mod runner;
pub mod snatchlist;
pub mod stats;
pub mod torrent_downloader;

pub use crate::config::Config;
pub use crate::context::{Backend, Context, Triggers};
pub use crate::stats::{Events, Stats, StatsValues};

// Re-export types from mlm_db for convenience
pub use mlm_db::{
    ClientStatus, Event, EventKey, EventType, Flags, Language, LibraryMismatch, MetadataSource,
    OldCategory, Timestamp, Torrent, TorrentCost, TorrentKey, ids,
};

pub struct SsrBackend {
    pub db: std::sync::Arc<native_db::Database<'static>>,
    pub mam: std::sync::Arc<anyhow::Result<std::sync::Arc<mlm_mam::api::MaM<'static>>>>,
    pub metadata: std::sync::Arc<metadata::MetadataService>,
}

impl Backend for SsrBackend {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub trait ContextExt {
    fn ssr(&self) -> &SsrBackend;
    fn db(&self) -> &std::sync::Arc<native_db::Database<'static>>;
    fn mam(&self) -> anyhow::Result<std::sync::Arc<mlm_mam::api::MaM<'static>>>;
    fn metadata(&self) -> &std::sync::Arc<metadata::MetadataService>;
}

impl ContextExt for Context {
    fn ssr(&self) -> &SsrBackend {
        self.backend
            .as_ref()
            .expect("Backend not initialized")
            .as_any()
            .downcast_ref::<SsrBackend>()
            .expect("Failed to downcast to SsrBackend")
    }

    fn db(&self) -> &std::sync::Arc<native_db::Database<'static>> {
        &self.ssr().db
    }

    fn mam(&self) -> anyhow::Result<std::sync::Arc<mlm_mam::api::MaM<'static>>> {
        let mam = self.ssr().mam.as_ref();
        match mam {
            Ok(m) => Ok(m.clone()),
            Err(_) => Err(anyhow::anyhow!("mam_id error")),
        }
    }

    fn metadata(&self) -> &std::sync::Arc<metadata::MetadataService> {
        &self.ssr().metadata
    }
}
