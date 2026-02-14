use anyhow::Result;
use mlm_meta::http::HttpClient;
use std::sync::Arc;

pub struct MockClient;

#[async_trait::async_trait]
impl HttpClient for MockClient {
    async fn get(&self, url: &str) -> Result<String> {
        let u = url::Url::parse(url).map_err(|e| anyhow::anyhow!(e))?;
        if !u.host_str().is_some_and(|h| h.contains("romance.io")) {
            return Err(anyhow::anyhow!("unexpected host in test fetch"));
        }
        if u.path().starts_with("/json/search_books") {
            return Ok(r#"{
  "success": true,
  "books": [
    {
      "_id":"68b95a390bc0cee156edaf2b",
      "info":{"title":"Of Ink and Alchemy"},
      "authors":[{"name":"Sloane St. James"}],
      "url":"/books/68b95a390bc0cee156edaf2b/of-ink-and-alchemy-sloane-st-james"
    }
  ]
}"#
            .to_string());
        }
        if u.path().starts_with("/json/search_authors") {
            return Ok(r#"{ "success": true, "authors": [] }"#.to_string());
        }
        if u.path().starts_with("/search") {
            return Ok("<html><body>search</body></html>".to_string());
        }

        Ok(r#"
<html><head>
<script type="application/ld+json">
{
  "@graph": [{
    "name": "Of Ink and Alchemy",
    "author": [{"name":"Sloane St. James"}],
    "description": "A dark contemporary romance with friends to lovers."
  }]
}
</script>
</head><body>
<ul id="valid-topics-list">
  <li><a class="topic">Contemporary</a></li>
  <li><a class="topic">Dark Romance</a></li>
  <li><a class="topic">Age Difference</a></li>
  <li><a class="topic">Friends to Lovers</a></li>
</ul>
</body></html>
"#
        .to_string())
    }

    async fn post(
        &self,
        _url: &str,
        _body: Option<&str>,
        _headers: &[(&str, &str)],
    ) -> Result<String> {
        Err(anyhow::anyhow!("post not implemented in mock"))
    }
}

pub fn boxed() -> Arc<dyn HttpClient> {
    Arc::new(MockClient)
}
