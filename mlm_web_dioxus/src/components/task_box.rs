use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct TaskBoxProps {
    pub last_run: Option<String>,
    pub result: Option<Result<(), String>>,
    #[props(default = None)]
    pub on_run: Option<EventHandler<()>>,
    #[props(default = true)]
    pub show_result: bool,
}

#[component]
pub fn TaskBox(props: TaskBoxProps) -> Element {
    let last_run = props
        .last_run
        .clone()
        .unwrap_or_else(|| "never".to_string());
    let has_run = props.last_run.is_some();
    let result_text = props
        .result
        .as_ref()
        .map(|res| match res {
            Ok(()) => "success".to_string(),
            Err(e) => e.clone(),
        })
        .unwrap_or_else(|| "running".to_string());
    let on_run = props.on_run;

    rsx! {
        p { "Last run: {last_run}" }
        if let Some(on_run) = on_run {
            button {
                onclick: move |_| on_run.call(()),
                "run now"
            }
        }
        if has_run && props.show_result {
            p { "Result: {result_text}" }
        }
    }
}
