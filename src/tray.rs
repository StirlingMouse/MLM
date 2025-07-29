use std::{path::PathBuf, process::exit, sync::Arc};

use anyhow::Result;
use tracing::error;
use tray_item::{IconSource, TrayItem};

use crate::config::Config;

pub fn start_tray_icon(
    log_dir: Option<PathBuf>,
    config_file: PathBuf,
    config: Arc<Config>,
) -> Result<TrayItem> {
    let mut tray = TrayItem::new("MLM", IconSource::Resource("tray-icon"))?;
    tray.add_label(&format!("MLM v{}", env!("CARGO_PKG_VERSION")))
        .unwrap();
    tray.add_menu_item("Open Web UI", move || {
        if let Err(err) = open::that(format!("http://localhost:{}", config.web_port)) {
            error!("Error opening web ui: {}", err);
        }
    })?;
    tray.add_menu_item("Open Config File", move || {
        if let Err(err) = open::that(&config_file) {
            error!("Error opening config file: {}", err);
        }
    })?;
    if let Some(log_dir) = log_dir {
        tray.add_menu_item("Open Log Directory", move || {
            if let Err(err) = open::that(&log_dir) {
                error!("Error opening log directory: {}", err);
            }
        })?;
    }
    tray.inner_mut().add_separator()?;
    tray.add_menu_item("Quit", || exit(0))?;
    Ok(tray)
}
