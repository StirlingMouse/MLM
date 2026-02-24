use dioxus::prelude::*;

#[component]
pub fn StatusMessage(mut status_msg: Signal<Option<(String, bool)>>) -> Element {
    let Some((msg, is_error)) = status_msg.read().as_ref().cloned() else {
        return rsx! {};
    };

    rsx! {
        div {
            class: if is_error { "error" } else { "success" },
            style: if is_error {
                "padding: 10px; margin-bottom: 10px; border-radius: 4px; color: #000; background: #fdd;"
            } else {
                "padding: 10px; margin-bottom: 10px; border-radius: 4px; color: #000; background: #dfd;"
            },
            "{msg}"
            button {
                style: "margin-left: 10px; cursor: pointer;",
                onclick: move |_| status_msg.set(None),
                "⨯"
            }
        }
    }
}
