use dioxus::prelude::*;

#[component]
pub fn TorrentGridTable(
    grid_template: String,
    extra_class: Option<String>,
    pending: bool,
    children: Element,
) -> Element {
    let refresh_class = if pending { " is-refreshing" } else { "" };
    let class = if let Some(extra_class) = extra_class {
        format!("TorrentsTable table2 {extra_class}{refresh_class}")
    } else {
        format!("TorrentsTable table2{refresh_class}")
    };
    rsx! {
        div { class: "{class}", style: "--torrents-grid: {grid_template};",
            {children}
            if pending {
                div { class: "stale-refresh-overlay",
                    div { class: "stale-refresh-spinner" }
                }
            }
        }
    }
}
