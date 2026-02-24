use crate::torrent_detail::select_torrent_action;
use dioxus::prelude::*;

/// Display mode for download buttons
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloadButtonMode {
    /// Full text labels (default)
    #[default]
    Full,
    /// Compact icon labels (↓ and W)
    Compact,
}

/// Reusable download buttons component.
///
/// Shows appropriate download options based on torrent status:
/// - VIP torrents: "Download as VIP"
/// - Personal Freeleech: "Download as Personal Freeleech"
/// - Global Freeleech: "Download as Global Freeleech"
/// - Regular: "Download with Wedge" + "Download with Ratio" (or compact versions)
///
/// The component manages its own loading state internally.
#[derive(Props, Clone, PartialEq)]
pub struct DownloadButtonsProps {
    /// The MaM ID of the torrent
    pub mam_id: u64,
    /// Whether this is a VIP torrent
    pub is_vip: bool,
    /// Whether this is global freeleech
    pub is_free: bool,
    /// Whether this is personal freeleech
    pub is_personal_freeleech: bool,
    /// Whether wedge download is available
    pub can_wedge: bool,
    /// External disabled state (e.g., when another operation is in progress)
    pub disabled: bool,
    /// Display mode: Full labels or compact icons
    #[props(default)]
    pub mode: DownloadButtonMode,
    /// Callback for status messages (message, is_error)
    pub on_status: EventHandler<(String, bool)>,
    /// Callback when download is triggered successfully
    pub on_refresh: EventHandler<()>,
}

#[component]
pub fn DownloadButtons(props: DownloadButtonsProps) -> Element {
    let mut loading = use_signal(|| false);
    let mam_id = props.mam_id;

    let mut handle_download = move |wedge: bool, success_msg: String| {
        loading.set(true);
        props.on_status.call((String::new(), false)); // Clear previous status
        spawn(async move {
            match select_torrent_action(mam_id, wedge).await {
                Ok(_) => {
                    props.on_status.call((success_msg, false));
                    props.on_refresh.call(());
                }
                Err(e) => {
                    props
                        .on_status
                        .call((format!("Selection failed: {e}"), true));
                }
            }
            loading.set(false);
        });
    };

    let is_disabled = *loading.read() || props.disabled;
    let auto_wedge =
        props.can_wedge && !props.is_vip && !props.is_personal_freeleech && !props.is_free;

    rsx! {
        if props.is_vip {
            button {
                class: if props.mode == DownloadButtonMode::Compact { "icon" } else { "btn" },
                disabled: is_disabled,
                onclick: move |_| {
                    handle_download(false, "Torrent queued for download".to_string());
                },
                if *loading.read() {
                    if props.mode == DownloadButtonMode::Compact {
                        img {
                            src: "/assets/icons/down.png",
                            alt: "Downloading",
                            title: "Downloading",
                            style: "filter:saturate(0)",
                        }
                    } else {
                        "..."
                    }
                } else if props.mode == DownloadButtonMode::Compact {
                    img {
                        src: "/assets/icons/down.png",
                        alt: "Download",
                        title: "Download",
                    }
                } else {
                    "Download as VIP"
                }
            }
        } else if props.is_personal_freeleech {
            button {
                class: if props.mode == DownloadButtonMode::Compact { "icon" } else { "btn" },
                disabled: is_disabled,
                onclick: move |_| {
                    handle_download(false, "Torrent queued for download".to_string());
                },
                if *loading.read() {
                    if props.mode == DownloadButtonMode::Compact {
                        img {
                            src: "/assets/icons/down.png",
                            alt: "Downloading",
                            title: "Downloading",
                            style: "filter:saturate(0)",
                        }
                    } else {
                        "..."
                    }
                } else if props.mode == DownloadButtonMode::Compact {
                    img {
                        src: "/assets/icons/down.png",
                        alt: "Download",
                        title: "Download",
                    }
                } else {
                    "Download as Personal Freeleech"
                }
            }
        } else if props.is_free {
            button {
                class: if props.mode == DownloadButtonMode::Compact { "icon" } else { "btn" },
                disabled: is_disabled,
                onclick: move |_| {
                    handle_download(false, "Torrent queued for download".to_string());
                },
                if *loading.read() {
                    if props.mode == DownloadButtonMode::Compact {
                        img {
                            src: "/assets/icons/down.png",
                            alt: "Downloading",
                            title: "Downloading",
                            style: "filter:saturate(0)",
                        }
                    } else {
                        "..."
                    }
                } else if props.mode == DownloadButtonMode::Compact {
                    img {
                        src: "/assets/icons/down.png",
                        alt: "Download",
                        title: "Download",
                    }
                } else {
                    "Download as Global Freeleech"
                }
            }
        } else {
            // Regular download options
            if props.mode == DownloadButtonMode::Compact {
                button {
                    class: "icon",
                    style: if auto_wedge { "filter:hue-rotate(180deg)" },
                    disabled: is_disabled,
                    onclick: move |_| {
                        handle_download(
                            auto_wedge,
                            if auto_wedge {
                                "Torrent queued with wedge".to_string()
                            } else {
                                "Torrent queued for download".to_string()
                            },
                        );
                    },
                    if *loading.read() {
                        img {
                            src: "/assets/icons/down.png",
                            alt: "Downloading",
                            title: "Downloading",
                            style: "filter:saturate(0)",
                        }
                    } else {
                        img {
                            src: "/assets/icons/down.png",
                            alt: if auto_wedge { "Download with Wedge" } else { "Download" },
                            title: if auto_wedge { "Download with Wedge" } else { "Download" },
                        }
                    }
                }
            } else {
                // Full mode: text buttons
                if props.can_wedge {
                    button {
                        class: "btn",
                        disabled: is_disabled,
                        onclick: move |_| {
                            handle_download(true, "Torrent queued with wedge".to_string());
                        },
                        if *loading.read() { "..." } else { "Download with Wedge" }
                    }
                }
                button {
                    class: "btn",
                    disabled: is_disabled,
                    onclick: move |_| {
                        handle_download(false, "Torrent queued for download".to_string());
                    },
                    if *loading.read() { "..." } else { "Download with Ratio" }
                }
            }
        }
    }
}

/// Simplified download buttons for torrents that are already known to be regular (non-freeleech).
/// Shows download + optional wedge buttons.
#[derive(Props, Clone, PartialEq)]
pub struct SimpleDownloadButtonsProps {
    /// The MaM ID of the torrent
    pub mam_id: u64,
    /// Whether wedge download is available
    pub can_wedge: bool,
    /// External disabled state
    pub disabled: bool,
    /// Display mode: Full labels or compact icons
    #[props(default)]
    pub mode: DownloadButtonMode,
    /// Callback for status messages (message, is_error)
    pub on_status: EventHandler<(String, bool)>,
    /// Callback when download is triggered successfully
    pub on_refresh: EventHandler<()>,
}

#[component]
pub fn SimpleDownloadButtons(props: SimpleDownloadButtonsProps) -> Element {
    rsx! {
        DownloadButtons {
            mam_id: props.mam_id,
            is_vip: false,
            is_free: false,
            is_personal_freeleech: false,
            can_wedge: props.can_wedge,
            disabled: props.disabled,
            mode: props.mode,
            on_status: props.on_status,
            on_refresh: props.on_refresh,
        }
    }
}
