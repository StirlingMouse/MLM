use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

use anyhow::Result;
use axum::http::HeaderMap;
use mlm_db::{DatabaseExt as _, Flags, Torrent, TorrentMeta, ids, impls::format_serie};
use mlm_parse::clean_html;
use native_db::Database;
use reqwest::{Url, header::AUTHORIZATION};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, instrument, trace};

use crate::config::AudiobookShelfConfig;

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct LibrariesResponse {
    pub libraries: Vec<Library>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct Library {
    pub id: String,
    pub name: String,
    pub folders: Vec<Folder>,
    // #[serde(rename = "displayOrder")]
    // pub display_order: i64,
    // pub icon: String,
    // #[serde(rename = "mediaType")]
    // pub media_type: String,
    // pub provider: String,
    // pub settings: Settings,
    // #[serde(rename = "createdAt")]
    // pub created_at: i64,
    // #[serde(rename = "lastUpdate")]
    // pub last_update: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct Folder {
    pub id: String,
    #[serde(rename = "fullPath")]
    pub full_path: String,
    #[serde(rename = "libraryId")]
    pub library_id: String,
    #[serde(rename = "addedAt")]
    pub added_at: Option<i64>,
}

// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct Settings {
//     #[serde(rename = "coverAspectRatio")]
//     pub cover_aspect_ratio: i64,
//     #[serde(rename = "disableWatcher")]
//     pub disable_watcher: bool,
//     #[serde(rename = "skipMatchingMediaWithAsin")]
//     pub skip_matching_media_with_asin: bool,
//     #[serde(rename = "skipMatchingMediaWithIsbn")]
//     pub skip_matching_media_with_isbn: bool,
// }

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct SearchResponse {
    // pub book: Vec<Book>,
    pub authors: Vec<AuthorItem>,
}

// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct Book {
//     #[serde(rename = "libraryItem")]
//     pub library_item: LibraryItem,
//     // #[serde(rename = "matchKey")]
//     // pub match_key: Option<String>,
//     // #[serde(rename = "matchText")]
//     // pub match_text: Option<String>,
// }

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct AuthorItem {
    pub id: String,
    // pub asin: Option<String>,
    pub name: String,
    // pub description: Option<String>,
    // #[serde(rename = "imagePath")]
    // pub image_path: Option<String>,
    // #[serde(rename = "libraryId")]
    // pub library_id: String,
    // #[serde(rename = "addedAt")]
    // pub added_at: i64,
    // #[serde(rename = "updatedAt")]
    // pub updated_at: i64,
    // #[serde(rename = "numBooks")]
    // pub num_books: i64,
}

// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct LibraryItem {
//     pub id: String,
//     pub ino: String,
//     #[serde(rename = "libraryId")]
//     pub library_id: String,
//     #[serde(rename = "folderId")]
//     pub folder_id: String,
//     pub path: String,
//     #[serde(rename = "relPath")]
//     pub rel_path: String,
//     #[serde(rename = "isFile")]
//     pub is_file: bool,
//     #[serde(rename = "mtimeMs")]
//     pub mtime_ms: i64,
//     #[serde(rename = "ctimeMs")]
//     pub ctime_ms: i64,
//     #[serde(rename = "birthtimeMs")]
//     pub birthtime_ms: i64,
//     #[serde(rename = "addedAt")]
//     pub added_at: i64,
//     #[serde(rename = "updatedAt")]
//     pub updated_at: i64,
//     #[serde(rename = "lastScan")]
//     pub last_scan: i64,
//     #[serde(rename = "scanVersion")]
//     pub scan_version: String,
//     #[serde(rename = "isMissing")]
//     pub is_missing: bool,
//     #[serde(rename = "isInvalid")]
//     pub is_invalid: bool,
//     #[serde(rename = "mediaType")]
//     pub media_type: String,
//     pub media: Media,
//     #[serde(rename = "libraryFiles")]
//     pub library_files: Vec<LibraryFile>,
//     pub size: i64,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct Media {
//     #[serde(rename = "libraryItemId")]
//     pub library_item_id: String,
//     pub metadata: Metadata,
//     #[serde(rename = "coverPath")]
//     pub cover_path: Option<String>,
//     pub tags: Vec<String>,
//     #[serde(rename = "audioFiles")]
//     pub audio_files: Vec<AudioFile>,
//     pub chapters: Vec<Chapter>,
//     pub duration: f64,
//     pub size: i64,
//     pub tracks: Vec<Track>,
//     // #[serde(rename = "ebookFile")]
//     // pub ebook_file: Value,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct Metadata {
//     pub title: String,
//     #[serde(rename = "titleIgnorePrefix")]
//     pub title_ignore_prefix: String,
//     pub subtitle: Option<String>,
//     pub authors: Vec<Author>,
//     pub narrators: Vec<String>,
//     pub series: Vec<Series>,
//     pub genres: Vec<String>,
//     #[serde(rename = "publishedYear")]
//     pub published_year: Option<String>,
//     #[serde(rename = "publishedDate")]
//     pub published_date: Option<String>,
//     pub publisher: Option<String>,
//     pub description: String,
//     pub isbn: Option<String>,
//     pub asin: Option<String>,
//     pub language: Option<String>,
//     pub explicit: bool,
//     #[serde(rename = "authorName")]
//     pub author_name: Option<String>,
//     #[serde(rename = "authorNameLF")]
//     pub author_name_lf: Option<String>,
//     #[serde(rename = "narratorName")]
//     pub narrator_name: Option<String>,
//     #[serde(rename = "seriesName")]
//     pub series_name: Option<String>,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct Author {
//     pub id: String,
//     pub name: String,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct Series {
//     pub id: String,
//     pub name: String,
//     pub sequence: Option<String>,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct AudioFile {
//     pub index: i64,
//     pub ino: String,
//     pub metadata: Metadata2,
//     #[serde(rename = "addedAt")]
//     pub added_at: i64,
//     #[serde(rename = "updatedAt")]
//     pub updated_at: i64,
//     #[serde(rename = "trackNumFromMeta")]
//     pub track_num_from_meta: Option<i64>,
//     // #[serde(rename = "discNumFromMeta")]
//     // pub disc_num_from_meta: Option<i64>,
//     #[serde(rename = "trackNumFromFilename")]
//     pub track_num_from_filename: Option<i64>,
//     // #[serde(rename = "discNumFromFilename")]
//     // pub disc_num_from_filename: Option<i64>,
//     #[serde(rename = "manuallyVerified")]
//     pub manually_verified: bool,
//     pub exclude: bool,
//     // pub error: Value,
//     pub format: String,
//     pub duration: f64,
//     #[serde(rename = "bitRate")]
//     pub bit_rate: i64,
//     // pub language: Value,
//     pub codec: String,
//     #[serde(rename = "timeBase")]
//     pub time_base: String,
//     pub channels: i64,
//     #[serde(rename = "channelLayout")]
//     pub channel_layout: String,
//     // pub chapters: Vec<Value>,
//     // #[serde(rename = "embeddedCoverArt")]
//     // pub embedded_cover_art: Value,
//     #[serde(rename = "metaTags")]
//     pub meta_tags: MetaTags,
//     #[serde(rename = "mimeType")]
//     pub mime_type: String,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct Metadata2 {
//     pub filename: String,
//     pub ext: String,
//     pub path: String,
//     #[serde(rename = "relPath")]
//     pub rel_path: String,
//     pub size: i64,
//     #[serde(rename = "mtimeMs")]
//     pub mtime_ms: i64,
//     #[serde(rename = "ctimeMs")]
//     pub ctime_ms: i64,
//     #[serde(rename = "birthtimeMs")]
//     pub birthtime_ms: i64,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct MetaTags {
//     #[serde(rename = "tagAlbum")]
//     pub tag_album: Option<String>,
//     #[serde(rename = "tagArtist")]
//     pub tag_artist: Option<String>,
//     #[serde(rename = "tagGenre")]
//     pub tag_genre: Option<String>,
//     #[serde(rename = "tagTitle")]
//     pub tag_title: Option<String>,
//     #[serde(rename = "tagTrack")]
//     pub tag_track: Option<String>,
//     #[serde(rename = "tagAlbumArtist")]
//     pub tag_album_artist: Option<String>,
//     #[serde(rename = "tagComposer")]
//     pub tag_composer: Option<String>,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct Chapter {
//     pub id: i64,
//     pub start: f64,
//     pub end: f64,
//     pub title: String,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct Track {
//     pub index: i64,
//     #[serde(rename = "startOffset")]
//     pub start_offset: f64,
//     pub duration: f64,
//     pub title: String,
//     #[serde(rename = "contentUrl")]
//     pub content_url: String,
//     #[serde(rename = "mimeType")]
//     pub mime_type: String,
//     pub metadata: Metadata3,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct Metadata3 {
//     pub filename: String,
//     pub ext: String,
//     pub path: String,
//     #[serde(rename = "relPath")]
//     pub rel_path: String,
//     pub size: i64,
//     #[serde(rename = "mtimeMs")]
//     pub mtime_ms: i64,
//     #[serde(rename = "ctimeMs")]
//     pub ctime_ms: i64,
//     #[serde(rename = "birthtimeMs")]
//     pub birthtime_ms: i64,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct LibraryFile {
//     pub ino: String,
//     pub metadata: Metadata4,
//     #[serde(rename = "addedAt")]
//     pub added_at: i64,
//     #[serde(rename = "updatedAt")]
//     pub updated_at: i64,
//     #[serde(rename = "fileType")]
//     pub file_type: String,
// }
//
// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct Metadata4 {
//     pub filename: String,
//     pub ext: String,
//     pub path: String,
//     #[serde(rename = "relPath")]
//     pub rel_path: String,
//     pub size: i64,
//     #[serde(rename = "mtimeMs")]
//     pub mtime_ms: i64,
//     #[serde(rename = "ctimeMs")]
//     pub ctime_ms: i64,
//     #[serde(rename = "birthtimeMs")]
//     pub birthtime_ms: i64,
// }

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct AuthorResponse {
    pub id: String,
    // pub asin: Option<String>,
    pub name: String,
    // pub description: Option<String>,
    // #[serde(rename = "imagePath")]
    // pub image_path: Option<String>,
    // #[serde(rename = "addedAt")]
    // pub added_at: i64,
    // #[serde(rename = "updatedAt")]
    // pub updated_at: i64,
    #[serde(rename = "libraryItems")]
    pub library_items: Vec<LibraryItemMinified>,
    // pub series: Vec<SeriesItem>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct LibraryItemMinified {
    pub id: String,
    pub ino: String,
    #[serde(rename = "libraryId")]
    pub library_id: String,
    #[serde(rename = "folderId")]
    pub folder_id: String,
    pub path: String,
    #[serde(rename = "relPath")]
    pub rel_path: String,
    #[serde(rename = "isFile")]
    pub is_file: bool,
    // #[serde(rename = "mtimeMs")]
    // pub mtime_ms: i64,
    // #[serde(rename = "ctimeMs")]
    // pub ctime_ms: i64,
    // #[serde(rename = "birthtimeMs")]
    // pub birthtime_ms: i64,
    // #[serde(rename = "addedAt")]
    // pub added_at: i64,
    // #[serde(rename = "updatedAt")]
    // pub updated_at: i64,
    // #[serde(rename = "isMissing")]
    // pub is_missing: bool,
    // #[serde(rename = "isInvalid")]
    // pub is_invalid: bool,
    // #[serde(rename = "mediaType")]
    // pub media_type: String,
    // pub media: MediaMinified,
    // #[serde(rename = "numFiles")]
    // pub num_files: i64,
    // pub size: i64,
}

// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct MediaMinified {
//     pub metadata: MetadataSmall,
//     #[serde(rename = "coverPath")]
//     pub cover_path: Option<String>,
//     pub tags: Vec<String>,
//     #[serde(rename = "numTracks")]
//     pub num_tracks: i64,
//     #[serde(rename = "numAudioFiles")]
//     pub num_audio_files: i64,
//     #[serde(rename = "numChapters")]
//     pub num_chapters: i64,
//     pub duration: f64,
//     pub size: i64,
//     #[serde(rename = "ebookFileFormat")]
//     pub ebook_file_format: Option<String>,
// }

// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct MetadataSmall {
//     pub title: String,
//     #[serde(rename = "titleIgnorePrefix")]
//     pub title_ignore_prefix: String,
//     pub subtitle: Option<String>,
//     #[serde(rename = "authorName")]
//     pub author_name: Option<String>,
//     #[serde(rename = "narratorName")]
//     pub narrator_name: Option<String>,
//     #[serde(rename = "seriesName")]
//     pub series_name: Option<String>,
//     pub genres: Vec<String>,
//     #[serde(rename = "publishedYear")]
//     pub published_year: Option<String>,
//     #[serde(rename = "publishedDate")]
//     pub published_date: Option<String>,
//     pub publisher: Option<String>,
//     pub description: Option<String>,
//     pub isbn: Option<String>,
//     pub asin: Option<String>,
//     pub language: Option<String>,
//     pub explicit: bool,
// }

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct MediaUpdate<'a> {
    metadata: MetadataUpdate<'a>,
    // #[serde(rename = "coverPath")]
    // pub cover_path: Option<String>,
    // pub tags: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct MetadataUpdate<'a> {
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub authors: Vec<AuthorUpdate<'a>>,
    pub narrators: Vec<&'a str>,
    pub series: Vec<SeriesUpdate<'a>>,

    // pub genres: Vec<String>,
    // #[serde(rename = "publishedYear")]
    // pub published_year: Option<&'a str>,
    // #[serde(rename = "publishedDate")]
    // pub published_date: Option<&'a str>,
    // pub publisher: Option<&'a str>,
    pub description: Option<&'a str>,
    pub isbn: Option<&'a str>,
    pub asin: Option<&'a str>,
    pub genres: BTreeSet<&'a str>,
    pub language: Option<&'a str>,
    pub explicit: bool,
    pub abridged: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AuthorUpdate<'a> {
    pub name: &'a str,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct SeriesUpdate<'a> {
    pub name: &'a str,
    pub sequence: Option<String>,
}

// #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
// pub struct SeriesItem {
//     pub id: String,
//     pub name: String,
//     pub items: Vec<LibraryItemMinified>,
// }

#[instrument(skip_all)]
pub async fn match_torrents_to_abs(
    config: &AudiobookShelfConfig,
    db: Arc<Database<'_>>,
) -> Result<()> {
    let abs = Abs::new(config)?;
    let torrents = db.r_transaction()?.scan().primary::<Torrent>()?;
    let torrents = torrents.all()?.filter(|t| {
        t.as_ref()
            .is_ok_and(|t| !t.meta.ids.contains_key(ids::ABS) && t.library_path.is_some())
    });

    for torrent in torrents {
        let mut torrent = torrent?;
        let Some(book) = abs.get_book(&torrent).await? else {
            trace!(
                "Could not find ABS entry for torrent {}",
                torrent.meta.title
            );
            continue;
        };
        debug!("Matched ABS entry with torrent {}", torrent.meta.title);
        torrent.meta.ids.insert(ids::ABS.to_string(), book.id);
        let (_guard, rw) = db.rw_async().await?;
        rw.upsert(torrent)?;
        rw.commit()?;
    }

    Ok(())
}

pub struct Abs {
    base_url: String,
    client: reqwest::Client,
}

impl Abs {
    pub fn new(config: &AudiobookShelfConfig) -> Result<Abs> {
        // let mut headers = Default::default();
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {}", config.token).parse()?);
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("MLM")
            .build()?;

        Ok(Abs {
            base_url: config.url.to_owned(),
            client,
        })
    }

    pub async fn get_book(&self, torrent: &Torrent) -> Result<Option<LibraryItemMinified>> {
        let Some(library_path) = &torrent.library_path else {
            return Ok(None);
        };
        let Some(first_author) = torrent.meta.authors.first() else {
            return Ok(None);
        };
        let resp: LibrariesResponse = self
            .client
            .get(format!("{}/api/libraries", self.base_url))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let libraries = resp.libraries.into_iter().filter(|l| {
            l.folders
                .iter()
                .any(|f| library_path.starts_with(&f.full_path))
        });

        for library in libraries {
            let mut url: Url = format!("{}/api/libraries/{}/search", self.base_url, library.id)
                .parse()
                .unwrap();
            url.query_pairs_mut().append_pair("q", first_author);
            let resp = self
                .client
                .get(url)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;

            let resp: SearchResponse = serde_json::from_str(&resp).map_err(|err| {
                error!("Error parsing ABS response: {err}\nResponse: {resp}");
                err
            })?;

            for author in resp.authors {
                let url: Url = format!("{}/api/authors/{}?include=items", self.base_url, author.id)
                    .parse()
                    .unwrap();
                let resp = self
                    .client
                    .get(url)
                    .send()
                    .await?
                    .error_for_status()?
                    .text()
                    .await?;

                let resp: AuthorResponse = serde_json::from_str(&resp).map_err(|err| {
                    error!("Error parsing ABS response: {err}\nResponse: {resp}");
                    err
                })?;

                let Some(book) = resp
                    .library_items
                    .into_iter()
                    .find(|b| &PathBuf::from(&b.path) == library_path)
                else {
                    continue;
                };

                return Ok(Some(book));
            }
        }

        Ok(None)
    }

    pub async fn update_book(&self, id: &str, meta: &TorrentMeta) -> Result<()> {
        let (title, subtitle) = parse_titles(meta);
        self.client
            .patch(format!("{}/api/items/{id}/media", self.base_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&MediaUpdate {
                metadata: MetadataUpdate {
                    title,
                    subtitle,
                    authors: meta
                        .authors
                        .iter()
                        .map(|name| AuthorUpdate { name })
                        .collect(),
                    series: meta
                        .series
                        .iter()
                        .map(|series| SeriesUpdate {
                            name: &series.name,
                            sequence: if series.entries.0.is_empty() {
                                None
                            } else {
                                Some(format!("{}", series.entries))
                            },
                        })
                        .collect(),
                    narrators: meta.narrators.iter().map(|name| name.as_str()).collect(),
                    description: Some(&clean_html(&meta.description)),
                    isbn: meta.ids.get(ids::ISBN).map(|s| s.as_str()),
                    asin: meta.ids.get(ids::ASIN).map(|s| s.as_str()),
                    genres: meta
                        .cat
                        .as_ref()
                        .map(|c| c.as_str())
                        .into_iter()
                        .chain(meta.categories.iter().map(|c| c.as_str()))
                        .collect::<BTreeSet<_>>(),
                    language: meta.language.map(|l| l.to_str()),
                    explicit: meta
                        .flags
                        .is_some_and(|f| Flags::from_bitfield(f.0).explicit == Some(true)),
                    abridged: meta
                        .flags
                        .is_some_and(|f| Flags::from_bitfield(f.0).abridged == Some(true)),
                },
            })?)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn delete_book(&self, id: &str) -> Result<()> {
        self.client
            .delete(format!("{}/api/items/{id}", self.base_url))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

pub fn create_metadata(meta: &TorrentMeta) -> serde_json::Value {
    let (title, subtitle) = parse_titles(meta);
    let flags = Flags::from_bitfield(meta.flags.map_or(0, |f| f.0));

    let metadata = json!({
        "authors": &meta.authors,
        "narrators": &meta.narrators,
        "series": &meta.series.iter().map(format_serie).collect::<Vec<_>>(),
        "title": title,
        "subtitle": subtitle,
        "description": meta.description,
        "isbn": meta.ids.get(ids::ISBN),
        "asin": meta.ids.get(ids::ASIN),
        "tags": if flags.lgbt == Some(true) { Some(vec!["LGBT"]) } else { None },
        "genres": meta
            .cat
            .as_ref()
            .map(|c| c.as_str())
            .into_iter()
            .chain(meta.categories.iter().map(|c| c.as_str()))
            .collect::<BTreeSet<_>>(),
        "language": meta.language.map(|l| l.to_str()),
        "explicit": flags.explicit == Some(true),
        "abridged": flags.abridged == Some(true),
    });

    metadata
}

fn parse_titles(meta: &TorrentMeta) -> (&str, Option<&str>) {
    let mut titles = meta.title.splitn(2, ":");
    let mut title = titles.next().unwrap();
    let mut subtitle = titles.next().map(|t| t.trim());
    if title.len() < 4 {
        title = &meta.title;
        subtitle = None;
    }

    (title, subtitle)
}
