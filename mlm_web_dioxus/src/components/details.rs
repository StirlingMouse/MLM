use dioxus::prelude::*;
use lucide_dioxus::ChevronRight;

#[component]
pub fn Details(label: String, open: Option<bool>, children: Element) -> Element {
    rsx! {
        details { open: open.unwrap_or(false),
            summary { class: "details-summary",
                span { class: "details-summary-icon",
                    ChevronRight { size: 16 }
                }
                span { class: "details-summary-label", "{label}" }
            }
            {children}
        }
    }
}
