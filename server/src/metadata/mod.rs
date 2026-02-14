use crate::stats::Context;
use anyhow::Result;
use mlm_db::DatabaseExt as _;
use mlm_db::{Event, EventType, MetadataSource, TorrentMeta};
use mlm_meta::providers::{Hardcover, RomanceIo};
use mlm_meta::traits::Provider;
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use tracing::instrument;
pub mod mam_meta;

pub struct MetadataService {
    // Each provider can have its own request timeout
    providers: Vec<(Arc<dyn Provider>, Duration)>,
    #[allow(dead_code)]
    default_timeout: Duration,
}

/// Simple provider configuration used by the server.
pub enum ProviderSetting {
    Hardcover {
        enabled: bool,
        timeout_secs: Option<u64>,
        api_key: Option<String>,
    },
    RomanceIo {
        enabled: bool,
        timeout_secs: Option<u64>,
    },
}

impl MetadataService {
    pub fn new(providers: Vec<(Arc<dyn Provider>, Duration)>, default_timeout: Duration) -> Self {
        Self {
            providers,
            default_timeout,
        }
    }

    /// Build a MetadataService from a list of ProviderSetting.
    pub fn from_settings(settings: &[ProviderSetting], default_timeout: Duration) -> Self {
        let mut providers: Vec<(Arc<dyn Provider>, Duration)> = Vec::new();
        for s in settings {
            match s {
                ProviderSetting::Hardcover {
                    enabled,
                    timeout_secs,
                    api_key,
                } => {
                    if !enabled {
                        continue;
                    }
                    let to = timeout_secs
                        .map(Duration::from_secs)
                        .unwrap_or(default_timeout);
                    providers.push((Arc::new(Hardcover::new(api_key.clone())), to));
                }
                ProviderSetting::RomanceIo {
                    enabled,
                    timeout_secs,
                } => {
                    if !enabled {
                        continue;
                    }
                    let to = timeout_secs
                        .map(Duration::from_secs)
                        .unwrap_or(default_timeout);
                    providers.push((Arc::new(RomanceIo::new()), to));
                }
            }
        }
        Self::new(providers, default_timeout)
    }

    pub fn enabled_providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|(p, _)| p.id().to_string())
            .collect()
    }

    #[instrument(skip(self, ctx))]
    pub async fn fetch_and_persist(
        &self,
        ctx: &Context,
        query: TorrentMeta,
    ) -> Result<TorrentMeta> {
        // Query providers in parallel with timeout and pick first successful
        let mut handles = vec![];
        for (p, to) in &self.providers {
            let p = p.clone();
            let q = query.clone();
            let to = *to;
            handles.push(tokio::spawn(async move {
                let r = timeout(to, p.fetch(&q)).await;
                match r {
                    Ok(Ok(m)) => Ok((p.id().to_string(), m)),
                    Ok(Err(e)) => Err(anyhow::anyhow!(e)),
                    Err(_) => Err(anyhow::anyhow!("timeout")),
                }
            }));
        }

        let mut best: Option<(String, TorrentMeta)> = None;

        for h in handles {
            match h.await {
                Ok(Ok((id, meta))) => {
                    // pick first for now
                    best = Some((id, meta));
                    break;
                }
                Ok(Err(e)) => {
                    tracing::debug!(error=?e, "provider task returned error");
                }
                Err(join_err) => {
                    tracing::debug!(error=?join_err, "provider task panicked or was cancelled");
                }
            }
        }

        let (provider_id, meta): (String, TorrentMeta) = match best {
            Some(v) => v,
            None => return Err(anyhow::anyhow!("no provider matched")),
        };

        // Provider already returns a TorrentMeta; use it and mark source
        let mut tmeta: TorrentMeta = meta;
        tmeta.source = MetadataSource::Match;

        // Persist: write a SelectedTorrent or Torrent depending on context.
        // Here we insert an Event to record metadata update and return the meta.
        let ev = Event {
            id: mlm_db::Uuid::new(),
            torrent_id: None,
            mam_id: None,
            created_at: mlm_db::Timestamp::now(),
            event: EventType::Updated {
                fields: vec![],
                source: (MetadataSource::Match, provider_id.clone()),
            },
        };

        // Insert event into DB using async rw transaction helper from mlm_db
        let (guard, rw) = ctx.db.rw_async().await?;
        rw.insert(ev)?;
        rw.commit()?;
        drop(guard);

        Ok(tmeta)
    }

    /// Fetch using an explicit provider id. This looks up the provider in the
    /// registered list and executes it with its configured timeout. Returns
    /// the provider-provided TorrentMeta on success.
    #[instrument(skip(self, _ctx))]
    pub async fn fetch_provider(
        &self,
        _ctx: &Context,
        query: TorrentMeta,
        provider_id: &str,
    ) -> Result<TorrentMeta> {
        // find provider
        let mut found: Option<(Arc<dyn Provider>, Duration)> = None;
        for (p, to) in &self.providers {
            if p.id() == provider_id {
                found = Some((p.clone(), *to));
                break;
            }
        }

        let (p, to) = match found {
            Some(v) => v,
            None => anyhow::bail!("unknown provider id: {}", provider_id),
        };

        // run with timeout
        let r = timeout(to, p.fetch(&query)).await;
        let meta = match r {
            Ok(Ok(m)) => m,
            Ok(Err(e)) => return Err(anyhow::anyhow!(e)),
            Err(_) => return Err(anyhow::anyhow!("timeout")),
        };

        let mut tmeta: TorrentMeta = meta;
        tmeta.source = MetadataSource::Match;
        Ok(tmeta)
    }
}
