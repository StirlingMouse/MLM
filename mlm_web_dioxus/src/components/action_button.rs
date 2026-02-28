use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ActionButtonProps {
    pub label: String,
    pub onclick: EventHandler<()>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = None)]
    pub class: Option<String>,
    #[props(default = None)]
    pub style: Option<String>,
    #[props(default = None)]
    pub loading_label: Option<String>,
    #[props(default = false)]
    pub loading: bool,
}

#[component]
pub fn ActionButton(props: ActionButtonProps) -> Element {
    let class = props.class.clone().unwrap_or_else(|| "btn".to_string());
    let style = props.style.clone().unwrap_or_default();
    let label = if props.loading {
        props
            .loading_label
            .clone()
            .unwrap_or_else(|| "...".to_string())
    } else {
        props.label.clone()
    };

    rsx! {
        button {
            class: "{class}",
            style: "{style}",
            disabled: props.disabled || props.loading,
            onclick: move |_| props.onclick.call(()),
            "{label}"
        }
    }
}
