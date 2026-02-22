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
                    {
                        let label = option.label;
                        let checked = option.checked;
                        let on_toggle = option.on_toggle;
                        rsx! {
                            label {
                                "{label}"
                                input {
                                    r#type: "checkbox",
                                    checked: checked,
                                    onchange: move |ev| on_toggle.call(ev.value() == "true"),
                                }
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
                {
                    let label = chip.label.clone();
                    let on_remove = chip.on_remove;
                    rsx! {
                        span { class: "item",
                            "{label}"
                            button {
                                r#type: "button",
                                onclick: move |_| on_remove.call(()),
                                " ×"
                            }
                        }
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
