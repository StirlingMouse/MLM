use std::{
    fs::{File, create_dir_all, hard_link},
    io::{BufWriter, Write},
    path::{Component, PathBuf},
};

use anyhow::{Context, Result};
use htmlentity::entity::{self, ICodedDataTrait as _};
use native_db::Database;
use qbit::{models::TorrentContent, parameters::TorrentListParams};
use regex::Regex;
use serde_json::{Value, json};

use crate::{
    config::{Config, QbitConfig},
    data,
    mam::MaM,
    qbittorrent::QbitError,
};

pub async fn link_torrents_to_library(
    config: &Config,
    db: &Database<'_>,
    qbit: (&QbitConfig, qbit::Api),
    mam: &MaM<'_>,
) -> Result<()> {
    let disk_pattern = Regex::new(r"(?:CD|Disc|Disk)\s*(\d+)").unwrap();

    let torrents = qbit
        .1
        .torrents(TorrentListParams::deafult())
        .await
        .map_err(QbitError)
        .context("qbit main data")?;

    for torrent in torrents {
        let Some(hash) = torrent.hash else {
            continue;
        };
        {
            let r = db.r_transaction()?;
            let torrent: Option<data::Torrent> = r.get().primary(hash.clone())?;
            if torrent.is_some() {
                continue;
            }
        }
        if torrent.progress < 1.0 {
            continue;
        }
        if let Some(ref wanted_tags) = qbit.0.tags {
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
        let files = qbit.1.files(&hash, None).await.map_err(QbitError)?;
        let selected_audio_format = select_format(&config.audio_types, &files);
        let selected_ebook_format = select_format(&config.ebook_types, &files);
        println!("{selected_audio_format:?} {selected_ebook_format:?}");

        if selected_audio_format.is_none() && selected_ebook_format.is_none() {
            eprintln!(
                "Could not find and wanted formats in torrent \"{}\"",
                torrent.name,
            );
            continue;
        }

        let Some(mam_torrent) = mam.get_torrent_info(&hash).await.context("get_mam_info")? else {
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
        let author = clean_value(author)?;

        let series = mam_torrent.series_info.first_key_value();
        let dir = match series {
            Some((_, (series_name, series_num))) => {
                let series_name = clean_value(series_name)?;
                PathBuf::from(author)
                    .join(&series_name)
                    .join(if series_num.is_empty() {
                        mam_torrent.title.clone()
                    } else {
                        format!("{series_name} #{series_num} - {}", mam_torrent.title)
                    })
            }
            None => PathBuf::from(author).join(&mam_torrent.title),
        };
        let dir = library.library_dir.join(dir);
        println!("out_dir: {:?}", dir);
        let mut titles = mam_torrent.title.splitn(2, ":");
        let title = titles.next().unwrap();
        let subtitle = titles.next().map(|t| t.trim());
        let isbn_raw = match mam_torrent.isbn {
            Some(Value::String(isbn)) => isbn,
            Some(Value::Number(isbn)) => isbn.to_string(),
            _ => "".to_string(),
        };
        let isbn = if isbn_raw.is_empty() || isbn_raw.starts_with("ASIN:") {
            None
        } else {
            Some(&isbn_raw[..])
        };
        let asin = isbn_raw.strip_prefix("ASIN:");
        let metadata = json!({
            "authors": mam_torrent.author_info.values().map(|a| clean_value(a)).collect::<Result<Vec<_>>>()?,
            "narrators": mam_torrent.narrator_info.values().map(|n| clean_value(n)).collect::<Result<Vec<_>>>()?,
            "series": mam_torrent.series_info.values().map(|(series_name, series_num)| {
                let series_name = clean_value(series_name)?;
                Ok(if series_num.is_empty() { series_name.clone() } else { format!("{series_name} #{series_num}") })
            }).collect::<Result<Vec<_>>>()?,
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
                .is_some_and(|ext| file.name.ends_with(ext))
                || selected_ebook_format
                    .as_ref()
                    .is_some_and(|ext| file.name.ends_with(ext)))
            {
                eprintln!("Skiping \"{}\"", file.name);
                continue;
            }
            let torrent_path = PathBuf::from(&file.name);
            let mut path_components = torrent_path.components();
            let file_name = path_components.next_back().unwrap();
            let dir_name = path_components.next_back().and_then(|dir_name| {
                if let Component::Normal(dir_name) = dir_name {
                    let dir_name = dir_name.to_string_lossy().to_string();
                    if let Some(disc) = disk_pattern.captures(&dir_name).and_then(|c| c.get(1)) {
                        return Some(format!("Disc {}", disc.as_str()));
                    }
                }
                None
            });
            let library_path = if let Some(dir_name) = dir_name {
                let sub_dir = dir.join(&dir_name);
                create_dir_all(&sub_dir)?;
                sub_dir.join(file_name)
            } else {
                dir.join(file_name)
            };
            let download_path = PathBuf::from(&torrent.save_path).join(&file.name);
            println!("linking: {:?} -> {:?}", download_path, library_path);
            hard_link(download_path, library_path)?;
        }

        let file = File::create(dir.join("metadata.json"))?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &metadata)?;
        writer.flush()?;

        {
            let rw = db.rw_transaction()?;
            rw.insert(data::Torrent {
                hash,
                library_path: dir,
            })?;
            rw.commit()?;
        }
    }

    Ok(())
}

fn clean_value(value: &str) -> Result<String> {
    entity::decode(value.as_bytes()).to_string()
}

fn select_format(wanted_formats: &[String], files: &[TorrentContent]) -> Option<String> {
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
