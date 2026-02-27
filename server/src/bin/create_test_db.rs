/// Creates a test database with fake data for e2e Playwright tests.
/// Usage: create_test_db <output_path>
use mlm_db::{
    DuplicateTorrent, ErroredTorrent, ErroredTorrentId, Event, EventType, MODELS, MainCat,
    MediaType, MetadataSource, SelectedTorrent, Series, SeriesEntries, SeriesEntry, Size,
    Timestamp, Torrent, TorrentCost, TorrentMeta, Uuid, migrate,
};
use native_db::Builder;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

fn title_search(title: &str) -> String {
    title.to_lowercase()
}

fn main() -> anyhow::Result<()> {
    let path: PathBuf = env::args()
        .nth(1)
        .expect("Usage: create_test_db <output_path>")
        .into();

    if path.exists() {
        std::fs::remove_file(&path)?;
    }

    let db = Builder::new().create(&MODELS, &path)?;
    migrate(&db)?;

    let rw = db.rw_transaction()?;

    // 35 torrents with varied metadata for pagination, sorting, and filter tests
    let authors_pool: &[&[&str]] = &[
        &["Brandon Sanderson"],
        &["Patrick Rothfuss"],
        &["Robin Hobb"],
        &["Terry Pratchett", "Neil Gaiman"],
        &["N.K. Jemisin"],
        &["Joe Abercrombie"],
        &["Ursula K. Le Guin"],
    ];
    let narrators_pool: &[&str] = &[
        "Michael Kramer",
        "Kate Reading",
        "Tim Gerard Reynolds",
        "Nick Podehl",
    ];
    let series_pool: &[Option<(&str, f32)>] = &[
        Some(("The Stormlight Archive", 1.0)),
        Some(("The Stormlight Archive", 2.0)),
        Some(("The Kingkiller Chronicle", 1.0)),
        Some(("The Realm of the Elderlings", 1.0)),
        None,
        None,
        None,
    ];

    let mut torrent_ids: Vec<String> = Vec::new();

    for i in 1u64..=35 {
        let id = format!("torrent-{i:03}");
        let mam_id = 10_000 + i;
        let ai = (i as usize - 1) % authors_pool.len();
        let ni = (i as usize - 1) % narrators_pool.len();
        let si = (i as usize - 1) % series_pool.len();

        let authors: Vec<String> = authors_pool[ai].iter().map(|s| s.to_string()).collect();
        let narrators: Vec<String> = vec![narrators_pool[ni].to_string()];
        let series = match &series_pool[si] {
            Some((name, num)) => vec![Series {
                name: name.to_string(),
                entries: SeriesEntries::new(vec![SeriesEntry::Num(*num)]),
            }],
            None => vec![],
        };

        let title = format!("Test Book {i:03}");
        let size_bytes = 300_000_000 + i * 10_000_000;
        let has_library = i <= 20;

        // torrent-005 is replaced by torrent-006
        let replaced_with = if i == 5 {
            Some(("torrent-006".to_string(), Timestamp::now()))
        } else {
            None
        };

        let mut ids = BTreeMap::new();
        ids.insert(mlm_db::ids::MAM.to_string(), mam_id.to_string());

        let torrent = Torrent {
            id: id.clone(),
            id_is_hash: false,
            mam_id: Some(mam_id),
            library_path: has_library.then(|| PathBuf::from(format!("/library/books/{title}"))),
            library_files: if has_library {
                vec![PathBuf::from(format!("{title}.m4b"))]
            } else {
                vec![]
            },
            linker: has_library.then(|| "test".to_string()),
            category: None,
            selected_audio_format: None,
            selected_ebook_format: None,
            title_search: title_search(&title),
            meta: TorrentMeta {
                ids,
                vip_status: None,
                cat: None,
                media_type: MediaType::Audiobook,
                main_cat: Some(if i % 3 == 0 {
                    MainCat::Nonfiction
                } else {
                    MainCat::Fiction
                }),
                categories: vec![],
                tags: vec![],
                language: None,
                flags: None,
                filetypes: vec!["m4b".to_string()],
                num_files: 1,
                size: Size::from_bytes(size_bytes),
                title: title.clone(),
                edition: None,
                description: format!("Description for {title}"),
                authors,
                narrators,
                series,
                source: MetadataSource::Mam,
                uploaded_at: Timestamp::now(),
            },
            created_at: Timestamp::now(),
            replaced_with,
            library_mismatch: None,
            client_status: None,
        };

        rw.insert(torrent)?;
        torrent_ids.push(id);
    }

    // 5 selected torrents (pending/queued downloads)
    for i in 1u64..=5 {
        let mam_id = 20_000 + i;
        let title = format!("Selected Book {i}");
        let mut ids = BTreeMap::new();
        ids.insert(mlm_db::ids::MAM.to_string(), mam_id.to_string());

        rw.insert(SelectedTorrent {
            mam_id,
            hash: None,
            dl_link: format!("https://www.myanonamouse.net/t/{mam_id}"),
            unsat_buffer: Some(5),
            wedge_buffer: None,
            cost: TorrentCost::PersonalFreeleech,
            category: None,
            tags: vec![],
            title_search: title_search(&title),
            meta: TorrentMeta {
                ids,
                title: title.clone(),
                authors: vec!["Test Author".to_string()],
                media_type: MediaType::Audiobook,
                main_cat: Some(MainCat::Fiction),
                size: Size::from_bytes(200_000_000),
                num_files: 1,
                filetypes: vec!["m4b".to_string()],
                source: MetadataSource::Mam,
                uploaded_at: Timestamp::now(),
                ..Default::default()
            },
            grabber: Some("bookmarks".to_string()),
            created_at: Timestamp::now(),
            started_at: None,
            removed_at: None,
        })?;
    }

    // 5 duplicate torrents
    for i in 1u64..=5 {
        let mam_id = 30_000 + i;
        let title = format!("Duplicate Book {i}");
        let mut ids = BTreeMap::new();
        ids.insert(mlm_db::ids::MAM.to_string(), mam_id.to_string());

        rw.insert(DuplicateTorrent {
            mam_id,
            dl_link: Some(format!("https://www.myanonamouse.net/t/{mam_id}")),
            title_search: title_search(&title),
            meta: TorrentMeta {
                ids,
                title: title.clone(),
                authors: vec!["Dup Author".to_string()],
                media_type: MediaType::Audiobook,
                main_cat: Some(MainCat::Fiction),
                size: Size::from_bytes(150_000_000),
                num_files: 1,
                filetypes: vec!["m4b".to_string()],
                source: MetadataSource::Mam,
                uploaded_at: Timestamp::now(),
                ..Default::default()
            },
            created_at: Timestamp::now(),
            duplicate_of: Some("torrent-001".to_string()),
        })?;
    }

    // 5 errored torrents
    for i in 1u64..=5 {
        let mam_id = 40_000 + i;
        rw.insert(ErroredTorrent {
            id: ErroredTorrentId::Grabber(mam_id),
            title: format!("Errored Book {i}"),
            error: format!("Download failed: connection timeout (attempt {i})"),
            meta: None,
            created_at: Timestamp::now(),
        })?;
    }

    // 10 events
    for i in 0u64..10 {
        let torrent_id = torrent_ids.get(i as usize).cloned();
        let event_type = match i % 3 {
            0 => EventType::Grabbed {
                grabber: Some("bookmarks".to_string()),
                cost: Some(TorrentCost::PersonalFreeleech),
                wedged: false,
            },
            1 => EventType::Linked {
                linker: Some("test".to_string()),
                library_path: PathBuf::from(format!("/library/books/Test Book {i:03}")),
            },
            _ => EventType::RemovedFromTracker,
        };

        rw.insert(Event {
            id: Uuid::new(),
            torrent_id,
            mam_id: None,
            created_at: Timestamp::now(),
            event: event_type,
        })?;
    }

    rw.commit()?;
    println!("Test database created at {}", path.display());
    Ok(())
}
