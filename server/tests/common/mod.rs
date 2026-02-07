use anyhow::Result;
use mlm::config::{Config, Library, LibraryByRipDir, LibraryLinkMethod, LibraryOptions};
use mlm_db::{
    migrate, Database, MainCat, MediaType, MetadataSource, Size, Timestamp, Torrent, TorrentMeta,
    MODELS,
};
use native_db::Builder;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

pub struct TestDb {
    pub db: Arc<Database<'static>>,
    #[allow(dead_code)]
    temp_dir: TempDir,
}

impl TestDb {
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        let db = Builder::new().create(&MODELS, db_path)?;
        migrate(&db)?;
        Ok(Self {
            db: Arc::new(db),
            temp_dir,
        })
    }
}

#[allow(dead_code)]
pub struct MockTorrentBuilder {
    torrent: Torrent,
}

#[allow(dead_code)]
impl MockTorrentBuilder {
    pub fn new(id: &str, title: &str) -> Self {
        Self {
            torrent: Torrent {
                id: id.to_string(),
                id_is_hash: false,
                mam_id: None,
                library_path: None,
                library_files: vec![],
                linker: None,
                category: None,
                selected_audio_format: None,
                selected_ebook_format: None,
                title_search: mlm_parse::normalize_title(title),
                meta: TorrentMeta {
                    ids: Default::default(),
                    vip_status: None,
                    cat: None,
                    media_type: MediaType::Audiobook,
                    main_cat: Some(MainCat::Fiction),
                    categories: vec![],
                    tags: vec![],
                    language: None,
                    flags: None,
                    filetypes: vec!["m4b".to_string()],
                    num_files: 1,
                    size: Size::from_bytes(0),
                    title: title.to_string(),
                    edition: None,
                    description: "".to_string(),
                    authors: vec![],
                    narrators: vec![],
                    series: vec![],
                    source: MetadataSource::Mam,
                    uploaded_at: Timestamp::now(),
                },
                created_at: Timestamp::now(),
                replaced_with: None,
                library_mismatch: None,
                client_status: None,
            },
        }
    }

    pub fn with_library_path(mut self, path: PathBuf) -> Self {
        self.torrent.library_path = Some(path);
        self
    }

    pub fn with_mam_id(mut self, mam_id: u64) -> Self {
        self.torrent.mam_id = Some(mam_id);
        self.torrent
            .meta
            .ids
            .insert(mlm_db::ids::MAM.to_string(), mam_id.to_string());
        self
    }

    pub fn with_size(mut self, size_bytes: u64) -> Self {
        self.torrent.meta.size = Size::from_bytes(size_bytes);
        self
    }

    pub fn with_author(mut self, author: &str) -> Self {
        self.torrent.meta.authors.push(author.to_string());
        self
    }

    pub fn with_language(mut self, language: mlm_db::Language) -> Self {
        self.torrent.meta.language = Some(language);
        self
    }

    pub fn build(self) -> Torrent {
        self.torrent
    }
}

pub struct MockFs {
    #[allow(dead_code)]
    pub root: TempDir,
    pub rip_dir: PathBuf,
    pub library_dir: PathBuf,
}

impl MockFs {
    pub fn new() -> Result<Self> {
        let root = tempfile::tempdir()?;
        let rip_dir = root.path().join("rip");
        let library_dir = root.path().join("library");
        std::fs::create_dir_all(&rip_dir)?;
        std::fs::create_dir_all(&library_dir)?;
        Ok(Self {
            root,
            rip_dir,
            library_dir,
        })
    }

    #[allow(dead_code)]
    pub fn create_libation_folder(
        &self,
        asin: &str,
        title: &str,
        authors: Vec<&str>,
    ) -> Result<PathBuf> {
        let folder_path = self.rip_dir.join(asin);
        std::fs::create_dir_all(&folder_path)?;

        let libation_meta = serde_json::json!({
            "asin": asin,
            "title": title,
            "subtitle": "",
            "authors": authors.into_iter().map(|a| serde_json::json!({"name": a})).collect::<Vec<_>>(),
            "narrators": [],
            "series": [],
            "language": "English",
            "format_type": "unabridged",
            "publisher_summary": "Test summary",
            "merchandising_summary": "Test merchandising summary",
            "category_ladders": [],
            "is_adult_product": false,
            "issue_date": "2023-01-01",
            "publication_datetime": "2023-01-01T00:00:00Z",
            "publication_name": "Test Publisher",
            "publisher_name": "Test Publisher",
            "release_date": "2023-01-01",
            "runtime_length_min": 60,
        });

        let meta_path = folder_path.join(format!("{}.json", asin));
        std::fs::write(meta_path, serde_json::to_string(&libation_meta)?)?;

        let audio_path = folder_path.join(format!("{}.m4b", asin));
        std::fs::write(audio_path, "fake audio data")?;

        Ok(folder_path)
    }
}

pub fn mock_config(rip_dir: PathBuf, library_dir: PathBuf) -> Config {
    Config {
        mam_id: "test".to_string(),
        web_host: "127.0.0.1".to_string(),
        web_port: 3157,
        min_ratio: 2.0,
        unsat_buffer: 10,
        wedge_buffer: 0,
        add_torrents_stopped: false,
        exclude_narrator_in_library_dir: false,
        search_interval: 30,
        link_interval: 10,
        import_interval: 135,
        ignore_torrents: vec![],
        audio_types: vec!["m4b".to_string()],
        ebook_types: vec!["epub".to_string()],
        music_types: vec!["mp3".to_string()],
        radio_types: vec!["mp3".to_string()],
        search: Default::default(),
        audiobookshelf: None,
        autograbs: vec![],
        snatchlist: vec![],
        goodreads_lists: vec![],
        notion_lists: vec![],
        tags: vec![],
        qbittorrent: vec![],
        libraries: vec![Library::ByRipDir(LibraryByRipDir {
            rip_dir,
            options: LibraryOptions {
                name: Some("test_library".to_string()),
                library_dir,
                method: LibraryLinkMethod::Hardlink,
                audio_types: None,
                ebook_types: None,
            },
            filter: Default::default(),
        })],
    }
}
