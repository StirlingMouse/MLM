pub mod impls;
mod v01;
mod v02;
mod v03;
mod v04;
mod v05;
mod v06;
mod v07;
mod v08;
mod v09;
mod v10;
mod v11;
mod v12;
mod v13;
mod v14;
mod v15;
mod v16;
mod v17;

use anyhow::Result;
use native_db::Models;
use native_db::transaction::RwTransaction;
use native_db::{Database, ToInput, db_type};
use once_cell::sync::Lazy;
use tokio::sync::MutexGuard;
use tracing::{info, instrument};

pub static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<v01::Config>().unwrap();

    models.define::<v17::Torrent>().unwrap();
    models.define::<v17::SelectedTorrent>().unwrap();
    models.define::<v17::DuplicateTorrent>().unwrap();
    models.define::<v17::ErroredTorrent>().unwrap();
    models.define::<v17::Event>().unwrap();

    models.define::<v16::Torrent>().unwrap();
    models.define::<v16::SelectedTorrent>().unwrap();
    models.define::<v16::DuplicateTorrent>().unwrap();
    models.define::<v16::ErroredTorrent>().unwrap();

    models.define::<v15::Torrent>().unwrap();
    models.define::<v15::SelectedTorrent>().unwrap();
    models.define::<v15::DuplicateTorrent>().unwrap();
    models.define::<v15::ErroredTorrent>().unwrap();
    models.define::<v15::Event>().unwrap();

    models.define::<v14::Torrent>().unwrap();
    models.define::<v14::SelectedTorrent>().unwrap();
    models.define::<v14::DuplicateTorrent>().unwrap();
    models.define::<v14::ErroredTorrent>().unwrap();

    models.define::<v13::Torrent>().unwrap();
    models.define::<v13::SelectedTorrent>().unwrap();
    models.define::<v13::DuplicateTorrent>().unwrap();
    models.define::<v13::ErroredTorrent>().unwrap();

    models.define::<v12::Torrent>().unwrap();
    models.define::<v12::SelectedTorrent>().unwrap();
    models.define::<v12::DuplicateTorrent>().unwrap();
    models.define::<v12::ErroredTorrent>().unwrap();
    models.define::<v12::Event>().unwrap();

    models.define::<v11::Torrent>().unwrap();
    models.define::<v11::SelectedTorrent>().unwrap();
    models.define::<v11::DuplicateTorrent>().unwrap();
    models.define::<v11::ErroredTorrent>().unwrap();
    models.define::<v11::Event>().unwrap();

    models.define::<v10::Torrent>().unwrap();
    models.define::<v10::SelectedTorrent>().unwrap();
    models.define::<v10::DuplicateTorrent>().unwrap();
    models.define::<v10::ErroredTorrent>().unwrap();
    models.define::<v10::Event>().unwrap();

    models.define::<v09::Torrent>().unwrap();
    models.define::<v09::SelectedTorrent>().unwrap();
    models.define::<v09::DuplicateTorrent>().unwrap();
    models.define::<v09::ErroredTorrent>().unwrap();

    models.define::<v08::Torrent>().unwrap();
    models.define::<v08::SelectedTorrent>().unwrap();
    models.define::<v08::DuplicateTorrent>().unwrap();
    models.define::<v08::ErroredTorrent>().unwrap();
    models.define::<v08::Event>().unwrap();

    models.define::<v07::Torrent>().unwrap();
    models.define::<v07::SelectedTorrent>().unwrap();
    models.define::<v07::DuplicateTorrent>().unwrap();
    models.define::<v07::Event>().unwrap();

    models.define::<v06::Torrent>().unwrap();
    models.define::<v06::SelectedTorrent>().unwrap();
    models.define::<v06::DuplicateTorrent>().unwrap();
    models.define::<v06::ErroredTorrent>().unwrap();

    models.define::<v05::Torrent>().unwrap();
    models.define::<v05::List>().unwrap();
    models.define::<v05::ListItem>().unwrap();

    models.define::<v04::SelectedTorrent>().unwrap();
    models.define::<v04::Event>().unwrap();
    models.define::<v04::List>().unwrap();
    models.define::<v04::ListItem>().unwrap();

    models.define::<v03::Torrent>().unwrap();
    models.define::<v03::SelectedTorrent>().unwrap();
    models.define::<v03::DuplicateTorrent>().unwrap();
    models.define::<v03::ErroredTorrent>().unwrap();
    models.define::<v03::Event>().unwrap();
    models.define::<v03::List>().unwrap();
    models.define::<v03::ListItem>().unwrap();

    models.define::<v02::Torrent>().unwrap();
    models.define::<v02::SelectedTorrent>().unwrap();
    models.define::<v02::DuplicateTorrent>().unwrap();
    models.define::<v02::ErroredTorrent>().unwrap();

    models.define::<v01::Torrent>().unwrap();
    models.define::<v01::SelectedTorrent>().unwrap();
    models.define::<v01::DuplicateTorrent>().unwrap();
    models.define::<v01::ErroredTorrent>().unwrap();

    models
});

pub type Config = v01::Config;
pub type Torrent = v17::Torrent;
pub type TorrentKey = v17::TorrentKey;
pub type SelectedTorrent = v17::SelectedTorrent;
pub type SelectedTorrentKey = v17::SelectedTorrentKey;
pub type DuplicateTorrent = v17::DuplicateTorrent;
pub type ErroredTorrent = v17::ErroredTorrent;
pub type ErroredTorrentKey = v17::ErroredTorrentKey;
pub type ErroredTorrentId = v11::ErroredTorrentId;
pub type Event = v17::Event;
pub type EventKey = v17::EventKey;
pub type EventType = v17::EventType;
pub type List = v05::List;
pub type ListKey = v05::ListKey;
pub type ListItem = v05::ListItem;
pub type ListItemKey = v05::ListItemKey;
pub type ListItemTorrent = v04::ListItemTorrent;
pub type TorrentMeta = v17::TorrentMeta;
pub type TorrentMetaDiff = v17::TorrentMetaDiff;
pub type TorrentMetaField = v17::TorrentMetaField;
pub type VipStatus = v11::VipStatus;
pub type MetadataSource = v10::MetadataSource;
pub type OldDbMainCat = v01::MainCat;
pub type MainCat = v12::MainCat;
pub type Uuid = v03::Uuid;
pub type Timestamp = v03::Timestamp;
pub type Series = v09::Series;
pub type SeriesEntries = v09::SeriesEntries;
pub type SeriesEntry = v09::SeriesEntry;
pub type Language = v03::Language;
pub type FlagBits = v08::FlagBits;
pub type Size = v03::Size;
pub type TorrentCost = v04::TorrentCost;
pub type TorrentStatus = v04::TorrentStatus;
pub type LibraryMismatch = v08::LibraryMismatch;
pub type ClientStatus = v08::ClientStatus;
pub type AudiobookCategory = v06::AudiobookCategory;
pub type EbookCategory = v06::EbookCategory;
pub type MusicologyCategory = v16::MusicologyCategory;
pub type RadioCategory = v16::RadioCategory;
pub type OldCategory = v16::OldCategory;
pub type MediaType = v13::MediaType;
pub type Category = v15::Category;

#[instrument(skip_all)]
pub fn migrate(db: &Database<'_>) -> Result<()> {
    let rw = db.rw_transaction()?;

    info!("Migrations started");
    rw.migrate::<Torrent>()?;
    rw.migrate::<SelectedTorrent>()?;
    rw.migrate::<DuplicateTorrent>()?;
    // recover_migrate::<v02::ErroredTorrent, v03::ErroredTorrent>(&rw)?;
    rw.migrate::<ErroredTorrent>()?;
    // recover_migrate::<v03::Event, v04::Event>(&rw)?;
    rw.migrate::<Event>()?;
    rw.migrate::<List>()?;
    rw.migrate::<ListItem>()?;
    rw.commit()?;
    info!("Migrations done");

    Ok(())
}

#[allow(clippy::result_large_err, dead_code)]
fn recover_migrate<Old, New>(rw: &RwTransaction<'_>) -> Result<(), db_type::Error>
where
    Old: From<New> + Clone + ToInput,
    New: From<Old> + ToInput,
{
    let old_data = rw
        .scan()
        .primary::<Old>()?
        .all()?
        .collect::<Result<Vec<_>, _>>()?;

    for old in old_data {
        let new: New = old.clone().into();
        rw.insert(new).or_else(|err| match err {
            db_type::Error::DuplicateKey { .. } => Ok(()),
            err => Err(err),
        })?;
        rw.remove(old)?;
    }

    Ok(())
}

static RW_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub trait DatabaseExt {
    fn db(&self) -> &Database<'_>;

    async fn rw_async(&self) -> Result<(MutexGuard<'_, ()>, RwTransaction<'_>)> {
        // Make sure we are only running one rw_transaction at a time
        let guard = RW_MUTEX.lock().await;
        let rw = self.db().rw_transaction()?;
        Ok((guard, rw))
    }

    fn rw_try(&self) -> Result<(MutexGuard<'_, ()>, RwTransaction<'_>)> {
        // Make sure we are only running one rw_transaction at a time
        let Ok(guard) = RW_MUTEX.try_lock() else {
            return Err(anyhow::Error::msg("Failed to acquire lock"));
        };
        let rw = self.db().rw_transaction()?;
        Ok((guard, rw))
    }
}

impl DatabaseExt for Database<'_> {
    fn db(&self) -> &Database<'_> {
        self
    }
}
