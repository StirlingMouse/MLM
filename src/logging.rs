use std::fmt::Display;

use anyhow::{Error, Result};
use native_db::Database;
use tracing::{error, warn};

use crate::data::{
    ErroredTorrent, ErroredTorrentId, Event, EventType, Timestamp, TorrentMeta, Uuid,
};

#[derive(Debug)]
pub struct TorrentMetaError(pub TorrentMeta, pub anyhow::Error);
impl Display for TorrentMetaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.1.fmt(f)
    }
}
impl std::error::Error for TorrentMetaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.1.source()
    }
}

pub fn update_errored_torrent(
    db: &Database<'_>,
    id: ErroredTorrentId,
    torrent: String,
    result: Result<(), Error>,
) {
    let is_ok = result.is_ok();
    if let Err(err) = db.rw_transaction().and_then(|rw| {
        if let Err(err) = result {
            let name = match id {
                ErroredTorrentId::Grabber(_) => "Autograbber",
                ErroredTorrentId::Linker(_) => "Linker",
                ErroredTorrentId::Cleaner(_) => "Cleaner",
            };
            warn!("{name} error for {torrent}: {err}");
            let (err, meta) = match err.downcast::<TorrentMetaError>() {
                Ok(TorrentMetaError(meta, err)) => (err, Some(meta)),
                Err(err) => (err, None),
            };
            rw.upsert(ErroredTorrent {
                id,
                title: torrent,
                error: format!("{err:?}"),
                meta,
                created_at: Timestamp::now(),
            })?;
        } else if let Some(error) = rw.get().primary::<ErroredTorrent>(id)? {
            rw.remove(error)?;
        }
        rw.commit()
    }) {
        if is_ok {
            error!("Error clearing error from db: {err:?}");
        } else {
            error!("Error writing error to db: {err:?}");
        }
    }
}

impl Event {
    pub fn new(hash: Option<String>, mam_id: Option<u64>, event: EventType) -> Self {
        Self {
            id: Uuid::new(),
            hash,
            mam_id,
            created_at: Timestamp::now(),
            event,
        }
    }
}

pub fn write_event(db: &Database<'_>, event: Event) {
    if let Err(err) = db.rw_transaction().and_then(|rw| {
        rw.upsert(event.clone())?;
        rw.commit()
    }) {
        error!("Error writing event: {err:?}, event: {event:?}");
    }
}
