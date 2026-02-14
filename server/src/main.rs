#![windows_subsystem = "windows"]

use std::{
    env,
    fs::{self, create_dir_all},
    io, mem,
    path::PathBuf,
    process,
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use dirs::{config_dir, data_local_dir};
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use mlm_mam::api::MaM;
use tracing::error;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    EnvFilter, Layer as _, fmt::time::LocalTime, layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
};

use mlm_core::{config::Config, metadata::MetadataService, stats::Stats};
use mlm_web_askama::start_webserver;

#[cfg(target_family = "windows")]
use mlm::windows;

#[tokio::main]
async fn main() {
    if let Err(err) = app_main().await {
        #[cfg(target_family = "windows")]
        windows::error_window::ErrorWindow::create_and_run(
            "MLM App Error".to_string(),
            err.to_string(),
            None,
        )
        .unwrap();
        error!("AppError: {err:?}");
        eprintln!("{:?}", err);
        process::exit(1);
    }
}

async fn app_main() -> Result<()> {
    let log_dir = env::var("MLM_LOG_DIR")
        .map(|path| {
            if path.is_empty() {
                None
            } else {
                Some(PathBuf::from(path))
            }
        })
        .unwrap_or_else(|_| {
            #[cfg(all(debug_assertions, not(windows)))]
            return None;
            #[allow(unused)]
            Some(
                data_local_dir()
                    .map(|d| d.join("MLM").join("logs"))
                    .unwrap_or_else(|| "logs".into()),
            )
        });

    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stderr);

    let file_layer = log_dir
        .as_ref()
        .map(|log_dir| {
            Result::<_, anyhow::Error>::Ok(
                tracing_subscriber::fmt::layer().pretty().with_writer(
                    RollingFileAppender::builder()
                        .rotation(Rotation::DAILY)
                        .filename_prefix("mlm")
                        .filename_suffix("log")
                        .build(log_dir)?,
                ),
            )
        })
        .transpose()?;

    tracing_subscriber::registry()
        .with(
            stderr_layer.with_timer(LocalTime::rfc_3339()).with_filter(
                EnvFilter::builder()
                    .with_default_directive("mlm=debug".parse()?)
                    .with_env_var("MLM_LOG")
                    .from_env_lossy(),
            ),
        )
        .with(file_layer.map(|file_layer| {
            file_layer
                .with_timer(LocalTime::rfc_3339())
                .with_ansi(false)
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive("mlm=debug".parse().unwrap())
                        .with_env_var("MLM_LOG")
                        .from_env_lossy(),
                )
        }))
        .try_init()?;
    #[cfg(target_family = "windows")]
    std::panic::set_hook(Box::new(tracing_panic::panic_hook));

    let config_file = env::var("MLM_CONFIG_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            #[cfg(debug_assertions)]
            return "config.toml".into();
            #[allow(unused)]
            config_dir()
                .map(|d| d.join("MLM").join("config.toml"))
                .unwrap_or_else(|| "config.toml".into())
        });
    let database_file = env::var("MLM_DB_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            #[cfg(debug_assertions)]
            return "data.db".into();
            #[allow(unused)]
            data_local_dir()
                .map(|d| d.join("MLM").join("data.db"))
                .unwrap_or_else(|| "data.db".into())
        });
    if !config_file.exists() {
        if let Some(dir) = config_file.parent() {
            create_dir_all(dir)?;
        }
        let default_config = r#"mam_id = """#;
        fs::write(&config_file, default_config)?;
    }
    if !database_file.exists()
        && let Some(dir) = database_file.parent()
    {
        create_dir_all(dir)?;
    }
    let config: Result<Config, _> = Figment::new()
        .merge(Toml::file_exact(&config_file))
        .merge(Env::prefixed("MLM_CONF_"))
        .extract();
    #[cfg(target_family = "windows")]
    if let Err(err) = &config {
        windows::error_window::ErrorWindow::create_and_run(
            "MLM Config Error".to_string(),
            err.to_string(),
            Some(config_file.clone()),
        )
        .unwrap();
        return Ok(());
    }
    let mut config = config?;
    for autograb in &mut config.autograbs {
        autograb.filter.edition = mem::take(&mut autograb.edition);
    }
    for snatchlist in &mut config.snatchlist {
        snatchlist.filter.edition = mem::take(&mut snatchlist.edition);
    }
    for list in &mut config.goodreads_lists {
        for grab in &mut list.grab {
            grab.filter.edition = mem::take(&mut grab.edition);
        }
    }
    for list in &mut config.notion_lists {
        for grab in &mut list.grab {
            grab.filter.edition = mem::take(&mut grab.edition);
        }
    }
    for tag in &mut config.tags {
        tag.filter.edition = mem::take(&mut tag.edition);
    }
    let config = Arc::new(config);

    let db = native_db::Builder::new().create(&mlm_db::MODELS, database_file)?;
    mlm_db::migrate(&db)?;

    if env::args().any(|arg| arg == "--update-search-title") {
        mlm_db::update_search_title(&db)?;
        return Ok(());
    }

    let db = Arc::new(db);

    #[cfg(target_family = "windows")]
    let _tray = windows::tray::start_tray_icon(log_dir, config_file.clone(), config.clone())?;

    let stats = Stats::new();

    // Instantiate metadata service from config provider settings
    let default_timeout = Duration::from_secs(5);
    // Convert Config's ProviderConfig -> metadata::ProviderSetting
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
    let metadata_service = MetadataService::from_settings(&provider_settings, default_timeout);
    let metadata_service = Arc::new(metadata_service);

    let mam = if config.mam_id.is_empty() {
        Err(anyhow::Error::msg("No mam_id set"))
    } else {
        MaM::new(&config.mam_id, db.clone()).await.map(Arc::new)
    };

    #[cfg(target_family = "windows")]
    let web_port = config.web_port;

    let context = mlm_core::runner::spawn_tasks(config, db, Arc::new(mam), stats, metadata_service);

    let result = start_webserver(context).await;

    #[cfg(target_family = "windows")]
    if let Err(err) = &result {
        windows::error_window::ErrorWindow::create_and_run(
            "MLM Webserver Error".to_string(),
            format!(
                "{err}\r\n\r\nThis usually mean that your port is in use.\r\nConfigured port: {}",
                web_port
            ),
            Some(config_file.clone()),
        )
        .unwrap();
        return Ok(());
    }
    result?;

    Ok(())
}
