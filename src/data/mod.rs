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

use anyhow::Result;
use native_db::Models;
use native_db::transaction::RwTransaction;
use native_db::{Database, ToInput, db_type};
use once_cell::sync::Lazy;
use tracing::{info, instrument};

pub static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<v01::Config>().unwrap();

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
pub type Torrent = v13::Torrent;
pub type TorrentKey = v13::TorrentKey;
pub type SelectedTorrent = v13::SelectedTorrent;
pub type SelectedTorrentKey = v13::SelectedTorrentKey;
pub type DuplicateTorrent = v13::DuplicateTorrent;
pub type ErroredTorrent = v13::ErroredTorrent;
pub type ErroredTorrentKey = v13::ErroredTorrentKey;
pub type ErroredTorrentId = v11::ErroredTorrentId;
pub type Event = v12::Event;
pub type EventKey = v12::EventKey;
pub type EventType = v12::EventType;
pub type List = v05::List;
pub type ListKey = v05::ListKey;
pub type ListItem = v05::ListItem;
pub type ListItemKey = v05::ListItemKey;
pub type ListItemTorrent = v04::ListItemTorrent;
pub type TorrentMeta = v13::TorrentMeta;
pub type TorrentMetaDiff = v12::TorrentMetaDiff;
pub type TorrentMetaField = v12::TorrentMetaField;
pub type VipStatus = v11::VipStatus;
pub type MetadataSource = v10::MetadataSource;
pub type OldMainCat = v01::MainCat;
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
pub type OldCategory = v06::Category;
pub type MediaType = v13::MediaType;
pub type Category = v12::Category;

#[instrument(skip_all)]
pub fn migrate(db: &Database<'_>) -> Result<()> {
    let rw = db.rw_transaction()?;

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
