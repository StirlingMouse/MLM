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

    #[route("/dioxus/events")]
    EventsPage {},

    #[route("/dioxus/events/:..segments")]
    EventsWithQuery { segments: Vec<String> },

    #[route("/dioxus/errors")]
    ErrorsPage {},

    #[route("/dioxus/selected")]
    SelectedPage {},

    #[route("/dioxus/replaced")]
    ReplacedPage {},

    #[route("/dioxus/duplicate")]
    DuplicatePage {},

    #[route("/dioxus/torrents")]
    TorrentsPage {},

    #[route("/dioxus/torrents/:id")]
    TorrentDetailPage { id: String },

    #[route("/dioxus/torrents/:id/edit")]
    TorrentEditPage { id: String },

    #[route("/dioxus/torrents/:..segments")]
    TorrentsWithQuery { segments: Vec<String> },

    #[route("/dioxus/search")]
    SearchPage {},

    #[route("/dioxus/lists")]
    ListsPage {},

    #[route("/dioxus/lists/:id")]
    ListPage { id: String },

    #[route("/dioxus/config")]
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

    rsx! {
        document::Title { "MLM - Dioxus" }
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
    rsx! { EventsPage {} }
}

#[component]
fn TorrentsWithQuery(segments: Vec<String>) -> Element {
    rsx! { TorrentsPage {} }
}
