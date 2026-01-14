//! WebRTC-based phone dialer
//!
//! This component provides browser-based calling using Telnyx WebRTC.

use dioxus::prelude::*;
use crate::state::{NotificationType, show_notification, WEBRTC_STATE};
#[cfg(target_arch = "wasm32")]
use crate::state::{set_webrtc_connecting, set_webrtc_connected, set_webrtc_error, set_webrtc_call_state, clear_webrtc_call};

#[cfg(target_arch = "wasm32")]
use super::webrtc::{init_telnyx_webrtc, make_webrtc_call, hangup_webrtc_call, toggle_mute_webrtc, is_telnyx_ready};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
use js_sys::Reflect;

/// Play DTMF tone for a digit
#[cfg(target_arch = "wasm32")]
fn play_dtmf_tone(digit: &str) {
    use web_sys::{AudioContext, OscillatorType};

    let (low_freq, high_freq) = match digit {
        "1" => (697.0, 1209.0),
        "2" => (697.0, 1336.0),
        "3" => (697.0, 1477.0),
        "4" => (770.0, 1209.0),
        "5" => (770.0, 1336.0),
        "6" => (770.0, 1477.0),
        "7" => (852.0, 1209.0),
        "8" => (852.0, 1336.0),
        "9" => (852.0, 1477.0),
        "*" => (941.0, 1209.0),
        "0" => (941.0, 1336.0),
        "#" => (941.0, 1477.0),
        _ => return,
    };

    if let Ok(ctx) = AudioContext::new() {
        let duration = 0.15;
        let current_time = ctx.current_time();

        if let Ok(gain) = ctx.create_gain() {
            let _ = gain.gain().set_value(0.1);
            let _ = gain.connect_with_audio_node(&ctx.destination());

            if let Ok(osc1) = ctx.create_oscillator() {
                osc1.set_type(OscillatorType::Sine);
                let _ = osc1.frequency().set_value(low_freq as f32);
                let _ = osc1.connect_with_audio_node(&gain);
                let _ = osc1.start();
                let _ = osc1.stop_with_when(current_time + duration);
            }

            if let Ok(osc2) = ctx.create_oscillator() {
                osc2.set_type(OscillatorType::Sine);
                let _ = osc2.frequency().set_value(high_freq as f32);
                let _ = osc2.connect_with_audio_node(&gain);
                let _ = osc2.start();
                let _ = osc2.stop_with_when(current_time + duration);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn play_dtmf_tone(_digit: &str) {}

/// Format phone number to E.164
#[cfg(target_arch = "wasm32")]
fn format_e164(number: &str) -> String {
    let cleaned: String = number.chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect();

    if cleaned.starts_with('+') {
        return cleaned;
    }

    if cleaned.len() == 10 {
        return format!("+1{}", cleaned);
    }

    if cleaned.len() == 11 && cleaned.starts_with('1') {
        return format!("+{}", cleaned);
    }

    format!("+{}", cleaned)
}

#[component]
pub fn WebRTCDialer() -> Element {
    let mut phone_number = use_signal(String::new);
    #[allow(unused_mut, unused_variables)]
    let mut caller_id = use_signal(String::new);
    let webrtc_state = WEBRTC_STATE.read();

    // Fetch WebRTC config and initialize on mount
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        spawn(async move {
            if is_telnyx_ready() {
                set_webrtc_connected();
                return;
            }

            // Fetch config from server
            match crate::api::get_webrtc_config().await {
                Ok(config) => {
                    caller_id.set(config.caller_id.clone());

                    if config.sip_username.is_empty() || config.sip_password.is_empty() {
                        set_webrtc_error("WebRTC not configured. Set TELNYX_SIP_USERNAME and TELNYX_SIP_PASSWORD".to_string());
                        show_notification("WebRTC credentials not configured", NotificationType::Warning);
                        return;
                    }

                    set_webrtc_connecting();
                    show_notification("Connecting to Telnyx...", NotificationType::Info);

                    let result = init_telnyx_webrtc(&config.sip_username, &config.sip_password).await;
                    if result.is_truthy() {
                        set_webrtc_connected();
                        show_notification("Phone ready!", NotificationType::Success);
                    } else {
                        set_webrtc_error("Failed to connect to Telnyx".to_string());
                        show_notification("Failed to connect to phone service", NotificationType::Error);
                    }
                }
                Err(e) => {
                    set_webrtc_error(format!("Failed to fetch config: {}", e));
                    show_notification("Failed to fetch WebRTC config", NotificationType::Error);
                }
            }
        });
    });

    // Listen for call state updates from JavaScript
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        use wasm_bindgen::closure::Closure;

        let callback = Closure::wrap(Box::new(move |event: web_sys::CustomEvent| {
            let detail = event.detail();
            if detail.is_object() {
                if let Some(state) = Reflect::get(&detail, &"state".into())
                    .ok()
                    .and_then(|v| v.as_string())
                {
                    tracing::info!("WebRTC call state: {}", state);
                    set_webrtc_call_state(state.clone());

                    match state.as_str() {
                        "active" => {
                            show_notification("Call connected!", NotificationType::Success);
                        }
                        "hangup" | "destroy" => {
                            show_notification("Call ended", NotificationType::Info);
                            clear_webrtc_call();
                        }
                        _ => {}
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        if let Some(win) = web_sys::window() {
            let _ = win.add_event_listener_with_callback(
                "telnyxCallUpdate",
                callback.as_ref().unchecked_ref(),
            );
        }

        callback.forget(); // Keep the closure alive
    });

    let mut append_digit = move |digit: &str| {
        play_dtmf_tone(digit);

        // Also send DTMF if in call
        #[cfg(target_arch = "wasm32")]
        if WEBRTC_STATE.read().is_in_call {
            super::webrtc::send_dtmf(digit);
        }

        phone_number.write().push_str(digit);
    };

    let clear_number = move |_| {
        phone_number.set(String::new());
    };

    let backspace = move |_| {
        let mut current = phone_number.write();
        if !current.is_empty() {
            current.pop();
        }
    };

    let make_call = move |_| {
        let number = phone_number();
        if number.is_empty() {
            show_notification("Please enter a phone number", NotificationType::Warning);
        } else {
            #[cfg(target_arch = "wasm32")]
            {
                let formatted_number = format_e164(&number);
                let cid = caller_id();

                spawn(async move {
                    if !is_telnyx_ready() {
                        show_notification("Phone not ready. Please wait...", NotificationType::Warning);
                        return;
                    }

                    show_notification(&format!("Calling {}...", formatted_number), NotificationType::Info);
                    set_webrtc_call_state("trying".to_string());

                    let call_result = make_webrtc_call(&formatted_number, &cid).await;
                    if call_result.is_object() {
                        tracing::info!("WebRTC call initiated");
                    } else {
                        show_notification("Failed to make call", NotificationType::Error);
                        clear_webrtc_call();
                    }
                });
            }
        }
    };

    let hangup = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            hangup_webrtc_call();
            clear_webrtc_call();
            show_notification("Call ended", NotificationType::Info);
        }
    };

    let toggle_mute = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let muted = toggle_mute_webrtc();
            if muted {
                show_notification("Muted", NotificationType::Info);
            } else {
                show_notification("Unmuted", NotificationType::Info);
            }
        }
    };

    // Determine UI state
    let is_connected = webrtc_state.is_connected;
    let is_connecting = webrtc_state.is_connecting;
    let is_in_call = webrtc_state.is_in_call;
    let call_state = webrtc_state.call_state.clone().unwrap_or_default();

    rsx! {
        div { class: "bg-gray-50 rounded-lg p-3 w-full",
            // Connection status
            div { class: "mb-2 text-center text-sm",
                if is_connecting {
                    span { class: "text-yellow-600 animate-pulse", "Connecting..." }
                } else if is_connected {
                    span { class: "text-green-600", "Phone Ready" }
                } else {
                    span { class: "text-red-600", "Disconnected" }
                }
            }

            // Display
            div { class: "bg-white rounded-lg p-3 mb-3 text-center border",
                input {
                    class: "text-xl font-mono w-full text-center bg-transparent outline-none",
                    r#type: "tel",
                    value: "{phone_number}",
                    placeholder: "Enter number",
                    oninput: move |e| phone_number.set(e.value()),
                    disabled: is_in_call,
                }
                if is_in_call {
                    div { class: "text-sm text-blue-600 mt-1 animate-pulse",
                        match call_state.as_str() {
                            "trying" | "new" => "Dialing...",
                            "ringing" => "Ringing...",
                            "active" => "Connected",
                            "held" => "On Hold",
                            _ => &call_state,
                        }
                    }
                }
            }

            // Dialpad
            div { class: "grid grid-cols-3 gap-1 mb-2",
                DialButton { digit: "1", letters: "", on_click: move |_| append_digit("1") }
                DialButton { digit: "2", letters: "ABC", on_click: move |_| append_digit("2") }
                DialButton { digit: "3", letters: "DEF", on_click: move |_| append_digit("3") }
                DialButton { digit: "4", letters: "GHI", on_click: move |_| append_digit("4") }
                DialButton { digit: "5", letters: "JKL", on_click: move |_| append_digit("5") }
                DialButton { digit: "6", letters: "MNO", on_click: move |_| append_digit("6") }
                DialButton { digit: "7", letters: "PQRS", on_click: move |_| append_digit("7") }
                DialButton { digit: "8", letters: "TUV", on_click: move |_| append_digit("8") }
                DialButton { digit: "9", letters: "WXYZ", on_click: move |_| append_digit("9") }
                DialButton { digit: "*", letters: "", on_click: move |_| append_digit("*") }
                DialButton { digit: "0", letters: "+", on_click: move |_| append_digit("0") }
                DialButton { digit: "#", letters: "", on_click: move |_| append_digit("#") }
            }

            // Action buttons
            div { class: "flex justify-center gap-2",
                if is_in_call {
                    // In-call controls
                    button {
                        class: "bg-blue-500 hover:bg-blue-600 text-white rounded-full w-10 h-10 flex items-center justify-center transition-colors",
                        onclick: toggle_mute,
                        title: "Mute",
                        "\u{1F507}"
                    }
                    button {
                        class: "bg-red-500 hover:bg-red-600 text-white rounded-full w-12 h-12 flex items-center justify-center transition-colors",
                        onclick: hangup,
                        title: "Hang Up",
                        "\u{260E}"
                    }
                } else {
                    // Pre-call controls
                    button {
                        class: "bg-gray-400 hover:bg-gray-500 text-white rounded-full w-10 h-10 flex items-center justify-center transition-colors text-sm",
                        onclick: clear_number,
                        title: "Clear",
                        "C"
                    }
                    button {
                        class: "bg-green-500 hover:bg-green-600 text-white rounded-full w-12 h-12 flex items-center justify-center transition-colors disabled:opacity-50",
                        disabled: phone_number().is_empty() || !is_connected,
                        onclick: make_call,
                        title: "Call",
                        span { class: "text-xl", "\u{1F4DE}" }
                    }
                    button {
                        class: "bg-gray-400 hover:bg-gray-500 text-white rounded-full w-10 h-10 flex items-center justify-center transition-colors text-sm",
                        onclick: backspace,
                        title: "Backspace",
                        "\u{232B}"
                    }
                }
            }
        }
    }
}

#[component]
fn DialButton(digit: &'static str, letters: &'static str, on_click: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "bg-white hover:bg-gray-100 border rounded-lg w-full h-12 flex flex-col items-center justify-center transition-colors",
            onclick: move |e| on_click.call(e),
            span { class: "text-lg font-semibold", "{digit}" }
            if !letters.is_empty() {
                span { class: "text-xs text-gray-400 leading-none", "{letters}" }
            }
        }
    }
}
