use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    fs::{File, create_dir_all, hard_link},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::Arc,
};

use anyhow::{Context, Error, Result};
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use qbit::models::TorrentContent;
use reqwest::{Url, cookie::Jar};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use serde_nested_json;

#[derive(Debug, Deserialize)]
struct Config {
    mam_id: String,
    qbittorrent: QbitConfig,
    #[serde(default = "default_audio_types")]
    audio_types: Vec<String>,
    #[serde(default = "default_ebook_types")]
    ebook_types: Vec<String>,
    #[serde(alias = "library")]
    libraries: Vec<Library>,
}

#[derive(Debug, Deserialize)]
struct QbitConfig {
    url: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Library {
    download_dir: PathBuf,
    library_dir: PathBuf,
}

fn default_audio_types() -> Vec<String> {
    ["m4b", "mp3", "ogg"]
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn default_ebook_types() -> Vec<String> {
    ["cbz", "epub", "pdf", "mobi", "azw3", "cbr"]
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[derive(Debug)]
struct QbitError(qbit::Error);

impl std::error::Error for QbitError {}
impl Display for QbitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}
impl From<qbit::Error> for QbitError {
    fn from(value: qbit::Error) -> Self {
        QbitError(value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    perpage: i64,
    start: i64,
    data: Vec<MaMTorrent>,
    total: i64,
    found: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaMTorrent {
    id: i64,
    added: String,
    #[serde(deserialize_with = "json_or_default")]
    author_info: BTreeMap<u64, String>,
    bookmarked: Option<u64>,
    browseflags: u64,
    category: u64,
    catname: String,
    cat: String,
    comments: u64,
    description: String,
    filetype: String,
    fl_vip: i64,
    free: i64,
    isbn: Value,
    lang_code: String,
    language: u64,
    leechers: u64,
    main_cat: u64,
    my_snatched: u64,
    #[serde(deserialize_with = "json_or_default")]
    narrator_info: BTreeMap<u64, String>,
    numfiles: u64,
    owner: u64,
    owner_name: String,
    personal_freeleech: u64,
    seeders: u64,
    #[serde(deserialize_with = "json_or_default")]
    series_info: BTreeMap<u64, (String, String)>,
    size: String,
    tags: String,
    times_completed: u64,
    title: String,
    vip: u64,
    w: u64,
}

fn json_or_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: Deserializer<'de>,
{
    let v: Result<Value, _> = serde_nested_json::deserialize(deserializer);
    let Ok(v) = v else {
        return Ok(T::default());
    };
    Ok(T::deserialize(v).unwrap_or_default())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config = Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::prefixed("MLM_"))
        .extract()?;

    let mam = MaM::new(&config)?;
    let qbit = qbit::Api::login(
        &config.qbittorrent.url,
        &config.qbittorrent.username,
        &config.qbittorrent.password,
    )
    .await
    .map_err(QbitError)?;

    let main_data = qbit
        .main_data(None)
        .await
        .map_err(QbitError)
        .context("qbit main data")?;

    let torrents = main_data.torrents.iter().flatten();

    for (hash, torrent) in torrents {
        if torrent.progress < 1.0 {
            continue;
        }
        if let Some(ref wanted_tags) = config.qbittorrent.tags {
            let mut torrent_tags = torrent.tags.split(", ");
            let wanted = torrent_tags.any(|ttag| wanted_tags.iter().any(|wtag| ttag == wtag));
            if !wanted {
                continue;
            };
        }
        let Some(library) = config
            .libraries
            .iter()
            .find(|l| PathBuf::from(&torrent.save_path).starts_with(&l.download_dir))
        else {
            eprintln!(
                "Could not find matching library for torrent \"{}\", save_path {}",
                torrent.name, torrent.save_path
            );
            continue;
        };
        let files = qbit.files(hash, None).await.map_err(QbitError)?;
        let selected_audio_format = select_format(&config.audio_types, &files);
        let selected_ebook_format = select_format(&config.ebook_types, &files);

        if selected_audio_format.is_none() && selected_ebook_format.is_none() {
            eprintln!(
                "Could not find and wanted formats in torrent \"{}\"",
                torrent.name,
            );
            continue;
        }

        let Some(mam_torrent) = mam.get_torrent_info(hash).await.context("get_mam_info")? else {
            eprintln!(
                "Could not find torrent \"{}\", hash {} on mam",
                torrent.name, hash
            );
            continue;
        };
        let Some((_, author)) = mam_torrent.author_info.first_key_value() else {
            eprintln!("Torrent \"{}\" has no author", torrent.name);
            continue;
        };

        let series = mam_torrent.series_info.first_key_value();
        let dir =
            match series {
                Some((_, (series_name, series_num))) => PathBuf::from(author)
                    .join(series_name)
                    .join(if series_num.is_empty() {
                        mam_torrent.title.clone()
                    } else {
                        format!("{series_name} #{series_num} - {}", mam_torrent.title)
                    }),
                None => PathBuf::from(author).join(&mam_torrent.title),
            };
        let dir = library.library_dir.join(dir);
        println!("out_dir: {:?}", dir);
        let mut titles = mam_torrent.title.splitn(2, ":");
        let title = titles.next().unwrap();
        let subtitle = titles.next().map(|t| t.trim());
        let isbn_raw = match mam_torrent.isbn {
            Value::String(isbn) => isbn,
            Value::Number(isbn) => isbn.to_string(),
            _ => "".to_string(),
        };
        let isbn = if isbn_raw.is_empty() || isbn_raw.starts_with("ASIN:") {
            None
        } else {
            Some(&isbn_raw[..])
        };
        let asin = isbn_raw.strip_prefix("ASIN:");
        let metadata = json!({
            "authors": mam_torrent.author_info.values().collect::<Vec<_>>(),
            "narrators": mam_torrent.narrator_info.values().collect::<Vec<_>>(),
            "series": mam_torrent.series_info.values().map(|(series_name, series_num)|
                if series_num.is_empty() { series_name.clone() } else { format!("{series_name} #{series_num}") }
            ).collect::<Vec<_>>(),
            "title": title,
            "subtitle": subtitle,
            "description": mam_torrent.description,
            "isbn": isbn,
            "asin": asin,
        });
        println!("metadata: {metadata:?}");
        create_dir_all(&dir)?;

        for file in files {
            println!("file: {:?}", file.name);
            if !(selected_audio_format
                .as_ref()
                .is_some_and(|ext| !file.name.ends_with(ext))
                || selected_ebook_format
                    .as_ref()
                    .is_some_and(|ext| !file.name.ends_with(ext)))
            {
                eprintln!("Skiping \"{}\"", file.name);
                continue;
            }
            let library_path = dir.join(PathBuf::from(&file.name).file_name().unwrap());
            let download_path = PathBuf::from(&torrent.save_path).join(&file.name);
            println!("linking: {:?} -> {:?}", download_path, library_path);
            hard_link(download_path, library_path)?;
        }

        let file = File::create(dir.join("metadata.json"))?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &metadata)?;
        writer.flush()?;
    }

    Ok(())
}

fn select_format(wanted_formats: &Vec<String>, files: &Vec<TorrentContent>) -> Option<String> {
    wanted_formats
        .iter()
        .map(|ext| {
            if ext.starts_with(".") {
                ext.clone()
            } else {
                format!(".{ext}")
            }
        })
        .find(|ext| files.iter().any(|f| f.name.ends_with(ext)))
}

struct MaM {
    jar: Arc<Jar>,
    client: reqwest::Client,
}

impl MaM {
    pub fn new(config: &Config) -> Result<MaM> {
        let cookie = format!("mam_id={}; Domain=.myanonamouse.net", config.mam_id);
        let url = "https://www.myanonamouse.net".parse::<Url>().unwrap();

        let jar = Jar::default();
        jar.add_cookie_str(&cookie, &url);
        let jar = Arc::new(jar);
        let client = reqwest::Client::builder()
            .cookie_provider(jar.clone())
            .user_agent("MLM")
            .build()?;

        Ok(MaM { jar, client })
    }

    async fn get_torrent_info(&self, hash: &str) -> Result<Option<MaMTorrent>, Error> {
        let mut resp = self
            .client
            .post("https://www.myanonamouse.net/tor/js/loadSearchJSONbasic.php")
            .json(&json!({
                "description": true,
                "isbn": true,
                "tor": { "hash": hash }
            }))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        println!("resp: {resp:?}");
        let mut resp: SearchResult = serde_json::from_str(&resp).context("parse mam response")?;
        println!("resp2: {resp:?}");
        Ok(resp.data.pop())
    }
}
