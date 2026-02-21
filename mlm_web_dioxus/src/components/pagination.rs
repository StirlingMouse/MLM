use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct PaginationProps {
    pub total: usize,
    pub from: usize,
    pub page_size: usize,
    pub on_change: EventHandler<usize>,
}

#[component]
pub fn Pagination(props: PaginationProps) -> Element {
    if props.page_size == 0 || props.total <= props.page_size {
        return rsx! { "" };
    }

    let max_pages = 7;
    let num_pages = (props.total as f64 / props.page_size as f64).ceil() as usize;
    let current_page = props.from / props.page_size + 1;

    let pages = {
        if num_pages > max_pages {
            let half = max_pages / 2;
            if current_page <= half {
                1..=max_pages
            } else if current_page >= num_pages - half {
                (num_pages - max_pages + 1)..=num_pages
            } else {
                (current_page - half)..=(current_page + half)
            }
        } else {
            1..=num_pages
        }
    };

    rsx! {
        div { class: "pagination",
            if num_pages > max_pages {
                button {
                    r#type: "button",
                    class: if current_page == 1 { "disabled" },
                    onclick: move |_| props.on_change.call(0),
                    "«"
                }
            }
            button {
                r#type: "button",
                class: if current_page == 1 { "disabled" },
                onclick: move |_| props.on_change.call(props.from.saturating_sub(props.page_size)),
                "‹"
            }
            div {
                for p in pages {
                    {
                        let p_from = (p - 1) * props.page_size;
                        let active = p == current_page;
                        rsx! {
                            button {
                                r#type: "button",
                                class: if active { "active" },
                                onclick: move |_| props.on_change.call(p_from),
                                "{p}"
                            }
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: if current_page == num_pages { "disabled" },
                onclick: move |_| {
                    props.on_change.call(
                        (props.from + props.page_size).min((num_pages - 1) * props.page_size)
                    )
                },
                "›"
            }
            if num_pages > max_pages {
                button {
                    r#type: "button",
                    class: if current_page == num_pages { "disabled" },
                    onclick: move |_| props.on_change.call((num_pages - 1) * props.page_size),
                    "»"
                }
            }
        }
    }
}
