use dioxus::prelude::*;

#[component]
pub fn StatusMessage(mut status_msg: Signal<Option<(String, bool)>>) -> Element {
    let Some((msg, is_error)) = status_msg.read().as_ref().cloned() else {
        return rsx! {};
    };

    let class = if is_error {
        "status-message error"
    } else {
        "status-message success"
    };

    rsx! {
        div {
            class,
            "{msg}"
            button {
                r#type: "button",
                onclick: move |_| status_msg.set(None),
                "⨯"
            }
        }
    }
}
