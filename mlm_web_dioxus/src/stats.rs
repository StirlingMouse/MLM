use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct StatsData {
    pub autograbber_count: usize,
    pub last_run: Option<String>,
}

#[server]
pub async fn get_stats_data() -> Result<StatsData, ServerFnError> {
    use dioxus_fullstack::FullstackContext;
    use mlm_core::Context;

    let context: Context = FullstackContext::current()
        .and_then(|ctx| ctx.extension())
        .ok_or_else(|| ServerFnError::new("Context not found in extensions"))?;
    let stats = context.stats.values.lock().await;

    Ok(StatsData {
        autograbber_count: stats.autograbber_run_at.len(),
        last_run: stats
            .autograbber_run_at
            .values()
            .next_back()
            .map(|t| t.to_string()),
    })
}

#[component]
pub fn StatsPage() -> Element {
    let stats_data = use_server_future(move || async move { get_stats_data().await })?;

    let data = stats_data.suspend()?;
    let data = data.read();

    rsx! {
        div { class: "stats-page",
            h2 { "System Stats (Dioxus)" }

            match &*data {
                Ok(data) => rsx! {
                    ul {
                        li { "Autograbbers configured: {data.autograbber_count}" }
                        li { "Last run: {data.last_run.clone().unwrap_or_else(|| \"Never\".to_string())}" }
                    }
                    StatsIsland {}
                },
                Err(e) => rsx! {
                    p { "Error: {e}" }
                },
            }

            hr {}
            a { href: "/", "Back to Legacy Home" }
        }
    }
}

#[component]
fn StatsIsland() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        div { class: "island", style: "border: 1px solid #ccc; padding: 10px; margin-top: 20px;",
            h3 { "Interactive Island" }
            p { "This part is hydrated on the client." }
            button {
                onclick: move |_| count += 1,
                "Click me: {count}"
            }
        }
    }
}
