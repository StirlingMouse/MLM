use crate::config::ConfigPage;
use crate::duplicate::DuplicatePage;
use crate::errors::ErrorsPage;
use crate::events::EventsPage;
use crate::home::HomePage;
use crate::list::ListPage;
use crate::lists::ListsPage;
use crate::replaced::ReplacedPage;
use crate::search::SearchPage;
use crate::selected::SelectedPage;
use crate::torrent_detail::TorrentDetailPage;
use crate::torrent_edit::TorrentEditPage;
use crate::torrents::TorrentsPage;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

const GLOBAL_STYLE_CSS: &str = include_str!("../../server/assets/style.css");

#[derive(Clone, Routable, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[rustfmt::skip]
pub enum Route {
    #[layout(App)]
    #[route("/")]
    HomePage {},

    #[route("/events")]
    EventsPage {},

    #[route("/events/:..segments")]
    EventsWithQuery { segments: Vec<String> },

    #[route("/errors")]
    ErrorsPage {},

    #[route("/selected")]
    SelectedPage {},

    #[route("/replaced")]
    ReplacedPage {},

    #[route("/duplicate")]
    DuplicatePage {},

    #[route("/torrents")]
    TorrentsPage {},

    #[route("/torrent-edit/:id")]
    TorrentEditPage { id: String },

    #[route("/torrents/:id")]
    TorrentDetailPage { id: String },

    #[route("/torrents/:..segments")]
    TorrentsWithQuery { segments: Vec<String> },

    #[route("/search")]
    SearchPage {},

    #[route("/lists")]
    ListsPage {},

    #[route("/lists/:id")]
    ListPage { id: String },

    #[route("/config")]
    ConfigPage {},
}

pub fn root() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
pub fn App() -> Element {
    use_hook(crate::sse::setup_sse);
    let route: Route = use_route();

    let page_title = match route {
        Route::HomePage {} => "MLM",
        Route::EventsPage {} | Route::EventsWithQuery { .. } => "MLM - Events",
        Route::ErrorsPage {} => "MLM - Errors",
        Route::SelectedPage {} => "MLM - Selected Torrents",
        Route::ReplacedPage {} => "MLM - Replaced Torrents",
        Route::DuplicatePage {} => "MLM - Duplicate Torrents",
        Route::TorrentsPage {} | Route::TorrentsWithQuery { .. } => "MLM - Torrents",
        Route::TorrentDetailPage { .. } => "MLM - Torrent",
        Route::TorrentEditPage { .. } => "MLM - Edit Torrent",
        Route::SearchPage {} => "MLM - Search",
        Route::ListsPage {} => "MLM - Goodreads Lists",
        Route::ListPage { .. } => "MLM - List",
        Route::ConfigPage {} => "MLM - Config",
    };

    rsx! {
        document::Title { "{page_title}" }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        document::Link { rel: "icon", r#type: "image/png", href: "/assets/favicon.png" }
        document::Style { "{GLOBAL_STYLE_CSS}" }

        nav { "aria-label": "Main navigation",
            Link { to: Route::HomePage {}, "Home" }
            Link { to: Route::TorrentsPage {}, "Torrents" }
            Link { to: Route::EventsPage {}, "Events" }
            Link { to: Route::SearchPage {}, "Search" }
            Link { to: Route::ListsPage {}, "Goodreads Lists" }
            Link { to: Route::ErrorsPage {}, "Errors" }
            Link { to: Route::SelectedPage {}, "Selected Torrents" }
            Link { to: Route::ReplacedPage {}, "Replaced Torrents" }
            Link { to: Route::DuplicatePage {}, "Duplicate Torrents" }
            Link { to: Route::ConfigPage {}, "Config" }
        }
        main { Outlet::<Route> {} }
    }
}

// Dioxus's router requires a distinct path segment to differentiate routes, but these
// pages read their filter state directly from the URL query string. These catch-all
// variants absorb any trailing path segments so that query-string-only navigations
// (e.g. `?kind=audiobook`) still land on the right page component.

#[component]
fn EventsWithQuery(segments: Vec<String>) -> Element {
    let _ = segments;
    rsx! { EventsPage {} }
}

#[component]
fn TorrentsWithQuery(segments: Vec<String>) -> Element {
    let _ = segments;
    rsx! { TorrentsPage {} }
}

#[cfg(test)]
mod tests {
    use super::Route;
    use std::str::FromStr;

    #[test]
    fn parses_torrent_edit_route() {
        let route = Route::from_str("/torrent-edit/torrent-001").expect("route should parse");
        assert_eq!(route.to_string(), "/torrent-edit/torrent-001");
        assert_eq!(
            route,
            Route::TorrentEditPage {
                id: "torrent-001".to_string(),
            }
        );
    }
}
