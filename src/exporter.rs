use std::{
    fs::File,
    io::{BufWriter, Write as _},
};

use anyhow::Result;
use native_db::{Database, db_type};
use serde::{Deserialize, Serialize};

use crate::data;

#[derive(Serialize, Deserialize, Debug)]
struct ExportV1 {
    config: Vec<data::Config>,
    torrents: Vec<data::Torrent>,
    selected_torrents: Vec<data::SelectedTorrent>,
    duplicate_torrents: Vec<data::DuplicateTorrent>,
    errored_torrents: Vec<data::ErroredTorrent>,
}

pub fn export_db(db: &Database<'_>) -> Result<()> {
    let r = db.r_transaction()?;
    let export = ExportV1 {
        config: r
            .scan()
            .primary()?
            .all()?
            .collect::<Result<_, db_type::Error>>()?,
        torrents: r
            .scan()
            .primary()?
            .all()?
            .collect::<Result<_, db_type::Error>>()?,
        selected_torrents: r
            .scan()
            .primary()?
            .all()?
            .collect::<Result<_, db_type::Error>>()?,
        duplicate_torrents: r
            .scan()
            .primary()?
            .all()?
            .collect::<Result<_, db_type::Error>>()?,
        errored_torrents: r
            .scan()
            .primary()?
            .all()?
            .collect::<Result<_, db_type::Error>>()?,
    };

    let file = File::create("export_v1.json")?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &export)?;
    writer.flush()?;

    Ok(())
}
