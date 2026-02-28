use dioxus::prelude::*;

#[component]
pub fn SortHeader<S: Copy + PartialEq + 'static>(
    label: String,
    sort_key: S,
    sort: Signal<Option<S>>,
    mut asc: Signal<bool>,
    mut from: Signal<usize>,
) -> Element {
    let mut sort = sort;
    let active = *sort.read() == Some(sort_key);
    let arrow = if active {
        if *asc.read() { "↑" } else { "↓" }
    } else {
        ""
    };
    rsx! {
        div { class: "header",
            button {
                r#type: "button",
                class: "link",
                onclick: move |_| {
                    if *sort.read() == Some(sort_key) {
                        let next_asc = !*asc.read();
                        asc.set(next_asc);
                    } else {
                        sort.set(Some(sort_key));
                        asc.set(false);
                    }
                    from.set(0);
                },
                "{label}{arrow}"
            }
        }
    }
}
