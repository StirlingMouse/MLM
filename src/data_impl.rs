use crate::data::{SelectedTorrent, Torrent, TorrentMeta};

impl Torrent {
    pub fn matches(&self, other: &Torrent) -> bool {
        // if self.hash == other.hash { return true };
        if self.title_search != other.title_search {
            return false;
        };
        self.meta.matches(&other.meta)
    }
}

impl TorrentMeta {
    pub(crate) fn matches(&self, other: &TorrentMeta) -> bool {
        self.main_cat == other.main_cat
            && self.authors.iter().any(|a| other.authors.contains(a))
            && self.narrators.iter().any(|a| other.narrators.contains(a))
    }
}
