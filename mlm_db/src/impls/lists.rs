use crate::ListItemTorrent;

impl ListItemTorrent {
    pub fn id(&self) -> String {
        self.torrent_id
            .clone()
            .or_else(|| self.mam_id.map(|id| id.to_string()))
            .unwrap_or_default()
    }
}
