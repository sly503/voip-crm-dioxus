use dioxus::prelude::*;
use crate::state::{UI_STATE, NotificationType};

#[component]
pub fn LoadingSpinner() -> Element {
    rsx! {
        div { class: "flex items-center justify-center p-4",
            div { class: "animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" }
        }
    }
}

#[component]
pub fn ErrorMessage(message: String) -> Element {
    rsx! {
        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
            p { "{message}" }
        }
    }
}

#[component]
pub fn Button(
    onclick: EventHandler<MouseEvent>,
    children: Element,
    #[props(default = "primary".to_string())]
    variant: String,
    #[props(default = false)]
    disabled: bool,
) -> Element {
    let class = match variant.as_str() {
        "primary" => "bg-blue-600 hover:bg-blue-700 text-white",
        "secondary" => "bg-gray-200 hover:bg-gray-300 text-gray-800",
        "danger" => "bg-red-600 hover:bg-red-700 text-white",
        "success" => "bg-green-600 hover:bg-green-700 text-white",
        _ => "bg-blue-600 hover:bg-blue-700 text-white",
    };

    let disabled_class = if disabled { "opacity-50 cursor-not-allowed" } else { "" };

    rsx! {
        button {
            class: "px-4 py-2 rounded font-medium transition-colors {class} {disabled_class}",
            disabled: disabled,
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}

#[component]
pub fn Card(children: Element, #[props(default = "".to_string())] class: String) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-4 {class}",
            {children}
        }
    }
}

#[component]
pub fn Badge(text: String, #[props(default = "bg-gray-100 text-gray-800".to_string())] color_class: String) -> Element {
    rsx! {
        span { class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {color_class}",
            "{text}"
        }
    }
}

#[component]
pub fn Notification() -> Element {
    let notification = UI_STATE.read().notification.clone();

    // Auto-dismiss notification after 4 seconds
    {
        let has_notification = notification.is_some();
        use_effect(move || {
            if has_notification {
                spawn(async move {
                    #[cfg(target_arch = "wasm32")]
                    {
                        gloo_timers::future::TimeoutFuture::new(4000).await;
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        tokio::time::sleep(std::time::Duration::from_millis(4000)).await;
                    }
                    crate::state::clear_notification();
                });
            }
        });
    }

    if let Some(notif) = notification {
        let color_class = notif.notification_type.color_class();
        let icon = match notif.notification_type {
            NotificationType::Success => "\u{2705}",
            NotificationType::Error => "\u{274C}",
            NotificationType::Warning => "\u{26A0}",
            NotificationType::Info => "\u{2139}",
        };
        rsx! {
            div {
                class: "fixed top-4 right-4 z-50 {color_class} text-white px-6 py-4 rounded-lg shadow-xl max-w-sm animate-slide-in",
                div { class: "flex items-start gap-3",
                    span { class: "text-xl flex-shrink-0", "{icon}" }
                    div { class: "flex-1",
                        p { class: "font-medium", "{notif.message}" }
                    }
                    button {
                        class: "ml-2 text-white hover:text-gray-200 flex-shrink-0",
                        onclick: move |_| {
                            crate::state::clear_notification();
                        },
                        "\u{2715}"
                    }
                }
            }
        }
    } else {
        rsx! {}
    }
}

#[component]
pub fn StatusIndicator(
    status: String,
    #[props(default = "bg-gray-400".to_string())]
    color_class: String,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-2",
            div { class: "w-3 h-3 rounded-full {color_class}" }
            span { class: "text-sm text-gray-600", "{status}" }
        }
    }
}

#[component]
pub fn Input(
    value: String,
    oninput: EventHandler<FormEvent>,
    #[props(default = "text".to_string())]
    input_type: String,
    #[props(default = "".to_string())]
    placeholder: String,
    #[props(default = "".to_string())]
    label: String,
    #[props(default = false)]
    required: bool,
) -> Element {
    rsx! {
        div { class: "mb-4",
            if !label.is_empty() {
                label { class: "block text-sm font-medium text-gray-700 mb-1",
                    "{label}"
                    if required {
                        span { class: "text-red-500", " *" }
                    }
                }
            }
            input {
                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                r#type: "{input_type}",
                value: "{value}",
                placeholder: "{placeholder}",
                required: required,
                oninput: move |e| oninput.call(e),
            }
        }
    }
}
