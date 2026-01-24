use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use once_cell::sync::Lazy;
use qbit::{
    models::Torrent,
    parameters::{AddTorrent, TorrentListParams},
};
use tokio::sync::RwLock;

use crate::config::{Config, QbitConfig};

const CATEGORY_CACHE_TTL_SECS: u64 = 60;

#[derive(Clone)]
pub struct CategoryCache {
    cache: Arc<RwLock<HashMap<String, (HashMap<String, ()>, Instant)>>>,
}

impl CategoryCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_or_fetch(&self, qbit: &qbit::Api, url: &str) -> Result<HashMap<String, ()>> {
        let now = Instant::now();
        let cache = self.cache.read().await;

        if let Some((categories, cached_at)) = cache.get(url) {
            if now.duration_since(*cached_at) < Duration::from_secs(CATEGORY_CACHE_TTL_SECS) {
                return Ok(categories.clone());
            }
        }
        drop(cache);

        let categories: HashMap<String, ()> = qbit.categories().await?.into_keys().map(|k| (k, ())).collect();
        let mut cache = self.cache.write().await;
        cache.insert(url.to_string(), (categories.clone(), now));

        Ok(categories)
    }

    pub async fn invalidate(&self, url: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(url);
    }
}

impl Default for CategoryCache {
    fn default() -> Self {
        Self::new()
    }
}

static CATEGORY_CACHE: Lazy<CategoryCache> = Lazy::new(CategoryCache::new);

pub async fn ensure_category_exists(
    qbit: &qbit::Api,
    url: &str,
    category: &str,
) -> Result<()> {
    if category.is_empty() {
        return Ok(());
    }

    let categories = CATEGORY_CACHE.get_or_fetch(qbit, url).await?;

    if categories.contains_key(category) {
        return Ok(());
    }

    if let Err(e) = qbit.create_category(category, "").await {
        qbit.create_category(category, "").await.map_err(|_| e)?;
    }

    CATEGORY_CACHE.invalidate(url).await;
    Ok(())
}

pub async fn add_torrent_with_category(
    qbit: &qbit::Api,
    url: &str,
    add_torrent: AddTorrent,
) -> Result<()> {
    if let Some(ref category) = add_torrent.category {
        if !category.is_empty() {
            ensure_category_exists(qbit, url, category).await?;
        }
    }

    qbit.add_torrent(add_torrent).await.map_err(|e| anyhow::Error::new(e))
}

pub async fn get_torrent<'a, 'b>(
    config: &'a Config,
    hash: &str,
) -> Result<Option<(Torrent, qbit::Api, &'a QbitConfig)>> {
    for qbit_conf in config.qbittorrent.iter() {
        let Ok(qbit) = qbit::Api::new_login_username_password(
            &qbit_conf.url,
            &qbit_conf.username,
            &qbit_conf.password,
        )
        .await
        else {
            continue;
        };
        let Some(torrent) = qbit
            .torrents(Some(TorrentListParams {
                hashes: Some(vec![hash.to_string()]),
                ..TorrentListParams::default()
            }))
            .await?
            .into_iter()
            .next()
        else {
            continue;
        };
        return Ok(Some((torrent, qbit, qbit_conf)));
    }
    Ok(None)
}
