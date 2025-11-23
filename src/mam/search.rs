use std::collections::BTreeMap;

use crate::{
    data::{
        Category, FlagBits, Language, MainCat, MediaType, MetadataSource, OldCategory, Series,
        SeriesEntries, Timestamp, TorrentMeta, VipStatus,
    },
    mam::{
        enums::{SearchIn, SearchKind, SearchTarget},
        meta::{MetaError, clean_value},
        serde::{
            DATE_TIME_FORMAT, bool_string_or_number, is_false, is_zero, json_or_default,
            opt_string_or_number, parse_title, string_or_number,
        },
    },
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::UtcDateTime;
use tracing::warn;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SearchQuery<'a> {
    /// If this parameter is set, it will display the full description field for the torrent.
    #[serde(skip_serializing_if = "is_false")]
    pub description: bool,
    /// show hash for dl link (prepend https://www.myanonamouse.net/tor/download.php/ to use) for downloading on something without a session cookie. Alternatively use session cookie and just hit https://www.myanonamouse.net/tor/download.php?tid=# replacing # with the id number.
    #[serde(skip_serializing_if = "is_false")]
    #[serde(rename = "dlLink")]
    pub dl_link: bool,
    /// If this value is set, will return the isbn field (though often blank).
    #[serde(skip_serializing_if = "is_false")]
    pub isbn: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub thumbnail: bool,
    /// int in range of 5 to 100, telling how many results to return
    #[serde(skip_serializing_if = "is_zero")]
    pub perpage: u64,

    #[serde(borrow)]
    pub tor: Tor<'a>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Tor<'a> {
    #[serde(rename = "searchIn")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<SearchTarget>,
    #[serde(rename = "searchType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<SearchKind>,

    /// Text to search for
    #[serde(skip_serializing_if = "str::is_empty")]
    pub text: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "srchIn")]
    pub srch_in: Vec<SearchIn>,

    /// List of integers for the languages you wish to view in results
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub browse_lang: Vec<u8>,
    /// Array of ID(s) of the main category to include
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub main_cat: Vec<u8>,
    /// List of integers for the categories you wish to view in results
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cat: Vec<u8>,

    /// Date in format YYYY-MM-DD or unix timestamp of earliest torrent(s) to show. Inclusive of the provided value
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(rename = "startDate")]
    pub start_date: String,
    /// Date in format YYYY-MM-DD or unix timestamp torrents should have been created before. Exclusive of value provided
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(rename = "endDate")]
    pub end_date: String,

    #[serde(skip_serializing_if = "is_zero")]
    #[serde(rename = "minSize")]
    pub min_size: u64,
    #[serde(skip_serializing_if = "is_zero")]
    #[serde(rename = "maxSize")]
    pub max_size: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub unit: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "minSeeders")]
    pub min_seeders: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxSeeders")]
    pub max_seeders: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "minLeechers")]
    pub min_leechers: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxLeechers")]
    pub max_leechers: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "minSnatched")]
    pub min_snatched: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxSnatched")]
    pub max_snatched: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "browseFlagsHideVsShow")]
    pub browse_flags_hide_vs_show: Option<u8>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "browseFlags")]
    pub browse_flags: Vec<u8>,

    /// Hexadecimal encoded hash from a torrent
    #[serde(skip_serializing_if = "str::is_empty")]
    pub hash: &'a str,

    #[serde(skip_serializing_if = "is_zero")]
    pub id: u64,

    // sortType	enum	'titleAsc': By the Title, Descending order
    // 'titleDesc': By the Title, Ascending order
    // 'fileAsc': By number of files, Ascending Order
    // 'fileDesc': By number of files, Descending Order
    // 'sizeAsc': By size of the torrent, Ascending Order
    // 'sizeDesc': By size of the torrent, Descending Order
    // 'seedersAsc': By number of Seeders, Ascending Order
    // 'seedersDesc': By number of Seeders, Descending Order
    // 'leechersAsc': By number of Leechers, Ascending Order
    // 'leechersDesc': By number of Leechers, Descending Order
    // 'snatchedAsc': By number of times snatched, Ascending Order
    // 'snatchedDesc': By number of times snatched, Descending Order
    // 'dateAsc': By Date Added, Ascending Order
    // 'dateDesc': By Date Added, Descending Order
    // 'bmkaAsc': By date bookmarked, Ascending Order (Note: may return odd results if not bookmarked)
    // 'bmkaDesc': By date bookmarked, Descending Order (Note: may return odd results if not bookmarked)
    // 'reseedAsc': Date Reseed Request Added, Ascending Order (Note: may return odd results if no reseed request)
    // 'reseedDesc': Date Reseed Request Added, Descending Order (Note: may return odd results if no reseed request)
    // 'categoryAsc': Sorted by category (number) Ascending, followed by title Ascending
    // 'categoryDesc': Sorted by category (number) Descending, followed by title Ascending
    // 'random': random, duh
    // 'default':
    // If text search present: by weight DESC, then ID desceding,
    // else if instead searchIn is 'myReseed' or 'allReseed': same as reseedAsc
    // else if searchIn is Bookmarks: same as 'bmkaDesc'
    // else same as 'dateDesc'
    #[serde(rename = "sortType")]
    #[serde(skip_serializing_if = "str::is_empty")]
    pub sort_type: &'a str,

    /// Number of entries to skip. Used in pagination.
    #[serde(rename = "startNumber")]
    #[serde(skip_serializing_if = "is_zero")]
    pub start_number: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SearchResult {
    pub perpage: usize,
    pub start: usize,
    pub data: Vec<MaMTorrent>,
    pub total: usize,
    pub found: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchError {
    pub error: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MaMTorrent {
    pub id: u64,
    /// format: DATE_TIME_FORMAT
    pub added: String,
    #[serde(deserialize_with = "json_or_default")]
    pub author_info: BTreeMap<u64, String>,
    pub bookmarked: Option<u64>,
    pub browseflags: u8,
    /// Old MainCat field -> MediaType
    pub main_cat: u8,
    pub category: u64,
    pub mediatype: u8,
    pub maincat: u8,
    #[serde(deserialize_with = "json_or_default")]
    pub categories: Vec<u8>,
    pub catname: String,
    pub cat: String,
    pub comments: u64,
    #[serde(default)]
    #[serde(deserialize_with = "opt_string_or_number")]
    pub description: Option<String>,
    pub dl: Option<String>,
    pub filetype: String,
    #[serde(deserialize_with = "bool_string_or_number")]
    pub fl_vip: bool,
    #[serde(deserialize_with = "bool_string_or_number")]
    pub free: bool,
    #[serde(default)]
    #[serde(deserialize_with = "opt_string_or_number")]
    pub isbn: Option<String>,
    pub lang_code: String,
    pub language: u8,
    pub leechers: u64,
    #[serde(deserialize_with = "bool_string_or_number")]
    pub my_snatched: bool,
    #[serde(deserialize_with = "json_or_default")]
    pub narrator_info: BTreeMap<u64, String>,
    pub numfiles: u64,
    pub owner: u64,
    #[serde(deserialize_with = "string_or_number")]
    pub owner_name: String,
    #[serde(deserialize_with = "bool_string_or_number")]
    pub personal_freeleech: bool,
    pub seeders: u64,
    #[serde(deserialize_with = "json_or_default")]
    pub series_info: BTreeMap<u64, Vec<Value>>,
    pub size: String,
    #[serde(deserialize_with = "string_or_number")]
    pub tags: String,
    pub times_completed: u64,
    pub thumbnail: Option<String>,
    #[serde(deserialize_with = "parse_title")]
    pub title: String,
    #[serde(deserialize_with = "bool_string_or_number")]
    pub vip: bool,
    pub vip_expire: u64,
    pub w: u64,
}

impl MaMTorrent {
    pub fn as_meta(&self) -> Result<TorrentMeta, MetaError> {
        let authors = self
            .author_info
            .values()
            .map(|a| clean_value(a))
            .collect::<Result<Vec<_>>>()?;
        let narrators = self
            .narrator_info
            .values()
            .map(|n| clean_value(n))
            .collect::<Result<Vec<_>>>()?;
        let series = self
            .series_info
            .values()
            .map(|series| {
                let Value::String(series_name) = series
                    .get(0)
                    .ok_or(MetaError::InvalidSeries("Missing series name"))?
                else {
                    return Err(MetaError::InvalidSeries("Series name is not a string").into());
                };
                let Value::String(series_num) = series
                    .get(1)
                    .ok_or(MetaError::InvalidSeries("Missing series num"))?
                else {
                    return Err(MetaError::InvalidSeries("Series num is not a string").into());
                };
                let series_name = clean_value(series_name)?;
                Series::try_from((series_name.clone(), series_num.clone())).or_else(|err| {
                    warn!("error parsing series num: {err}");
                    Ok(Series {
                        name: series_name,
                        entries: SeriesEntries::new(vec![]),
                    })
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let media_type = MediaType::from_id(self.mediatype)
            .or_else(|| MediaType::from_main_cat_id(self.main_cat))
            .ok_or_else(|| {
                MetaError::UnknownMediaType(format!(
                    "Unknown mediatype {} and main_cat {}",
                    self.mediatype, self.main_cat
                ))
            })?;
        let main_cat = MainCat::from_id(self.maincat);
        let categories = self
            .categories
            .iter()
            .map(|id| Category::from_id(*id).ok_or_else(|| MetaError::UnknownCat(*id)))
            .collect::<Result<Vec<_>, _>>()?;
        let cat = OldCategory::from_one_id(self.category)
            .ok_or_else(|| MetaError::UnknownOldCat(self.catname.clone()))?;

        let language = Language::from_id(self.language)
            .ok_or_else(|| MetaError::UnknownLanguage(self.language, self.lang_code.clone()))?;
        let filetypes = self
            .filetype
            .split(" ")
            .map(|t| t.to_owned())
            .collect::<Vec<_>>();
        let size = self.size.parse().map_err(MetaError::InvalidSize)?;
        let vip_status = if !self.vip {
            VipStatus::NotVip
        } else if self.vip_expire == 0 {
            VipStatus::Permanent
        } else {
            VipStatus::Temp(
                UtcDateTime::from_unix_timestamp(self.vip_expire as i64)
                    .map_err(|_| MetaError::InvalidVipExpiry(self.vip_expire))?
                    .date(),
            )
        };
        let uploaded_at = match UtcDateTime::parse(&self.added, &DATE_TIME_FORMAT) {
            Ok(added) => Timestamp::from(added),
            Err(_) => {
                return Err(MetaError::InvalidAdded(self.added.clone()));
            }
        };

        Ok(TorrentMeta {
            mam_id: self.id,
            vip_status: Some(vip_status),
            media_type,
            main_cat,
            categories,
            cat: Some(cat),
            language: Some(language),
            flags: Some(FlagBits::new(self.browseflags)),
            filetypes,
            size,
            title: self.title.to_owned(),
            authors,
            narrators,
            series,
            source: MetadataSource::Mam,
            uploaded_at,
        })
    }

    pub fn is_free(&self) -> bool {
        self.free || self.personal_freeleech || self.fl_vip
    }
}
