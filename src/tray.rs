use std::{path::PathBuf, process::exit, sync::Arc};

use anyhow::Result;
use tracing::error;
use tray_item::{IconSource, TrayItem};

use crate::config::Config;

pub fn start_tray_icon(config_file: PathBuf, config: Arc<Config>) -> Result<TrayItem> {
    let mut tray = TrayItem::new("MLM", IconSource::Resource("tray-icon"))?;
    tray.add_label("MLM").unwrap();
    tray.add_menu_item("Open Webpage", move || {
        if let Err(err) = open::that(format!("http://localhost:{}", config.web_port)) {
            error!("Error opening webpage: {}", err);
        }
    })?;
    tray.add_menu_item("Open Config File", move || {
        if let Err(err) = open::that(&config_file) {
            error!("Error opening webpage: {}", err);
        }
    })?;
    tray.inner_mut().add_separator()?;
    tray.add_menu_item("Quit", || exit(0))?;
    Ok(tray)
}
