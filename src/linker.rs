use std::{
    fs::{File, create_dir_all, hard_link},
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use qbit::models::TorrentContent;
use serde_json::{Value, json};

use crate::{config::Config, mam::MaM, qbittorrent::QbitError};

pub async fn link_torrents_to_library(config: &Config, qbit: qbit::Api, mam: MaM) -> Result<()> {
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
        println!("{selected_audio_format:?} {selected_ebook_format:?}");

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
                .is_some_and(|ext| file.name.ends_with(ext))
                || selected_ebook_format
                    .as_ref()
                    .is_some_and(|ext| file.name.ends_with(ext)))
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
        break;
    }

    Ok(())
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
