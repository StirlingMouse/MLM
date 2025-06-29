use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Error, Result};
use bytes::Bytes;
use cookie::Cookie;
use htmlentity::entity::{self, ICodedDataTrait as _};
use native_db::Database;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Url;
use reqwest_cookie_store::CookieStoreRwLock;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use unidecode::unidecode;

use crate::{
    config::Config,
    data,
    mam_enums::{SearchIn, UserClass},
};

fn is_false(value: &bool) -> bool {
    !value
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub classname: UserClass,
    pub connectable: String,
    pub country_code: String,
    pub country_name: String,
    pub created: u64,
    pub downloaded: String,
    pub downloaded_bytes: u64,
    pub duplicates: Duplicates,
    #[serde(rename = "inactHnr")]
    pub inact_hnr: InactHnr,
    #[serde(rename = "inactSat")]
    pub inact_sat: InactHnr,
    #[serde(rename = "inactUnsat")]
    pub inact_unsat: InactHnr,
    pub ipv6_mac: bool,
    pub ite: Ite,
    pub last_access: String,
    pub last_access_ago: String,
    pub leeching: InactHnr,
    pub partial: bool,
    pub ratio: f64,
    pub recently_deleted: u64,
    pub reseed: Reseed,
    #[serde(rename = "sSat")]
    pub s_sat: InactHnr,
    #[serde(rename = "seedHnr")]
    pub seed_hnr: InactHnr,
    #[serde(rename = "seedUnsat")]
    pub seed_unsat: InactHnr,
    pub seedbonus: i64,
    pub uid: u64,
    pub unsat: InactHnr,
    #[serde(rename = "upAct")]
    pub up_act: InactHnr,
    #[serde(rename = "upInact")]
    pub up_inact: InactHnr,
    pub update: u64,
    pub uploaded: String,
    pub uploaded_bytes: u64,
    pub username: String,
    pub v6_connectable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Duplicates {
    pub count: u64,
    pub red: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InactHnr {
    pub count: u64,
    pub red: bool,
    pub size: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ite {
    pub count: u64,
    pub latest: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reseed {
    pub count: u64,
    pub inactive: u64,
    pub red: bool,
}

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
    #[serde(skip_serializing_if = "str::is_empty")]
    #[serde(rename = "startDate")]
    pub start_date: &'a str,
    /// Date in format YYYY-MM-DD or unix timestamp torrents should have been created before. Exclusive of value provided
    #[serde(skip_serializing_if = "str::is_empty")]
    #[serde(rename = "endDate")]
    pub end_date: &'a str,

    #[serde(skip_serializing_if = "is_zero")]
    #[serde(rename = "minSize")]
    pub min_size: u64,
    #[serde(skip_serializing_if = "is_zero")]
    #[serde(rename = "maxSize")]
    pub max_size: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub unit: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "browseFlagsHideVsShow")]
    pub browse_flags_hide_vs_show: Option<u8>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "browseFlags")]
    pub browse_flags: Vec<u8>,

    /// Hexadecimal encoded hash from a torrent
    #[serde(skip_serializing_if = "str::is_empty")]
    pub hash: &'a str,

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

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SearchTarget {
    Bookmarks,
    New,
    Mine,
    AllReseed,
    MyReseed,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SearchKind {
    /// Last update had 1+ seeders
    #[serde(rename = "active")]
    Active,
    /// Last update has 0 seeders
    #[serde(rename = "inactive")]
    Inactive,
    /// Freeleech torrents
    #[serde(rename = "fl")]
    Freeleech,
    /// Freeleech or VIP torrents
    #[serde(rename = "fl-VIP")]
    Free,
    /// VIP torrents
    #[serde(rename = "VIP")]
    Vip,
    /// Torrents not VIP
    #[serde(rename = "nVIP")]
    NotVip,
    /// Torrents missing meta data (old torrents)
    #[serde(rename = "nMeta")]
    NoMeta,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct MaMTorrent {
    pub id: u64,
    pub added: String,
    #[serde(deserialize_with = "json_or_default")]
    pub author_info: BTreeMap<u64, String>,
    pub bookmarked: Option<u64>,
    pub browseflags: u8,
    pub category: u64,
    pub catname: String,
    pub cat: String,
    pub comments: u64,
    #[serde(default)]
    #[serde(deserialize_with = "opt_string_or_number")]
    pub description: Option<String>,
    pub dl: Option<String>,
    pub filetype: String,
    pub fl_vip: i64,
    pub free: i64,
    #[serde(default)]
    #[serde(deserialize_with = "opt_string_or_number")]
    pub isbn: Option<String>,
    pub lang_code: String,
    pub language: u8,
    pub leechers: u64,
    pub main_cat: u64,
    pub my_snatched: u64,
    #[serde(deserialize_with = "json_or_default")]
    pub narrator_info: BTreeMap<u64, String>,
    pub numfiles: u64,
    pub owner: u64,
    #[serde(deserialize_with = "string_or_number")]
    pub owner_name: String,
    pub personal_freeleech: u64,
    pub seeders: u64,
    #[serde(deserialize_with = "json_or_default")]
    pub series_info: BTreeMap<u64, (String, String)>,
    pub size: String,
    #[serde(deserialize_with = "string_or_number")]
    pub tags: String,
    pub times_completed: u64,
    #[serde(deserialize_with = "parse_title")]
    pub title: String,
    pub vip: u64,
    pub w: u64,
}

impl MaMTorrent {
    pub fn as_meta(&self) -> Result<data::TorrentMeta, MetaError> {
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
            .map(|(series_name, series_num)| {
                let series_name = clean_value(series_name)?;
                Ok((series_name, series_num.clone()))
            })
            .collect::<Result<Vec<_>>>()?;
        let main_cat = data::MainCat::from_id(self.main_cat).map_err(MetaError::UnknownMainCat)?;
        let filetypes = self
            .filetype
            .split(" ")
            .map(|t| t.to_owned())
            .collect::<Vec<_>>();

        Ok(data::TorrentMeta {
            mam_id: self.id,
            main_cat,
            filetypes,
            title: self.title.to_owned(),
            authors,
            narrators,
            series,
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MetaError {
    #[error("{0}")]
    UnknownMainCat(String),
    #[error("Unknown error: {0}")]
    Other(#[from] Error),
}

pub fn clean_value(value: &str) -> Result<String> {
    entity::decode(value.as_bytes()).to_string()
}

pub fn normalize_title(value: &str) -> String {
    unidecode(value).to_lowercase()
}

fn opt_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<Value>::deserialize(deserializer)?;
    match v {
        Some(Value::String(v)) => Ok(Some(v)),
        Some(Value::Number(v)) => Ok(Some(v.to_string())),
        None => Ok(None),
        _ => Err(serde::de::Error::custom("expected number or string")),
    }
}

fn string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Value::deserialize(deserializer)?;
    match v {
        Value::String(v) => Ok(v),
        Value::Number(v) => Ok(v.to_string()),
        _ => Err(serde::de::Error::custom("expected number or string")),
    }
}

pub static TITLE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(.*?) +\[[^\]]*\]$").unwrap());

// Workaround policy to put english translations in torrent titles
fn parse_title<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let title = string_or_number(deserializer)?;

    if let Some(title) = TITLE_PATTERN.captures(&title).and_then(|c| c.get(1)) {
        Ok(title.as_str().to_owned())
    } else {
        Ok(title)
    }
}

fn json_or_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: Deserializer<'de>,
{
    let v: Result<Value, _> = serde_nested_json::deserialize(deserializer);
    let Ok(v) = v else {
        return Ok(T::default());
    };
    Ok(T::deserialize(v).unwrap_or_default())
}

pub struct MaM<'a> {
    jar: Arc<CookieStoreRwLock>,
    client: reqwest::Client,
    db: Arc<Database<'a>>,
}

impl<'a> MaM<'a> {
    pub async fn new(config: &Config, db: Arc<Database<'a>>) -> Result<MaM<'a>> {
        let jar: CookieStoreRwLock = Default::default();
        let url = "https://www.myanonamouse.net".parse::<Url>().unwrap();

        let stored_mam_id = db
            .r_transaction()
            .and_then(|r| r.get().primary::<data::Config>("mam_id"))
            .ok()
            .flatten()
            .map(|c| c.value);
        if stored_mam_id.is_some() {
            println!("Restoring mam_id");
        } else {
            println!("Using mam_id from config");
        }
        let has_stored_mam_id = stored_mam_id.is_some();
        let cookie =
            Cookie::build(("mam_id", stored_mam_id.unwrap_or(config.mam_id.clone()))).build();
        jar.write()
            .unwrap()
            .store_response_cookies([cookie].into_iter(), &url);

        let jar = Arc::new(jar);
        let client = reqwest::Client::builder()
            .cookie_provider(jar.clone())
            .user_agent("MLM")
            .build()?;

        let mam = MaM { jar, client, db };
        if let Err(err) = mam.check_mam_id().await {
            if has_stored_mam_id {
                eprintln!("Stored mam_id failed with {err}, falling back to config value");
                let cookie = Cookie::build(("mam_id", config.mam_id.clone())).build();
                mam.jar
                    .write()
                    .unwrap()
                    .store_response_cookies([cookie].into_iter(), &url);
                mam.check_mam_id().await?;
            } else {
                return Err(err);
            }
        }

        Ok(mam)
    }

    pub async fn check_mam_id(&self) -> Result<()> {
        let resp = self
            .client
            .get("https://www.myanonamouse.net/json/checkCookie.php")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        println!("checked mam_id: {resp:?}");
        self.store_cookies();
        Ok(())
    }

    pub async fn user_info(&self) -> Result<UserResponse> {
        let resp = self
            .client
            .get("https://www.myanonamouse.net/jsonLoad.php?snatch_summary=true")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        self.store_cookies();
        Ok(resp)
    }

    pub async fn get_torrent_file(&self, dl_hash: &str) -> Result<Bytes> {
        let resp = self
            .client
            .get(format!(
                "https://www.myanonamouse.net/tor/download.php/{dl_hash}"
            ))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(resp)
    }
    pub async fn get_torrent_info(&self, hash: &str) -> Result<Option<MaMTorrent>> {
        let mut resp = self
            .search(&SearchQuery {
                description: true,
                isbn: true,
                tor: Tor {
                    hash,
                    ..Default::default()
                },
                ..Default::default()
            })
            .await?;
        Ok(resp.data.pop())
    }

    pub async fn search(&self, query: &SearchQuery<'_>) -> Result<SearchResult> {
        println!("search: {}", serde_json::to_string_pretty(query)?);
        let resp = self
            .client
            .post("https://www.myanonamouse.net/tor/js/loadSearchJSONbasic.php")
            .json(query)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        if let Ok(resp) = serde_json::from_str::<SearchError>(&resp) {
            if resp.error == "Nothing returned, out of 0" {
                return Ok(SearchResult::default());
            } else {
                return Err(Error::msg(resp.error));
            }
        };
        let resp: SearchResult = serde_json::from_str(&resp).map_err(|err| {
            eprintln!("Error parsing mam response: {err}\nResponse: {resp}");
            err
        })?;
        self.store_cookies();
        Ok(resp)
    }

    fn store_cookies(&self) {
        if let Some(cookie) = self
            .jar
            .read()
            .unwrap()
            .get("www.myanonamouse.net", "/", "mam_id")
        {
            if let Ok(rw) = self.db.rw_transaction() {
                let ok = rw
                    .upsert(data::Config {
                        key: "mam_id".to_string(),
                        value: cookie.value().to_string(),
                    })
                    .and_then(|_| rw.commit())
                    .is_ok();

                if ok {
                    println!("stored new mam_id");
                }
            }
        }
    }
}
