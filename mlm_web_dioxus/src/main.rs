#[cfg(feature = "web")]
use mlm_web_dioxus::app::root;

fn main() {
    #[cfg(feature = "web")]
    dioxus::launch(root);

    #[cfg(feature = "server")]
    server_main();
}

#[cfg(feature = "server")]
#[tokio::main]
async fn server_main() {
    use anyhow::Result;
    use axum::Router;
    use mlm_core::Context;
    use mlm_core::{SsrBackend, Stats, metadata::MetadataService};
    use mlm_mam::api::MaM;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use tower_http::services::ServeDir;

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let database_file = std::env::var("MLM_DX_DB_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("dev.db"));
    if let Some(parent) = database_file.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create dev database directory");
    }
    let db = native_db::Builder::new()
        .create(&mlm_db::MODELS, database_file)
        .expect("Failed to create database");
    mlm_db::migrate(&db).expect("Failed to migrate database");
    let db = Arc::new(db);

    let config_file = std::env::var_os("MLM_CONFIG_FILE")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            [
                std::path::PathBuf::from("config.toml"),
                std::path::PathBuf::from("../config.toml"),
                std::path::PathBuf::from("config/config.toml"),
                std::path::PathBuf::from("../config/config.toml"),
            ]
            .into_iter()
            .find(|path| path.exists())
        });
    let config: mlm_core::config::Config = if let Some(config_file) = config_file {
        use figment::{
            Figment,
            providers::{Format, Toml},
        };
        Figment::new()
            .merge(Toml::file_exact(&config_file))
            .extract()
            .expect("Failed to load config")
    } else {
        tracing::warn!(
            "No config.toml found (checked MLM_CONFIG_FILE, config.toml, ../config.toml, config/config.toml, ../config/config.toml); using defaults"
        );
        mlm_core::config::Config::default()
    };
    let config = Arc::new(config);

    let mam: Arc<Result<Arc<MaM<'static>>>> = if config.mam_id.is_empty() {
        Arc::new(Err(anyhow::Error::msg("No mam_id set (dev mode)")))
    } else {
        Arc::new(MaM::new(&config.mam_id, db.clone()).await.map(Arc::new))
    };

    let default_timeout = Duration::from_secs(5);
    let provider_settings: Vec<mlm_core::metadata::ProviderSetting> = config
        .metadata_providers
        .iter()
        .map(|p| match p {
            mlm_core::config::ProviderConfig::Hardcover(c) => {
                mlm_core::metadata::ProviderSetting::Hardcover {
                    enabled: c.enabled,
                    timeout_secs: c.timeout_secs,
                    api_key: c.api_key.clone(),
                }
            }
            mlm_core::config::ProviderConfig::RomanceIo(c) => {
                mlm_core::metadata::ProviderSetting::RomanceIo {
                    enabled: c.enabled,
                    timeout_secs: c.timeout_secs,
                }
            }
            mlm_core::config::ProviderConfig::OpenLibrary(c) => {
                mlm_core::metadata::ProviderSetting::OpenLibrary {
                    enabled: c.enabled,
                    timeout_secs: c.timeout_secs,
                }
            }
        })
        .collect();
    let metadata = Arc::new(MetadataService::from_settings(
        &provider_settings,
        default_timeout,
    ));

    let backend = Arc::new(SsrBackend {
        db: db.clone(),
        mam: mam.clone(),
        metadata: metadata.clone(),
    });

    let ctx = Context {
        config: Arc::new(Mutex::new(config)),
        stats: Stats::new(),
        events: mlm_core::Events::new(),
        backend: Some(backend),
        triggers: mlm_core::Triggers::default(),
    };

    let app = Router::new()
        .nest_service("/assets", ServeDir::new("../server/assets"))
        .merge(mlm_web_dioxus::ssr::router(ctx));

    let addr: std::net::SocketAddr = "0.0.0.0:3002".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Dioxus dev server listening on http://{}", addr);

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
