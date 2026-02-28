use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct ColumnToggleOption {
    pub label: &'static str,
    pub checked: bool,
    pub on_toggle: EventHandler<bool>,
}

#[component]
pub fn ColumnSelector(options: Vec<ColumnToggleOption>) -> Element {
    let mut is_open = use_signal(|| false);
    let selected_count = options.iter().filter(|option| option.checked).count();
    let total_count = options.len();

    rsx! {
        div { class: "option_group query column_selector",
            div {
                class: "column_selector_dropdown",
                button {
                    r#type: "button",
                    class: "column_selector_trigger",
                    "aria-expanded": if *is_open.read() { "true" } else { "false" },
                    onclick: move |_| {
                        let next = !*is_open.read();
                        is_open.set(next);
                    },
                    "Columns ({selected_count}/{total_count})"
                }
                if *is_open.read() {
                    div { class: "column_selector_menu",
                        for option in options {
                            label { class: "column_selector_option",
                                input {
                                    r#type: "checkbox",
                                    checked: option.checked,
                                    onchange: move |ev| option.on_toggle.call(ev.value() == "true"),
                                }
                                span { "{option.label}" }
                            }
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
