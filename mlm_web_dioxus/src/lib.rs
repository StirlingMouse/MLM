pub mod app;
pub mod components;
pub mod dto;
pub mod duplicate;
pub mod error;
pub mod errors;
pub mod events;
pub mod home;
pub mod list;
pub mod lists;
pub mod replaced;
pub mod search;
pub mod selected;
pub mod sse;
pub mod torrent_detail;
pub mod torrent_edit;
pub mod torrents;
pub mod utils;

#[cfg(feature = "server")]
pub mod ssr;

#[cfg(feature = "web")]
pub mod web {
    use crate::app::root;

    pub fn launch() {
        dioxus::launch(root);
    }
}
