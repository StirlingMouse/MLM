use dioxus::prelude::*;

#[component]
pub fn Details(label: String, open: Option<bool>, children: Element) -> Element {
    rsx! {
        details { open: open.unwrap_or(false),
            summary { class: "details-summary", "{label}" }
            {children}
        }
    }
}
