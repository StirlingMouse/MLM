use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Error, Result};
use reqwest::{Url, cookie::Jar};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};

use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub perpage: i64,
    pub start: i64,
    pub data: Vec<MaMTorrent>,
    pub total: i64,
    pub found: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchError {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaMTorrent {
    pub id: i64,
    pub added: String,
    #[serde(deserialize_with = "json_or_default")]
    pub author_info: BTreeMap<u64, String>,
    pub bookmarked: Option<u64>,
    pub browseflags: u64,
    pub category: u64,
    pub catname: String,
    pub cat: String,
    pub comments: u64,
    pub description: String,
    pub filetype: String,
    pub fl_vip: i64,
    pub free: i64,
    pub isbn: Value,
    pub lang_code: String,
    pub language: u64,
    pub leechers: u64,
    pub main_cat: u64,
    pub my_snatched: u64,
    #[serde(deserialize_with = "json_or_default")]
    pub narrator_info: BTreeMap<u64, String>,
    pub numfiles: u64,
    pub owner: u64,
    // number if name is only digits
    pub owner_name: Value,
    pub personal_freeleech: u64,
    pub seeders: u64,
    #[serde(deserialize_with = "json_or_default")]
    pub series_info: BTreeMap<u64, (String, String)>,
    pub size: String,
    pub tags: String,
    pub times_completed: u64,
    pub title: String,
    pub vip: u64,
    pub w: u64,
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

pub struct MaM {
    jar: Arc<Jar>,
    client: reqwest::Client,
}

impl MaM {
    pub fn new(config: &Config) -> Result<MaM> {
        let cookie = format!("mam_id={}; Domain=.myanonamouse.net", config.mam_id);
        let url = "https://www.myanonamouse.net".parse::<Url>().unwrap();

        let jar = Jar::default();
        jar.add_cookie_str(&cookie, &url);
        let jar = Arc::new(jar);
        let client = reqwest::Client::builder()
            .cookie_provider(jar.clone())
            .user_agent("MLM")
            .build()?;

        Ok(MaM { jar, client })
    }

    pub async fn get_torrent_info(&self, hash: &str) -> Result<Option<MaMTorrent>> {
        let resp = self
            .client
            .post("https://www.myanonamouse.net/tor/js/loadSearchJSONbasic.php")
            .json(&json!({
                "description": true,
                "isbn": true,
                "tor": { "hash": hash }
            }))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        println!("resp: {resp:?}");
        if let Ok(resp) = serde_json::from_str::<SearchError>(&resp) {
            if resp.error == "Nothing returned, out of 0" {
                return Ok(None);
            } else {
                return Err(Error::msg(resp.error));
            }
        };
        let mut resp: SearchResult = serde_json::from_str(&resp).context("parse mam response")?;
        println!("resp2: {resp:?}");
        Ok(resp.data.pop())
    }
}
