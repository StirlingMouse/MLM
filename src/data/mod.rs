pub mod impls;
mod v1;
mod v2;
mod v3;
mod v4;
mod v5;
mod v6;
mod v7;
mod v8;

use anyhow::Result;
use native_db::Models;
use native_db::transaction::RwTransaction;
use native_db::{Database, ToInput, db_type};
use once_cell::sync::Lazy;
use tracing::{info, instrument};

pub static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<v1::Config>().unwrap();

    models.define::<v8::Torrent>().unwrap();
    models.define::<v8::SelectedTorrent>().unwrap();
    models.define::<v8::DuplicateTorrent>().unwrap();
    models.define::<v8::ErroredTorrent>().unwrap();
    models.define::<v8::Event>().unwrap();

    models.define::<v7::Torrent>().unwrap();
    models.define::<v7::SelectedTorrent>().unwrap();
    models.define::<v7::DuplicateTorrent>().unwrap();
    models.define::<v7::Event>().unwrap();

    models.define::<v6::Torrent>().unwrap();
    models.define::<v6::SelectedTorrent>().unwrap();
    models.define::<v6::DuplicateTorrent>().unwrap();
    models.define::<v6::ErroredTorrent>().unwrap();

    models.define::<v5::Torrent>().unwrap();
    models.define::<v5::List>().unwrap();
    models.define::<v5::ListItem>().unwrap();

    models.define::<v4::SelectedTorrent>().unwrap();
    models.define::<v4::Event>().unwrap();
    models.define::<v4::List>().unwrap();
    models.define::<v4::ListItem>().unwrap();

    models.define::<v3::Torrent>().unwrap();
    models.define::<v3::SelectedTorrent>().unwrap();
    models.define::<v3::DuplicateTorrent>().unwrap();
    models.define::<v3::ErroredTorrent>().unwrap();
    models.define::<v3::Event>().unwrap();
    models.define::<v3::List>().unwrap();
    models.define::<v3::ListItem>().unwrap();

    models.define::<v2::Torrent>().unwrap();
    models.define::<v2::SelectedTorrent>().unwrap();
    models.define::<v2::DuplicateTorrent>().unwrap();
    models.define::<v2::ErroredTorrent>().unwrap();

    models.define::<v1::Torrent>().unwrap();
    models.define::<v1::SelectedTorrent>().unwrap();
    models.define::<v1::DuplicateTorrent>().unwrap();
    models.define::<v1::ErroredTorrent>().unwrap();

    models
});

pub type Config = v1::Config;
pub type Torrent = v8::Torrent;
pub type TorrentKey = v8::TorrentKey;
pub type SelectedTorrent = v8::SelectedTorrent;
pub type SelectedTorrentKey = v8::SelectedTorrentKey;
pub type DuplicateTorrent = v8::DuplicateTorrent;
pub type ErroredTorrent = v8::ErroredTorrent;
pub type ErroredTorrentKey = v8::ErroredTorrentKey;
pub type ErroredTorrentId = v1::ErroredTorrentId;
pub type Event = v8::Event;
pub type EventKey = v8::EventKey;
pub type EventType = v8::EventType;
pub type List = v5::List;
pub type ListKey = v5::ListKey;
pub type ListItem = v5::ListItem;
pub type ListItemKey = v5::ListItemKey;
pub type ListItemTorrent = v4::ListItemTorrent;
pub type TorrentMeta = v8::TorrentMeta;
pub type TorrentMetaDiff = v8::TorrentMetaDiff;
pub type TorrentMetaField = v8::TorrentMetaField;
pub type MainCat = v1::MainCat;
pub type Uuid = v3::Uuid;
pub type Timestamp = v3::Timestamp;
pub type Language = v3::Language;
pub type FlagBits = v8::FlagBits;
pub type Size = v3::Size;
pub type TorrentCost = v4::TorrentCost;
pub type TorrentStatus = v4::TorrentStatus;
pub type LibraryMismatch = v8::LibraryMismatch;
pub type ClientStatus = v8::ClientStatus;
pub type AudiobookCategory = v6::AudiobookCategory;
pub type EbookCategory = v6::EbookCategory;
pub type Category = v6::Category;

#[instrument(skip_all)]
pub fn migrate(db: &Database<'_>) -> Result<()> {
    let rw = db.rw_transaction()?;

    rw.migrate::<Torrent>()?;
    rw.migrate::<SelectedTorrent>()?;
    rw.migrate::<DuplicateTorrent>()?;
    // recover_migrate::<v2::ErroredTorrent, v3::ErroredTorrent>(&rw)?;
    rw.migrate::<ErroredTorrent>()?;
    // recover_migrate::<v3::Event, v4::Event>(&rw)?;
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
