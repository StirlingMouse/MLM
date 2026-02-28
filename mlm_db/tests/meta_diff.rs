use std::collections::BTreeMap;
use time::Duration;

use mlm_db::*;

#[test]
fn torrent_meta_diff_detects_field_changes() {
    let mut ids_a = BTreeMap::new();
    ids_a.insert(ids::MAM.to_string(), "1".to_string());

    let meta_a = TorrentMeta {
        ids: ids_a,
        vip_status: None,
        cat: None,
        media_type: MediaType::Ebook,
        main_cat: Some(MainCat::Fiction),
        categories: vec!["CatA".to_string()],
        tags: vec![],
        language: Some(Language::English),
        flags: Some(FlagBits::new(0)),
        filetypes: vec!["pdf".to_string()],
        num_files: 1,
        size: Size::from_bytes(1024),
        title: "Title A".to_string(),
        edition: None,
        description: String::new(),
        authors: vec!["Author A".to_string()],
        narrators: vec![],
        series: vec![],
        source: MetadataSource::Mam,
        uploaded_at: Timestamp::now(),
    };

    let mut ids_b = BTreeMap::new();
    ids_b.insert(ids::MAM.to_string(), "2".to_string());
    ids_b.insert(ids::ASIN.to_string(), "ASIN123".to_string());

    let meta_b = TorrentMeta {
        ids: ids_b,
        vip_status: Some(VipStatus::Permanent),
        cat: None,
        media_type: MediaType::Audiobook,
        main_cat: Some(MainCat::Nonfiction),
        categories: vec!["CatB".to_string()],
        tags: vec![],
        language: Some(Language::Other),
        flags: Some(FlagBits::new(0b0000_0011)),
        filetypes: vec!["epub".to_string()],
        num_files: 2,
        size: Size::from_bytes(2048),
        title: "Title B".to_string(),
        edition: Some(("2nd edition".to_string(), 2)),
        description: String::new(),
        authors: vec!["Author B".to_string()],
        narrators: vec!["Narrator".to_string()],
        series: vec![Series {
            name: "Series X".to_string(),
            entries: SeriesEntries::new(vec![]),
        }],
        source: MetadataSource::Manual,
        uploaded_at: Timestamp::now(),
    };

    let diffs = meta_a.diff(&meta_b);

    // Collect field names as strings since TorrentMetaField doesn't derive Eq/Hash in all
    // versions; Display is stable and used by the application.
    let names: Vec<String> = diffs.iter().map(|d| d.field.to_string()).collect();

    // Expect at least these diffs to be present
    let expected = [
        "ids",
        "vip",
        "media_type",
        "main_cat",
        "categories",
        "language",
        "flags",
        "filetypes",
        "size",
        "title",
        "edition",
        "authors",
        "narrators",
        "series",
        "source",
    ];

    for &e in &expected {
        assert!(names.contains(&e.to_string()), "missing diff for field {e}");
    }
}

#[test]
fn meta_diff_strict_checks() {
    let mut ids_a = BTreeMap::new();
    ids_a.insert(ids::MAM.to_string(), "1".to_string());

    let mut ids_b = BTreeMap::new();
    ids_b.insert(ids::MAM.to_string(), "2".to_string());
    ids_b.insert(ids::ASIN.to_string(), "ASIN123".to_string());

    let size_a = Size::from_bytes(1_024);
    let size_b = Size::from_bytes(2_048);

    let meta_a = TorrentMeta {
        ids: ids_a.clone(),
        vip_status: None,
        cat: None,
        media_type: MediaType::Ebook,
        main_cat: Some(MainCat::Fiction),
        categories: vec!["CatA".to_string()],
        tags: vec![],
        language: Some(Language::English),
        flags: Some(FlagBits::new(0)),
        filetypes: vec!["pdf".to_string()],
        num_files: 1,
        size: size_a,
        title: "Title A".to_string(),
        edition: None,
        description: String::new(),
        authors: vec!["Author A".to_string()],
        narrators: vec![],
        series: vec![],
        source: MetadataSource::Mam,
        uploaded_at: Timestamp::now(),
    };

    let meta_b = TorrentMeta {
        ids: ids_b.clone(),
        vip_status: Some(VipStatus::Permanent),
        cat: None,
        media_type: MediaType::Audiobook,
        main_cat: Some(MainCat::Nonfiction),
        categories: vec!["CatB".to_string()],
        tags: vec![],
        language: Some(Language::Other),
        flags: Some(FlagBits::new(0b11)),
        filetypes: vec!["epub".to_string()],
        num_files: 2,
        size: size_b,
        title: "Title B".to_string(),
        edition: Some(("2nd edition".to_string(), 2)),
        description: String::new(),
        authors: vec!["Author B".to_string()],
        narrators: vec!["Narrator".to_string()],
        series: vec![Series {
            name: "Series X".to_string(),
            entries: SeriesEntries::new(vec![]),
        }],
        source: MetadataSource::Manual,
        uploaded_at: Timestamp::now(),
    };

    let diffs = meta_a.diff(&meta_b);

    // ensure we have at least the expected number of diffs
    assert!(
        diffs.len() >= 10,
        "expected many diffs but got {}",
        diffs.len()
    );

    // check specific diffs content
    let get = |field: &str| {
        diffs
            .iter()
            .find(|d| d.field.to_string() == field)
            .unwrap_or_else(|| panic!("missing field {}", field))
    };

    let ids = get("ids");
    assert_eq!(ids.from, format!("{:?}", ids_a));
    assert_eq!(ids.to, format!("{:?}", ids_b));

    let size = get("size");
    assert_eq!(size.from, format!("{}", size_a));
    assert_eq!(size.to, format!("{}", size_b));

    let title = get("title");
    assert_eq!(title.from, "Title A");
    assert_eq!(title.to, "Title B");
}

#[test]
fn vip_expiry() {
    let mut ids = BTreeMap::new();
    ids.insert(ids::MAM.to_string(), "1".to_string());

    let base = TorrentMeta {
        ids: ids.clone(),
        vip_status: None,
        cat: None,
        media_type: MediaType::Ebook,
        main_cat: Some(MainCat::Fiction),
        categories: vec!["Cat".to_string()],
        tags: vec![],
        language: Some(Language::English),
        flags: Some(FlagBits::new(0)),
        filetypes: vec!["pdf".to_string()],
        num_files: 1,
        size: Size::from_bytes(1024),
        title: "Title".to_string(),
        edition: None,
        description: String::new(),
        authors: vec!["Author".to_string()],
        narrators: vec![],
        series: vec![],
        source: MetadataSource::Mam,
        uploaded_at: Timestamp::now(),
    };

    let mut a = base.clone();
    let past_date = (Timestamp::now().0 - Duration::days(30)).date();
    a.vip_status = Some(VipStatus::Temp(past_date));

    let mut b = base;
    b.vip_status = Some(VipStatus::NotVip);

    let diffs = a.diff(&b);
    let names: Vec<String> = diffs.iter().map(|d| d.field.to_string()).collect();
    assert!(
        !names.contains(&"vip".to_string()),
        "vip diff should be suppressed when going from expired temp -> NotVip"
    );
}
