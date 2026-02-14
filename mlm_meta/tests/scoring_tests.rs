use std::sync::Arc;

use mlm_meta::{HttpClient, providers::romanceio::RomanceIo};

const SAMPLE_ROMANCE_HTML: &str = r#"
<html><head>
<script type="application/ld+json">
{
  "@graph": [{
    "name": "Of Ink and Alchemy",
    "author": [{"name":"Sloane Harper"}],
    "description": "A moody romance with friends to lovers and age gap dynamics."
  }]
}
</script>
</head><body>
<ul id="valid-topics-list">
  <li><a class="topic">Contemporary</a></li>
  <li><a class="topic">Dark Romance</a></li>
  <li><a class="topic">Friends to Lovers</a></li>
  <li><a class="topic">Age Gap</a></li>
</ul>
</body></html>
"#;

#[test]
fn parse_book_html_smoke() {
    struct DummyClient;
    #[async_trait::async_trait]
    impl HttpClient for DummyClient {
        async fn get(&self, _url: &str) -> anyhow::Result<String> {
            anyhow::bail!("not used")
        }
        async fn post(
            &self,
            _url: &str,
            _body: Option<&str>,
            _headers: &[(&str, &str)],
        ) -> anyhow::Result<String> {
            anyhow::bail!("not used")
        }
    }

    let provider = RomanceIo::with_client(Arc::new(DummyClient));
    let meta = provider.parse_book_html(SAMPLE_ROMANCE_HTML).unwrap();
    assert!(!meta.title.is_empty());

    assert!(!meta.title.is_empty());
    assert!(!meta.authors.is_empty());
}
