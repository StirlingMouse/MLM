use mlm_core::config::{Cost, EditionFilter, GoodreadsList, Grab, TorrentFilter};
use mlm_db::AudiobookCategory;
use mlm_mam::enums::Categories;

#[test]
fn test_list_id_with_shelf_in_query() {
    let list = GoodreadsList {
        url: "https://goodreads.com/user/12345?foo=bar&shelf=to-read".to_string(),
        name: None,
        prefer_format: None,
        grab: vec![],
        search_interval: None,
        unsat_buffer: None,
        wedge_buffer: None,
        dry_run: false,
    };

    let id = list.list_id().expect("should parse list id");
    assert_eq!(id, "12345:to-read");
}

#[test]
fn test_list_id_without_shelf() {
    let list = GoodreadsList {
        url: "https://goodreads.com/user/67890".to_string(),
        name: None,
        prefer_format: None,
        grab: vec![],
        search_interval: None,
        unsat_buffer: None,
        wedge_buffer: None,
        dry_run: false,
    };

    let id = list.list_id().expect("should parse list id");
    assert_eq!(id, "67890:");
}

#[test]
fn test_list_id_shelf_first_param() {
    let list = GoodreadsList {
        url: "https://goodreads.com/user/abcde?shelf=owned&x=1".to_string(),
        name: None,
        prefer_format: None,
        grab: vec![],
        search_interval: None,
        unsat_buffer: None,
        wedge_buffer: None,
        dry_run: false,
    };

    let id = list.list_id().expect("should parse list id");
    assert_eq!(id, "abcde:owned");
}

#[test]
fn test_allow_audio_true_when_none_or_nonempty() {
    // audio = None -> allowed
    let g1 = GoodreadsList {
        url: "https://goodreads.com/user/1".to_string(),
        name: None,
        prefer_format: None,
        grab: vec![Grab {
            cost: Cost::default(),
            filter: TorrentFilter::default(),
            edition: EditionFilter::default(),
        }],
        search_interval: None,
        unsat_buffer: None,
        wedge_buffer: None,
        dry_run: false,
    };
    assert!(g1.allow_audio());

    // audio = Some(non-empty) -> allowed
    let cats = Categories {
        audio: Some(vec![AudiobookCategory::GeneralFiction]),
        ..Default::default()
    };
    let g2 = GoodreadsList {
        url: "https://goodreads.com/user/2".to_string(),
        name: None,
        prefer_format: None,
        grab: vec![Grab {
            cost: Cost::default(),
            filter: TorrentFilter::default(),
            edition: EditionFilter {
                categories: cats,
                ..Default::default()
            },
        }],
        search_interval: None,
        unsat_buffer: None,
        wedge_buffer: None,
        dry_run: false,
    };
    assert!(g2.allow_audio());
}

#[test]
fn test_allow_audio_false_when_all_grabs_empty_audio() {
    let cats = Categories {
        audio: Some(vec![]),
        ..Default::default()
    };

    let list = GoodreadsList {
        url: "https://goodreads.com/user/3".to_string(),
        name: None,
        prefer_format: None,
        grab: vec![Grab {
            cost: Cost::default(),
            filter: TorrentFilter::default(),
            edition: EditionFilter {
                categories: cats,
                ..Default::default()
            },
        }],
        search_interval: None,
        unsat_buffer: None,
        wedge_buffer: None,
        dry_run: false,
    };

    assert!(!list.allow_audio());
}

#[test]
fn test_allow_ebook_behaviour() {
    // ebook = None -> allowed
    let g1 = GoodreadsList {
        url: "https://goodreads.com/user/4".to_string(),
        name: None,
        prefer_format: None,
        grab: vec![Grab {
            cost: Cost::default(),
            filter: TorrentFilter::default(),
            edition: EditionFilter::default(),
        }],
        search_interval: None,
        unsat_buffer: None,
        wedge_buffer: None,
        dry_run: false,
    };
    assert!(g1.allow_ebook());

    // ebook = Some(empty) -> disallowed
    let cats = Categories {
        ebook: Some(vec![]),
        ..Default::default()
    };
    let g2 = GoodreadsList {
        url: "https://goodreads.com/user/5".to_string(),
        name: None,
        prefer_format: None,
        grab: vec![Grab {
            cost: Cost::default(),
            filter: TorrentFilter::default(),
            edition: EditionFilter {
                categories: cats,
                ..Default::default()
            },
        }],
        search_interval: None,
        unsat_buffer: None,
        wedge_buffer: None,
        dry_run: false,
    };
    assert!(!g2.allow_ebook());
}
