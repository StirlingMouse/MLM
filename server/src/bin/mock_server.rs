#![recursion_limit = "256"]
//! Mock server for e2e tests: serves MaM API and qBittorrent WebUI API.
//! Listens on port 3997 by default (override with MOCK_PORT env var).
// Increased recursion limit for serde_json::json! macro with large objects.
use axum::{
    Router,
    extract::Query,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Json},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;

// ── qBittorrent mock ──────────────────────────────────────────────────────────

async fn qbit_login() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_static("SID=mock-session-id; Path=/"),
    );
    (headers, "Ok.")
}

async fn qbit_version() -> impl IntoResponse {
    Json("5.0.0")
}

#[derive(Deserialize)]
struct HashesQuery {
    hashes: Option<String>,
    hash: Option<String>,
}

async fn qbit_torrents_info(Query(q): Query<HashesQuery>) -> impl IntoResponse {
    // Only return a torrent when the expected hash is requested.
    let requested = q.hashes.as_deref().unwrap_or("");
    if !requested.is_empty() && !requested.split('|').any(|h| h == "torrent-001") {
        return Json(json!([]));
    }
    Json(json!([{
        "hash": "torrent-001",
        "name": "Test Book 001",
        "state": "stalledUP",
        "category": "Audiobooks",
        "tags": "mlm",
        "size": 310000000i64,
        "total_size": 310000000i64,
        "uploaded": 620000000i64,
        "downloaded": 310000000i64,
        "ratio": 2.0f32,
        "progress": 1.0f32,
        "dlspeed": 0i64,
        "num_seeds": 5i64,
        "num_leechs": 0i64,
        "num_complete": 10i64,
        "num_incomplete": 0i64,
        "eta": 0i64,
        "added_on": 1700000000i64,
        "completion_on": 1700001000i64,
        "save_path": "/downloads/",
        "content_path": "/downloads/Test Book 001",
        "root_path": "/downloads/Test Book 001",
        "download_path": "",
        "amount_left": 0i64,
        "completed": 310000000i64,
        "dl_limit": -1i64,
        "up_limit": -1i64,
        "downloaded_session": 0i64,
        "uploaded_session": 0i64,
        "upspeed": 0i64,
        "time_active": 86400i64,
        "seeding_time": 86400i64,
        "seeding_time_limit": -2i64,
        "max_seeding_time": -1i64,
        "inactive_seeding_time_limit": -2i64,
        "max_inactive_seeding_time": -1i64,
        "ratio_limit": -2.0f32,
        "max_ratio": -1.0f32,
        "priority": -1i64,
        "reannounce": 1800i64,
        "last_activity": 1700100000i64,
        "seen_complete": 1700001000i64,
        "tracker": "http://tracker.myanonamouse.net",
        "trackers_count": 1i64,
        "magnet_uri": "",
        "infohash_v1": "aabbccddeeff001122334455667788990011223344",
        "infohash_v2": "",
        "comment": "",
        "auto_tmm": false,
        "availability": 1.0f64,
        "f_l_piece_prio": false,
        "force_start": false,
        "has_metadata": true,
        "seq_dl": false,
        "super_seeding": false,
        "private": true,
        "popularity": 1.0f64
    }]))
}

async fn qbit_trackers(Query(q): Query<HashesQuery>) -> impl IntoResponse {
    let hash = q.hash.as_deref().unwrap_or("");
    if hash != "torrent-001" {
        return (StatusCode::NOT_FOUND, Json(json!([])));
    }
    (
        StatusCode::OK,
        Json(json!([
            {
                "url": "** [DHT] **",
                "status": 0i64,
                "tier": -1i64,
                "num_peers": 5i64,
                "num_seeds": 5i64,
                "num_leeches": 0i64,
                "num_downloaded": -1i64,
                "msg": ""
            },
            {
                "url": "http://tracker.myanonamouse.net/announce",
                "status": 2i64,
                "tier": 0i64,
                "num_peers": 5i64,
                "num_seeds": 5i64,
                "num_leeches": 0i64,
                "num_downloaded": 50i64,
                "msg": ""
            }
        ])),
    )
}

async fn qbit_files(Query(q): Query<HashesQuery>) -> impl IntoResponse {
    let hash = q.hash.as_deref().unwrap_or("");
    if hash != "torrent-001" {
        return (StatusCode::NOT_FOUND, Json(json!([])));
    }
    (
        StatusCode::OK,
        Json(json!([
            {
                "index": 0i64,
                "name": "Test Book 001.m4b",
                "size": 310000000i64,
                "progress": 1.0f64,
                "priority": 1,
                "is_seed": true,
                "piece_range": [0i64, 295i64],
                "availability": 1.0f64
            }
        ])),
    )
}

async fn qbit_categories() -> impl IntoResponse {
    Json(json!({
        "Audiobooks": { "name": "Audiobooks", "savePath": "/downloads/audiobooks/" },
        "Ebooks": { "name": "Ebooks", "savePath": "/downloads/ebooks/" }
    }))
}

async fn qbit_tags() -> impl IntoResponse {
    Json(json!(["mlm", "fiction"]))
}

// ── MaM mock ──────────────────────────────────────────────────────────────────

async fn mam_check_cookie() -> impl IntoResponse {
    Json(json!({"Success": "You are logged in as: testuser"}))
}

async fn mam_user_info() -> impl IntoResponse {
    Json(json!({
        "uid": 12345u64,
        "username": "testuser",
        "downloaded_bytes": 500_000_000_000.0f64,
        "uploaded_bytes": 1_000_000_000_000.0f64,
        "seedbonus": 50000i64,
        "wedges": 3u64,
        "unsat": {
            "count": 2u64,
            "red": false,
            "size": null,
            "limit": 10u64
        }
    }))
}

#[derive(Debug, Deserialize)]
struct MockMaMSearchRequest {
    #[serde(default)]
    perpage: Option<usize>,
    #[serde(default)]
    tor: MockMaMSearchTor,
}

#[derive(Debug, Default, Deserialize)]
struct MockMaMSearchTor {
    #[serde(default)]
    text: String,
    #[serde(rename = "startNumber", default)]
    start_number: usize,
}

fn mock_search_result(index: usize) -> serde_json::Value {
    let id = 99_000u64 + index as u64;
    let month = (index % 12) + 1;
    let day = (index % 28) + 1;
    let seeders = 15u64 + (index % 10) as u64;
    let leechers = (index % 4) as u64;
    let comments = (index % 6) as u64;
    let snatches = 100u64 + index as u64;
    let size_mib = 300.0 + index as f64;

    json!({
        "id": id,
        "added": format!("2024-{month:02}-{day:02} 10:00:00"),
        "author_info": format!(r#"{{"{index}":"Test Author {index:03}"}}"#),
        "browseflags": 0u8,
        "main_cat": 13u8,
        "category": 39u64,
        "mediatype": 1u8,
        "maincat": 1u8,
        "categories": "[]",
        "catname": "Audiobook - Fantasy",
        "cat": "audiobook",
        "comments": comments,
        "filetype": if index % 2 == 0 { "m4b" } else { "mp3" },
        "fl_vip": 0,
        "free": if index % 5 == 0 { 1 } else { 0 },
        "lang_code": "en",
        "language": 1u8,
        "leechers": leechers,
        "my_snatched": 0,
        "narrator_info": format!(r#"{{"{index}":"Test Narrator {index:03}"}}"#),
        "numfiles": 1u64,
        "owner": 12345u64,
        "owner_name": "uploader",
        "ownership": "[]",
        "personal_freeleech": 0,
        "seeders": seeders,
        "series_info": "{}",
        "size": format!("{size_mib:.2} MiB"),
        "tags": "fantasy test",
        "times_completed": snatches,
        "thumbnail": null,
        "title": format!("Mock Search Result {index:03}"),
        "vip": 0,
        "vip_expire": 0u64,
        "w": 0u64
    })
}

async fn mam_search(payload: Option<Json<MockMaMSearchRequest>>) -> impl IntoResponse {
    let payload = payload
        .map(|Json(payload)| payload)
        .unwrap_or(MockMaMSearchRequest {
            perpage: Some(100),
            tor: MockMaMSearchTor::default(),
        });
    let query = payload.tor.text.trim().to_lowercase();
    let total = if query.is_empty() {
        0
    } else if query.contains("test book") || query.contains("mock search") {
        205usize
    } else {
        2usize
    };
    let perpage = payload.perpage.unwrap_or(100).clamp(1, 100);
    let start = payload.tor.start_number.min(total);
    let end = (start + perpage).min(total);
    let data = (start + 1..=end)
        .map(mock_search_result)
        .collect::<Vec<_>>();

    Json(json!({
        "total": total,
        "perpage": perpage,
        "start": start,
        "found": total,
        "data": data
    }))
}

// ── Router ────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("MOCK_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3997);

    let app = Router::new()
        // qBittorrent endpoints
        .route("/api/v2/auth/login", post(qbit_login))
        .route("/api/v2/app/version", get(qbit_version))
        .route("/api/v2/torrents/info", get(qbit_torrents_info))
        .route("/api/v2/torrents/trackers", get(qbit_trackers))
        .route("/api/v2/torrents/files", get(qbit_files))
        .route("/api/v2/torrents/categories", get(qbit_categories))
        .route("/api/v2/torrents/tags", get(qbit_tags))
        // MaM endpoints
        .route("/json/checkCookie.php", get(mam_check_cookie))
        .route("/jsonLoad.php", get(mam_user_info))
        .route("/tor/js/loadSearchJSONbasic.php", post(mam_search));

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    eprintln!("mock_server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
