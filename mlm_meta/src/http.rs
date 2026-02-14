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
        use reqwest::header::{
            ACCEPT, ACCEPT_LANGUAGE, CONNECTION, HeaderMap, HeaderName, HeaderValue,
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
        );
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("en,en-US;q=0.9,en-GB;q=0.8,sv;q=0.7"),
        );
        headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
        headers.insert(
            HeaderName::from_static("dnt"),
            HeaderValue::from_static("1"),
        );
        headers.insert(
            HeaderName::from_static("priority"),
            HeaderValue::from_static("u=0, i"),
        );
        headers.insert(
            HeaderName::from_static("sec-fetch-dest"),
            HeaderValue::from_static("document"),
        );
        headers.insert(
            HeaderName::from_static("sec-fetch-mode"),
            HeaderValue::from_static("navigate"),
        );
        headers.insert(
            HeaderName::from_static("sec-fetch-site"),
            HeaderValue::from_static("none"),
        );
        headers.insert(
            HeaderName::from_static("sec-fetch-user"),
            HeaderValue::from_static("?1"),
        );

        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36")
                .default_headers(headers)
                .gzip(true)
                .build()
                .unwrap()
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
