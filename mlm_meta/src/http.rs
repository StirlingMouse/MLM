use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn get(&self, url: &str) -> Result<String>;

    async fn post(&self, url: &str, body: Option<&str>, headers: &[(&str, &str)])
    -> Result<String>;
}

pub struct ReqwestClient {
    client: Client,
}

impl ReqwestClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpClient for ReqwestClient {
    async fn get(&self, url: &str) -> Result<String> {
        let res = self.client.get(url).send().await?.text().await?;
        Ok(res)
    }

    async fn post(
        &self,
        url: &str,
        body: Option<&str>,
        headers: &[(&str, &str)],
    ) -> Result<String> {
        let mut req = self.client.post(url);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        if let Some(b) = body {
            req = req.body(b.to_string());
        }
        let res = req.send().await?.text().await?;
        Ok(res)
    }
}
