#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt as _;
#[cfg(target_family = "windows")]
use std::os::windows::fs::MetadataExt as _;
use std::{
    collections::BTreeMap,
    fs::{self, Metadata},
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tokio::fs::DirEntry;

use anyhow::{Result, bail};
use file_id::get_file_id;
use mlm_db::Size;
use tracing::{debug, trace};

pub fn map_path(path_mapping: &BTreeMap<PathBuf, PathBuf>, save_path: &str) -> PathBuf {
    let mut path = PathBuf::from(save_path);
    for (from, to) in path_mapping.iter().rev() {
        if path.starts_with(from) {
            let mut components = path.components();
            for _ in from {
                components.next();
            }
            path = to.join(components.as_path());
            break;
        }
    }
    path
}

pub fn library_dir(
    exclude_narrator_in_library_dir: bool,
    library: &crate::config::Library,
    meta: &mlm_db::TorrentMeta,
) -> Option<PathBuf> {
    let author = meta.authors.first()?;
    let mut dir = match meta
        .series
        .iter()
        .find(|s| !s.entries.0.is_empty())
        .or(meta.series.first())
    {
        Some(series) => PathBuf::from(sanitize_filename::sanitize(author).to_string())
            .join(sanitize_filename::sanitize(&series.name).to_string())
            .join(
                sanitize_filename::sanitize(if series.entries.0.is_empty() {
                    meta.title.clone()
                } else {
                    format!("{} #{} - {}", series.name, series.entries, meta.title)
                })
                .to_string(),
            ),
        None => PathBuf::from(sanitize_filename::sanitize(author).to_string())
            .join(sanitize_filename::sanitize(&meta.title).to_string()),
    };
    if let Some((edition, _)) = &meta.edition {
        dir.set_file_name(
            sanitize_filename::sanitize(format!(
                "{}, {}",
                dir.file_name().unwrap().to_string_lossy(),
                edition
            ))
            .to_string(),
        );
    }
    if let Some(narrator) = meta.narrators.first()
        && !exclude_narrator_in_library_dir
    {
        dir.set_file_name(
            sanitize_filename::sanitize(format!(
                "{} {{{}}}",
                dir.file_name().unwrap().to_string_lossy(),
                narrator
            ))
            .to_string(),
        );
    }
    let dir = library.options().library_dir.join(dir);
    Some(dir)
}

pub trait HasFileName {
    fn name_lower(&self) -> String;
}

impl HasFileName for DirEntry {
    fn name_lower(&self) -> String {
        self.file_name().to_string_lossy().to_lowercase()
    }
}

impl HasFileName for qbit::models::TorrentContent {
    fn name_lower(&self) -> String {
        self.name.to_lowercase()
    }
}

pub fn select_format<T: HasFileName>(
    overridden_wanted_formats: &Option<Vec<String>>,
    wanted_formats: &[String],
    files: &[T],
) -> Option<String> {
    overridden_wanted_formats
        .as_deref()
        .unwrap_or(wanted_formats)
        .iter()
        .map(|ext| {
            let ext = ext.to_lowercase();
            if ext.starts_with('.') {
                ext.clone()
            } else {
                format!(".{ext}")
            }
        })
        .find(|ext| files.iter().any(|f| f.name_lower().ends_with(ext)))
}

pub fn hard_link(download_path: &Path, library_path: &Path, file_path: &Path) -> Result<()> {
    debug!("linking: {:?} -> {:?}", download_path, library_path);
    fs::hard_link(download_path, library_path).or_else(|err| {
            if err.kind() == ErrorKind::AlreadyExists {
                trace!("AlreadyExists: {}", err);
                let download_id = get_file_id(download_path);
                trace!("got 1: {download_id:?}");
                let library_id = get_file_id(library_path);
                trace!("got 2: {library_id:?}");
                if let (Ok(download_id), Ok(library_id)) = (download_id, library_id) {
                    trace!("got both");
                    if download_id == library_id {
                        trace!("both match");
                        return Ok(());
                    } else {
                        trace!("no match");
                        bail!(
                            "File \"{:?}\" already exists, torrent file size: {}, library file size: {}",
                            file_path,
                            fs::metadata(download_path).map_or("?".to_string(), |s| Size::from_bytes(file_size(&s)).to_string()),
                            fs::metadata(library_path).map_or("?".to_string(), |s| Size::from_bytes(file_size(&s)).to_string())
                        );
                    }
                }
            }
            Err(err.into())
        })?;
    Ok(())
}

pub fn copy(download_path: &Path, library_path: &Path) -> Result<()> {
    debug!("copying: {:?} -> {:?}", download_path, library_path);
    fs::copy(download_path, library_path)?;
    Ok(())
}

pub fn symlink(download_path: &Path, library_path: &Path) -> Result<()> {
    debug!("symlinking: {:?} -> {:?}", download_path, library_path);
    #[cfg(target_family = "unix")]
    std::os::unix::fs::symlink(download_path, library_path)?;
    #[cfg(target_family = "windows")]
    bail!("symlink is not supported on Windows");
    #[allow(unreachable_code)]
    Ok(())
}

pub fn file_size(m: &Metadata) -> u64 {
    #[cfg(target_family = "unix")]
    return m.size();
    #[cfg(target_family = "windows")]
    return m.file_size();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_map_path() {
        let mut mappings = BTreeMap::new();
        mappings.insert(PathBuf::from("/downloads"), PathBuf::from("/books"));
        mappings.insert(
            PathBuf::from("/downloads/audiobooks"),
            PathBuf::from("/audiobooks"),
        );
        mappings.insert(PathBuf::from("/audiobooks"), PathBuf::from("/audiobooks"));

        assert_eq!(
            map_path(&mappings, "/downloads/torrent"),
            PathBuf::from("/books/torrent")
        );
        assert_eq!(
            map_path(&mappings, "/downloads/audiobooks/torrent"),
            PathBuf::from("/audiobooks/torrent")
        );
        assert_eq!(
            map_path(&mappings, "/downloads/audiobooks/torrent/deep"),
            PathBuf::from("/audiobooks/torrent/deep")
        );
        assert_eq!(
            map_path(&mappings, "/audiobooks/torrent"),
            PathBuf::from("/audiobooks/torrent")
        );
        assert_eq!(
            map_path(&mappings, "/ebooks/torrent"),
            PathBuf::from("/ebooks/torrent")
        );
    }

    #[test]
    fn test_select_format() {
        struct F { name: String }
        impl HasFileName for F {
            fn name_lower(&self) -> String { self.name.to_lowercase() }
        }
        let files = vec![F { name: "book.M4B".to_string() }, F { name: "cover.jpg".to_string() }];
        let wanted = vec!["m4b".to_string(), "mp3".to_string()];
        let sel = select_format(&Some(vec!["m4b".to_string()]), &wanted, &files);
        assert_eq!(sel.unwrap(), ".m4b".to_string());
        let sel2 = select_format(&None, &wanted, &files);
        assert_eq!(sel2.unwrap(), ".m4b".to_string());
    }

    #[test]
    fn test_file_size_and_copy_and_hardlink() {
        use std::fs;
        use std::io::Write;
        let tmp = std::env::temp_dir().join(format!("mlm_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp);
        let src = tmp.join("src_file.bin");
        let dst = tmp.join("dst_file.bin");
        let mut f = fs::File::create(&src).unwrap();
        let data = b"hello world";
        f.write_all(data).unwrap();
        f.sync_all().unwrap();
        let meta = fs::metadata(&src).unwrap();
        assert_eq!(file_size(&meta), data.len() as u64);

        // copy
        copy(&src, &dst).unwrap();
        assert!(dst.exists());
        assert_eq!(fs::metadata(&dst).unwrap().len(), data.len() as u64);

        // hard link target
        let hl = tmp.join("hl_file.bin");
        // remove if exists
        let _ = fs::remove_file(&hl);
        hard_link(&src, &hl, &PathBuf::from("hl_file.bin")).unwrap();
        // both should exist and have same len
        assert!(hl.exists());
        assert_eq!(fs::metadata(&hl).unwrap().len(), data.len() as u64);

        // cleanup
        let _ = fs::remove_file(&src);
        let _ = fs::remove_file(&dst);
        let _ = fs::remove_file(&hl);
    }

    #[cfg(target_family = "unix")]
    #[test]
    fn test_symlink() {
        use std::fs;
        let tmp = std::env::temp_dir().join(format!("mlm_test_symlink_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp);
        let src = tmp.join("s_src.txt");
        let dst = tmp.join("s_dst.txt");
        fs::write(&src, b"x").unwrap();
        let _ = fs::remove_file(&dst);
        symlink(&src, &dst).unwrap();
        assert!(dst.exists());
        let _ = fs::remove_file(&src);
        let _ = fs::remove_file(&dst);
    }
}
