use crate::events::EventsPage;
use crate::home::HomePage;
use crate::search::SearchPage;
#[cfg(feature = "web")]
use crate::sse::{trigger_events_update, trigger_stats_update};
use crate::stats::StatsPage;
use crate::torrent_detail::TorrentDetailPage;
use crate::torrents::TorrentsPage;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Routable, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[rustfmt::skip]
pub enum Route {
    #[layout(App)]
    #[route("/dioxus/")]
    Home {},

    #[route("/dioxus/stats")]
    Stats {},

    #[route("/dioxus/events")]
    Events {},

    #[route("/dioxus/events/:..segments")]
    EventsWithQuery { segments: Vec<String> },

    #[route("/dioxus/torrents")]
    Torrents {},

    #[route("/dioxus/torrents/:id")]
    TorrentDetail { id: String },

    #[route("/dioxus/torrents/:..segments")]
    TorrentsWithQuery { segments: Vec<String> },

    #[route("/dioxus/search")]
    Search {},
}

pub fn root() -> Element {
    rsx! { Router::<Route> {} }
}

#[component]
pub fn App() -> Element {
    use_hook(setup_sse);

    rsx! {
        document::Title { "MLM - Dioxus" }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        document::Link { rel: "icon", r#type: "image/png", href: "/assets/favicon.png" }
        document::Link { rel: "stylesheet", href: "/assets/style.css" }

        nav {
            Link { to: Route::Home {}, "Home (Dioxus)" }
            a { href: "/", "Home (Legacy)" }
            a { href: "/torrents", "Torrents" }
            Link { to: Route::Events {}, "Events" }
            Link { to: Route::Search {}, "Search" }
            a { href: "/lists", "Goodreads lists" }
            a { href: "/errors", "Errors" }
            a { href: "/selected", "Selected Torrents" }
            a { href: "/replaced", "Replaced Torrents" }
            a { href: "/duplicate", "Duplicate Torrents" }
            a { href: "/config", "Config" }
        }
        main {
            Outlet::<Route> {}
        }
    }
}

#[component]
fn Home() -> Element {
    rsx! { HomePage {} }
}

#[component]
fn Stats() -> Element {
    rsx! { StatsPage {} }
}

#[component]
fn Events() -> Element {
    rsx! { EventsPage {} }
}

#[component]
fn EventsWithQuery(segments: Vec<String>) -> Element {
    rsx! { EventsPage {} }
}

#[component]
fn Torrents() -> Element {
    rsx! { TorrentsPage {} }
}

#[component]
fn TorrentsWithQuery(segments: Vec<String>) -> Element {
    rsx! { TorrentsPage {} }
}

#[component]
fn TorrentDetail(id: String) -> Element {
    rsx! { TorrentDetailPage { id } }
}

#[component]
fn Search() -> Element {
    rsx! { SearchPage {} }
}

fn setup_sse() {
    #[cfg(feature = "web")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::prelude::*;
        use web_sys::EventSource;

        fn connect_sse(url: &'static str, on_message: impl Fn() + 'static) {
            spawn(async move {
                match EventSource::new(url) {
                    Ok(es) => {
                        let callback =
                            Closure::<dyn FnMut(_)>::new(move |_: web_sys::MessageEvent| {
                                on_message();
                            });
                        es.set_onmessage(Some(callback.as_ref().unchecked_ref()));
                        // Intentionally leak to keep SSE connection alive for app lifetime.
                        // Browser cleans up on page unload.
                        std::mem::forget(callback);
                        std::mem::forget(es);
                    }
                    Err(e) => tracing::error!("Failed to create EventSource for {}: {:?}", url, e),
                }
            });
        }

        connect_sse("/dioxus-stats-updates", trigger_stats_update);
        connect_sse("/dioxus-events-updates", trigger_events_update);
    }
}
