use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Error, Result};
use cookie::Cookie;
use native_db::Database;
use reqwest::Url;
use reqwest_cookie_store::CookieStoreRwLock;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};

use crate::{config::Config, data};

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

pub struct MaM<'a> {
    jar: Arc<CookieStoreRwLock>,
    client: reqwest::Client,
    db: &'a Database<'a>,
}

impl<'a> MaM<'a> {
    pub async fn new(config: &Config, db: &'a Database<'a>) -> Result<MaM<'a>> {
        let jar: CookieStoreRwLock = Default::default();
        let url = "https://www.myanonamouse.net".parse::<Url>().unwrap();

        let stored_mam_id = db
            .r_transaction()
            .and_then(|r| r.get().primary::<data::Config>("mam_id"))
            .ok()
            .flatten()
            .map(|c| c.value);
        if let Some(stored_mam_id) = &stored_mam_id {
            println!("Restoring mam_id: {}", stored_mam_id);
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
        self.store_cookies();
        Ok(resp.data.pop())
    }

    fn store_cookies(&self) {
        let url = "https://www.myanonamouse.net".parse::<Url>().unwrap();
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
                    println!("stored new mam_id: {}", cookie.value());
                }
            }
        }
    }
}
