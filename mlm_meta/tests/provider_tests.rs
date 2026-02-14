use mlm_db::TorrentMeta;
use mlm_meta::providers::FakeProvider;
use mlm_meta::traits::Provider;

#[tokio::test]
async fn fake_provider_returns_meta() {
    let meta = TorrentMeta {
        title: "The Test Book".to_string(),
        authors: vec!["Jane Doe".to_string()],
        description: "desc".to_string(),
        ..Default::default()
    };

    let provider = FakeProvider::new("fake", Some(meta.clone()));
    let mut q: TorrentMeta = Default::default();
    q.ids
        .insert("isbn".to_string(), "9781234567897".to_string());
    let got = provider.fetch(&q).await.expect("should return meta");
    assert_eq!(got.title, meta.title);
    assert_eq!(got.authors, meta.authors);
}

#[tokio::test]
async fn fake_provider_not_found() {
    let provider = FakeProvider::new("fake", None);
    let q = TorrentMeta {
        title: "nope".to_string(),
        ..Default::default()
    };
    let res = provider.fetch(&q).await;
    assert!(res.is_err());
}
