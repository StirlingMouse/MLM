use std::{collections::BTreeMap, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use mlm_db::{Torrent, TorrentKey};
use mlm_mam::api::MaM;
use native_db::Database;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{instrument, trace};

use crate::{
    autograbber::select_torrents,
    config::{Config, NotionList},
};

static IMPORT_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[instrument(skip_all)]
pub async fn run_notion_import(
    config: Arc<Config>,
    db: Arc<Database<'_>>,
    mam: Arc<MaM<'_>>,
    list: &NotionList,
    max_torrents: u64,
) -> Result<()> {
    // Make sure we are only running one import at a time
    let _guard = IMPORT_MUTEX.lock().await;

    let content: DatasourceResponse = reqwest::ClientBuilder::new()
        .build()?
        .post(format!(
            "https://api.notion.com/v1/data_sources/{}/query",
            list.data_source,
        ))
        .header("Notion-Version", "2025-09-03")
        .header("Authorization", format!("Bearer {}", list.token))
        .send()
        .await?
        .json()
        .await?;

    trace!("Scanning Notion list {}", list.name);

    for item in content.results.into_iter() {
        let mam_ids = list
            .mam_fields
            .iter()
            .filter_map(|field| item.properties.get(field))
            .filter_map(|propery| match propery {
                Property::Url(url) => url.url.as_ref().and_then(|url| {
                    url.split('/')
                        .next_back()
                        .and_then(|id| id.parse::<u64>().ok())
                }),
                _ => None,
            })
            .collect::<Vec<u64>>();

        'torrent: for mam_id in mam_ids {
            let torrent = db
                .r_transaction()?
                .get()
                .secondary::<Torrent>(TorrentKey::mam_id, mam_id)?;
            if torrent.is_some() {
                continue;
            }

            let mam_torrent = mam.get_torrent_info_by_id(mam_id).await?;
            if let Some(torrent) = mam_torrent {
                for grab in &list.grab {
                    if !grab.filter.matches(&torrent) {
                        continue;
                    }
                    select_torrents(
                        &config,
                        &db,
                        &mam,
                        [torrent].into_iter(),
                        &grab.filter,
                        grab.cost,
                        list.unsat_buffer,
                        list.wedge_buffer,
                        None,
                        list.dry_run,
                        max_torrents,
                        None,
                    )
                    .await
                    .context("select_torrents")?;
                    continue 'torrent;
                }
            }
            sleep(Duration::from_millis(400)).await;
        }
    }

    Ok(())
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasourceResponse {
    pub results: Vec<Item>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub created_time: String,
    pub last_edited_time: String,
    pub archived: bool,
    pub in_trash: bool,
    pub properties: BTreeMap<String, Property>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Property {
    Date(Date),
    Files(Files),
    Number(Number),
    Relation(Relation),
    Select(Select),
    MultiSelect(MultiSelect),
    Status(Status),
    RichText(RichText),
    Title(Title),
    Url(Url),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Date {
    pub id: String,
    pub date: Option<DateValue>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateValue {
    pub start: Option<String>,
    pub end: Option<String>,
    pub time_zone: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Files {
    pub id: String,
    pub files: Vec<FileValue>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileValue {
    pub name: String,
    pub external: Option<External>,
    pub file: Option<File>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct External {
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct File {
    pub url: String,
    pub expiry_time: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Number {
    pub id: String,
    pub number: Option<f64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub relation: Vec<RelationValue>,
    pub has_more: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationValue {
    pub id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Select {
    pub id: String,
    pub select: Option<SelectValue>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiSelect {
    pub id: String,
    pub multi_select: Vec<SelectValue>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectValue {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub id: String,
    pub status: Option<SelectValue>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RichText {
    pub id: String,
    pub rich_text: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Url {
    pub id: String,
    pub url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Title {
    pub id: String,
    pub title: Vec<Value>,
}
