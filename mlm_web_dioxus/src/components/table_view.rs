use dioxus::prelude::*;

#[component]
pub fn TableView(class: String, style: String, children: Element) -> Element {
    rsx! {
        div { class: "{class}", style: "{style}", {children} }
    }
}

#[component]
pub fn TorrentGridTable(
    grid_template: String,
    extra_class: Option<String>,
    children: Element,
) -> Element {
    let class = if let Some(extra_class) = extra_class {
        format!("TorrentsTable table2 {extra_class}")
    } else {
        "TorrentsTable table2".to_string()
    };
    rsx! {
        div { class: "{class}", style: "--torrents-grid: {grid_template};", {children} }
    }
}
