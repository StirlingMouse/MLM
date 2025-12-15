use crate::{
    data::{
        Category, FlagBits, MediaType, MetadataSource, OldCategory, Series, SeriesEntries,
        Timestamp, TorrentMeta, VipStatus,
    },
    mam::{
        meta::{MetaError, clean_value},
        serde::{bool_string_or_number, num_string_or_number, opt_num_string_or_number},
    },
};
use anyhow::Result;
use itertools::Itertools as _;
use serde::{Deserialize, Serialize};
use time::UtcDateTime;
use tracing::warn;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserDetailsTorrentResponse {
    pub rows: Vec<UserDetailsTorrent>,
    pub success: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserDetailsTorrent {
    pub catname: String,
    pub catimg: String,
    #[serde(deserialize_with = "num_string_or_number")]
    pub category: u64,
    #[serde(deserialize_with = "num_string_or_number")]
    pub id: u64,
    pub title: String,
    #[serde(deserialize_with = "num_string_or_number")]
    pub comments: u64,
    /// format: DATE_TIME_FORMAT
    pub last_seed: String,
    pub size: String,
    pub tags: String,
    #[serde(rename = "browseFlags")]
    #[serde(deserialize_with = "num_string_or_number")]
    pub browse_flags: u8,
    #[serde(deserialize_with = "bool_string_or_number")]
    pub free: bool,
    #[serde(deserialize_with = "bool_string_or_number")]
    pub vip: bool,
    #[serde(default)]
    #[serde(deserialize_with = "opt_num_string_or_number")]
    pub vip_expire: Option<u64>,
    #[serde(deserialize_with = "num_string_or_number")]
    pub times_completed: u64,
    #[serde(deserialize_with = "num_string_or_number")]
    pub uploaded: u64,
    #[serde(deserialize_with = "num_string_or_number")]
    pub downloaded: u64,
    /// format: DATE_TIME_FORMAT
    pub complete_date: String,
    #[serde(deserialize_with = "num_string_or_number")]
    pub complete_unix: u64,
    #[serde(deserialize_with = "num_string_or_number")]
    pub seedtime: u64,
    #[serde(deserialize_with = "num_string_or_number")]
    pub leechtime: u64,
    #[serde(rename = "OneToOne")]
    #[serde(deserialize_with = "bool_string_or_number")]
    pub one_to_one: bool,
    pub last_action: String,
    #[serde(deserialize_with = "num_string_or_number")]
    pub last_action_unix: u64,
    /// format: DATE_TIME_FORMAT
    pub start_date: String,
    #[serde(deserialize_with = "num_string_or_number")]
    pub start_unix: u64,
    #[serde(deserialize_with = "bool_string_or_number")]
    pub finished: bool,
    #[serde(deserialize_with = "num_string_or_number")]
    pub to_go: u64,
    #[serde(deserialize_with = "num_string_or_number")]
    pub seeders: u64,
    #[serde(deserialize_with = "num_string_or_number")]
    pub leechers: u64,
    #[serde(rename = "uploaderName")]
    pub uploader_name: String,
    #[serde(rename = "uploaderID")]
    #[serde(deserialize_with = "num_string_or_number")]
    pub uploader_id: u64,
    #[serde(rename = "personalFree")]
    #[serde(deserialize_with = "bool_string_or_number")]
    pub personal_free: bool,
    // #[serde(rename = "uploadPretty")]
    // pub upload_pretty: String,
    // #[serde(rename = "downloadPretty")]
    // pub download_pretty: String,
    // #[serde(rename = "ratioColor")]
    // pub ratio_color: String,
    // pub ratio: String,
    // #[serde(rename = "seedtimeColor")]
    // pub seedtime_color: String,
    // #[serde(rename = "seedtimePretty")]
    // pub seedtime_pretty: String,
    // #[serde(rename = "leechtimePretty")]
    // pub leechtime_pretty: String,
    pub cat: String,
    pub dl: String,
    #[serde(rename = "percentDone")]
    pub percent_done: i64,
    pub to_go_pretty: String,
    #[serde(rename = "STG")]
    /// Seed Time to Go, format: "1d 12:10:28", "23:46:38"
    pub stg: Option<String>,
    #[serde(default)]
    pub author: Vec<Author>,
    #[serde(default)]
    pub series: Vec<MaMSeries>,
    #[serde(default)]
    pub categories: Vec<MaMCategory>,
    #[serde(default)]
    #[serde(rename = "fileTypes")]
    pub file_types: Vec<FileType>,
    #[serde(default)]
    pub narrator: Vec<Narrator>,
    #[serde(rename = "fileTypesDisabled")]
    #[serde(default)]
    pub file_types_disabled: Vec<FileTypesDisabled>,
}

impl UserDetailsTorrent {
    pub fn as_meta(&self) -> Result<TorrentMeta, MetaError> {
        let authors = self
            .author
            .iter()
            .sorted_by(|a, b| a.id.cmp(&b.id))
            .map(|a| a.name.clone())
            .collect();
        let narrators = self
            .narrator
            .iter()
            .sorted_by(|a, b| a.id.cmp(&b.id))
            .map(|a| a.name.clone())
            .collect();
        let series = self
            .series
            .iter()
            .sorted_by(|a, b| a.id.cmp(&b.id))
            .map(|series| {
                Series::try_from((series.name.clone(), series.number.clone())).or_else(|err| {
                    warn!("error parsing series num: {err}");
                    Ok(Series {
                        name: series.name.clone(),
                        entries: SeriesEntries::new(vec![]),
                    })
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let cat = OldCategory::from_one_id(self.category)
            .ok_or_else(|| MetaError::UnknownOldCat(self.catname.clone(), self.category))?;
        let media_type =
            MediaType::from_main_cat_id(cat.as_main_cat().as_id()).ok_or_else(|| {
                MetaError::UnknownMediaType(format!(
                    "Unknown mediatype from old cat {}",
                    self.category
                ))
            })?;
        let mut categories = self
            .categories
            .iter()
            .map(|c| Category::from_id(c.id as u8).ok_or_else(|| MetaError::UnknownCat(c.id as u8)))
            .collect::<Result<Vec<_>, _>>()?;
        categories.sort();

        let filetypes = self
            .file_types
            .iter()
            .map(|t| t.name.to_owned())
            .collect::<Vec<_>>();
        let size = self.size.parse().map_err(MetaError::InvalidSize)?;

        let vip_status = if !self.vip {
            VipStatus::NotVip
        } else if let Some(vip_expire) = self.vip_expire {
            VipStatus::Temp(
                UtcDateTime::from_unix_timestamp(vip_expire as i64)
                    .map_err(|_| MetaError::InvalidVipExpiry(vip_expire))?
                    .date(),
            )
        } else {
            VipStatus::Permanent
        };

        Ok(TorrentMeta {
            mam_id: self.id,
            vip_status: Some(vip_status),
            media_type,
            // TODO: Currently main_cat isn't returned
            main_cat: None,
            categories,
            cat: Some(cat),
            // TODO: Currently language isn't returned
            language: None,
            flags: Some(FlagBits::new(self.browse_flags)),
            filetypes,
            // TODO: Currently num_files isn't returned
            num_files: 0,
            size,
            title: clean_value(&self.title)?,
            edition: None,
            authors,
            narrators,
            series,
            source: MetadataSource::Mam,
            // TODO: Currently added isn't returned
            uploaded_at: Timestamp::from(UtcDateTime::UNIX_EPOCH),
        }
        .clean(&clean_value(&self.tags)?)?)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Author {
    #[serde(deserialize_with = "num_string_or_number")]
    pub id: u64,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaMSeries {
    #[serde(deserialize_with = "num_string_or_number")]
    pub id: u64,
    pub name: String,
    pub number: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaMCategory {
    #[serde(deserialize_with = "num_string_or_number")]
    pub id: u64,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileType {
    #[serde(deserialize_with = "num_string_or_number")]
    pub id: u64,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Narrator {
    #[serde(deserialize_with = "num_string_or_number")]
    pub id: u64,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileTypesDisabled {
    #[serde(deserialize_with = "num_string_or_number")]
    pub id: u64,
    pub name: String,
}
