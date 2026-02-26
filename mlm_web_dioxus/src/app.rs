use crate::duplicate::DuplicatePage;
use crate::errors::ErrorsPage;
use crate::events::EventsPage;
use crate::home::HomePage;
use crate::list::ListPage;
use crate::lists::ListsPage;
use crate::replaced::ReplacedPage;
use crate::search::SearchPage;
use crate::selected::SelectedPage;
#[cfg(feature = "web")]
use crate::sse::{
    trigger_errors_update, trigger_events_update, trigger_selected_update, trigger_stats_update,
    update_qbit_progress,
};
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
}

pub fn root() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
pub fn App() -> Element {
    use_hook(setup_sse);

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
            a { href: "/config", "Config" }
        }
        main { Outlet::<Route> {} }
    }
}

#[component]
fn EventsWithQuery(segments: Vec<String>) -> Element {
    rsx! { EventsPage {} }
}

#[component]
fn TorrentsWithQuery(segments: Vec<String>) -> Element {
    rsx! { TorrentsPage {} }
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

        fn connect_sse_data(url: &'static str, on_message: impl Fn(String) + 'static) {
            spawn(async move {
                match EventSource::new(url) {
                    Ok(es) => {
                        let callback =
                            Closure::<dyn FnMut(_)>::new(move |ev: web_sys::MessageEvent| {
                                if let Some(data) = ev.data().as_string() {
                                    on_message(data);
                                }
                            });
                        es.set_onmessage(Some(callback.as_ref().unchecked_ref()));
                        std::mem::forget(callback);
                        std::mem::forget(es);
                    }
                    Err(e) => tracing::error!("Failed to create EventSource for {}: {:?}", url, e),
                }
            });
        }

        connect_sse("/dioxus-stats-updates", trigger_stats_update);
        connect_sse("/dioxus-events-updates", trigger_events_update);
        connect_sse("/dioxus-selected-updates", trigger_selected_update);
        connect_sse("/dioxus-errors-updates", trigger_errors_update);
        connect_sse_data("/dioxus-qbit-progress", |data| {
            if let Ok(progress) = serde_json::from_str::<Vec<(u64, u32)>>(&data) {
                update_qbit_progress(progress);
            }
        });
    }
}
