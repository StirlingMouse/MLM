use dioxus::prelude::*;

#[component]
pub fn TableView(class: String, style: String, children: Element) -> Element {
    rsx! {
        div { class: "{class}", style: "{style}", {children} }
    }
}
