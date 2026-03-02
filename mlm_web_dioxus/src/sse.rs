use dioxus::prelude::*;

pub static STATS_UPDATE_TRIGGER: GlobalSignal<u32> = Signal::global(|| 0);
pub static EVENTS_UPDATE_TRIGGER: GlobalSignal<u32> = Signal::global(|| 0);
pub static SELECTED_UPDATE_TRIGGER: GlobalSignal<u32> = Signal::global(|| 0);
pub static ERRORS_UPDATE_TRIGGER: GlobalSignal<u32> = Signal::global(|| 0);
pub static QBIT_PROGRESS: GlobalSignal<Vec<(u64, u32)>> = Signal::global(Vec::new);

pub fn trigger_stats_update() {
    *STATS_UPDATE_TRIGGER.write() += 1;
}

pub fn trigger_events_update() {
    *EVENTS_UPDATE_TRIGGER.write() += 1;
}

pub fn trigger_selected_update() {
    *SELECTED_UPDATE_TRIGGER.write() += 1;
}

pub fn trigger_errors_update() {
    *ERRORS_UPDATE_TRIGGER.write() += 1;
}

pub fn update_qbit_progress(progress: Vec<(u64, u32)>) {
    *QBIT_PROGRESS.write() = progress;
}

/// Connects SSE streams for real-time updates. No-op on the server.
pub fn setup_sse() {
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
