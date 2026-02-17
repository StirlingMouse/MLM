use crate::events::EventsPage;
use crate::home::HomePage;
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
            a { href: "/search", "Search" }
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

fn setup_sse() {
    #[cfg(feature = "web")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::prelude::*;
        use web_sys::EventSource;

        // Stats SSE
        spawn(async move {
            match EventSource::new("/dioxus-stats-updates") {
                Ok(es) => {
                    let onmessage_callback =
                        Closure::<dyn FnMut(_)>::new(move |_event: web_sys::MessageEvent| {
                            crate::home::trigger_stats_update();
                        });
                    es.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
                    onmessage_callback.forget();
                    // Prevent ES from being dropped
                    Box::leak(Box::new(es));
                }
                Err(e) => tracing::error!("Failed to create EventSource for stats: {:?}", e),
            }
        });

        // Events SSE
        spawn(async move {
            match EventSource::new("/dioxus-events-updates") {
                Ok(es) => {
                    let onmessage_callback =
                        Closure::<dyn FnMut(_)>::new(move |_event: web_sys::MessageEvent| {
                            crate::events::trigger_events_update();
                        });
                    es.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
                    onmessage_callback.forget();
                    // Prevent ES from being dropped
                    Box::leak(Box::new(es));
                }
                Err(e) => tracing::error!("Failed to create EventSource for events: {:?}", e),
            }
        });
    }
}
