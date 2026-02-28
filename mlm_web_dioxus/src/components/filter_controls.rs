use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct ColumnToggleOption {
    pub label: &'static str,
    pub checked: bool,
    pub on_toggle: EventHandler<bool>,
}

#[component]
pub fn ColumnSelector(options: Vec<ColumnToggleOption>) -> Element {
    rsx! {
        div { class: "option_group query",
            "Columns:"
            div {
                for option in options {
                    label {
                        "{option.label}"
                        input {
                            r#type: "checkbox",
                            checked: option.checked,
                            onchange: move |ev| option.on_toggle.call(ev.value() == "true"),
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn PageSizeSelector(
    page_size: usize,
    options: Vec<usize>,
    show_all_option: bool,
    on_change: EventHandler<usize>,
) -> Element {
    rsx! {
        div { class: "option_group query",
            "Page size: "
            select {
                value: "{page_size}",
                onchange: move |ev| {
                    if let Ok(v) = ev.value().parse::<usize>() {
                        on_change.call(v);
                    }
                },
                for option in options {
                    option { value: "{option}", "{option}" }
                }
                if show_all_option {
                    option { value: "0", "all" }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ActiveFilterChip {
    pub label: String,
    pub on_remove: EventHandler<()>,
}

#[component]
pub fn ActiveFilters(
    chips: Vec<ActiveFilterChip>,
    on_clear_all: Option<EventHandler<()>>,
) -> Element {
    if chips.is_empty() {
        return rsx! { "" };
    }

    rsx! {
        div { class: "option_group query",
            for chip in chips {
                span { class: "item",
                    "{chip.label}"
                    button {
                        r#type: "button",
                        "aria-label": "Remove {chip.label} filter",
                        onclick: move |_| chip.on_remove.call(()),
                        " ×"
                    }
                }
            }
            if let Some(on_clear_all) = on_clear_all {
                button {
                    r#type: "button",
                    onclick: move |_| on_clear_all.call(()),
                    "Clear filters"
                }
            }
        }
    }
}
