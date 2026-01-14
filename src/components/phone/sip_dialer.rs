//! SIP trunk-based phone dialer
//!
//! This component provides server-side SIP calling using DIDLogic or similar SIP trunks.

use dioxus::prelude::*;
use crate::state::{NotificationType, show_notification};

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

// Desktop with audio support (requires alsa-lib-devel)
#[cfg(all(not(target_arch = "wasm32"), feature = "tinyaudio"))]
fn play_dtmf_tone(digit: &str) {
    use tinyaudio::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let (low_freq, high_freq) = match digit {
        "1" => (697.0_f32, 1209.0_f32),
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

    // Spawn audio in a separate thread to not block UI
    std::thread::spawn(move || {
        let sample_rate = 44100_u32;
        let duration_samples = (sample_rate as f32 * 0.15) as usize; // 150ms
        let sample_counter = Arc::new(AtomicUsize::new(0));
        let counter = sample_counter.clone();

        let params = OutputDeviceParameters {
            channels_count: 1,
            sample_rate: sample_rate as usize,
            channel_sample_count: 1024,
        };

        if let Ok(device) = run_output_device(params, move |data| {
            let current = counter.fetch_add(data.len(), Ordering::Relaxed);
            for (i, sample) in data.iter_mut().enumerate() {
                let idx = current + i;
                if idx < duration_samples {
                    let t = idx as f32 / sample_rate as f32;
                    let low = (2.0 * std::f32::consts::PI * low_freq * t).sin();
                    let high = (2.0 * std::f32::consts::PI * high_freq * t).sin();
                    *sample = (low + high) * 0.15; // Combine with reduced volume
                } else {
                    *sample = 0.0;
                }
            }
        }) {
            // Keep thread alive for the duration of the sound
            std::thread::sleep(std::time::Duration::from_millis(200));
            drop(device);
        }
    });
}

// Desktop without audio (no-op)
#[cfg(all(not(target_arch = "wasm32"), not(feature = "tinyaudio")))]
fn play_dtmf_tone(_digit: &str) {
    // Audio disabled - install alsa-lib-devel and build with --features desktop-audio
}

/// Format phone number to E.164
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
pub fn SipDialer() -> Element {
    let mut phone_number = use_signal(String::new);
    let mut sip_status = use_signal(|| "checking".to_string());
    let mut is_registered = use_signal(|| false);
    let mut is_in_call = use_signal(|| false);
    let mut call_id = use_signal(|| None::<String>);
    let mut call_state = use_signal(|| "idle".to_string());

    // Fetch SIP status on mount and periodically
    use_effect(move || {
        spawn(async move {
            // Initial check
            check_sip_status(sip_status, is_registered).await;

            // Periodic status check every 5 seconds
            loop {
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(5000).await;

                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                check_sip_status(sip_status, is_registered).await;
            }
        });
    });

    async fn check_sip_status(
        mut status: Signal<String>,
        mut registered: Signal<bool>,
    ) {
        match crate::api::sip::get_sip_status().await {
            Ok(sip) => {
                status.set(sip.status.clone());
                registered.set(sip.registered);
            }
            Err(_) => {
                status.set("error".to_string());
                registered.set(false);
            }
        }
    }

    let append_digit = move |digit: &'static str| {
        move |_| {
            play_dtmf_tone(digit);
            phone_number.write().push_str(digit);
        }
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
            return;
        }

        let formatted = format_e164(&number);
        spawn(async move {
            call_state.set("dialing".to_string());
            is_in_call.set(true);
            show_notification(&format!("Calling {}...", formatted), NotificationType::Info);

            match crate::api::sip::sip_dial(&formatted).await {
                Ok(resp) => {
                    if resp.success {
                        call_id.set(resp.call_id);
                        call_state.set("ringing".to_string());
                        show_notification("Call initiated!", NotificationType::Success);
                    } else {
                        let err = resp.error.unwrap_or_else(|| "Unknown error".to_string());
                        show_notification(&format!("Call failed: {}", err), NotificationType::Error);
                        is_in_call.set(false);
                        call_state.set("idle".to_string());
                    }
                }
                Err(e) => {
                    show_notification(&format!("Call error: {}", e), NotificationType::Error);
                    is_in_call.set(false);
                    call_state.set("idle".to_string());
                }
            }
        });
    };

    let hangup = move |_| {
        let cid = call_id();
        spawn(async move {
            if let Some(id) = cid {
                match crate::api::sip::sip_hangup(&id).await {
                    Ok(_) => {
                        show_notification("Call ended", NotificationType::Info);
                    }
                    Err(e) => {
                        show_notification(&format!("Hangup error: {}", e), NotificationType::Error);
                    }
                }
            }
            is_in_call.set(false);
            call_id.set(None);
            call_state.set("idle".to_string());
        });
    };

    // UI states
    let status = sip_status();
    let registered = is_registered();
    let in_call = is_in_call();
    let current_call_state = call_state();

    let status_display = match status.as_str() {
        "registered" => ("Phone Ready", "text-green-600", false),
        "registering" | "connecting" => ("Connecting...", "text-yellow-600 animate-pulse", true),
        "failed" => ("Connection Failed", "text-red-600", false),
        "not_configured" => ("Not Configured", "text-orange-600", false),
        "checking" => ("Checking...", "text-gray-500 animate-pulse", true),
        "error" => ("Server Error", "text-red-600", false),
        _ => ("Disconnected", "text-red-600", false),
    };

    rsx! {
        div { class: "bg-gray-50 rounded-lg p-3 w-full",
            // Connection status
            div { class: "mb-2 text-center text-sm",
                span { class: "{status_display.1}", "{status_display.0}" }
            }

            // Display
            div { class: "bg-white rounded-lg p-3 mb-3 text-center border",
                input {
                    class: "text-xl font-mono w-full text-center bg-transparent outline-none",
                    r#type: "tel",
                    value: "{phone_number}",
                    placeholder: "Enter number",
                    oninput: move |e| phone_number.set(e.value()),
                    disabled: in_call,
                }
                if in_call {
                    div { class: "text-sm text-blue-600 mt-1 animate-pulse",
                        match current_call_state.as_str() {
                            "dialing" => "Dialing...",
                            "ringing" => "Ringing...",
                            "active" => "Connected",
                            "held" => "On Hold",
                            _ => &current_call_state,
                        }
                    }
                }
            }

            // Dialpad
            div { class: "grid grid-cols-3 gap-1 mb-2",
                DialButton { digit: "1", letters: "", on_click: append_digit("1") }
                DialButton { digit: "2", letters: "ABC", on_click: append_digit("2") }
                DialButton { digit: "3", letters: "DEF", on_click: append_digit("3") }
                DialButton { digit: "4", letters: "GHI", on_click: append_digit("4") }
                DialButton { digit: "5", letters: "JKL", on_click: append_digit("5") }
                DialButton { digit: "6", letters: "MNO", on_click: append_digit("6") }
                DialButton { digit: "7", letters: "PQRS", on_click: append_digit("7") }
                DialButton { digit: "8", letters: "TUV", on_click: append_digit("8") }
                DialButton { digit: "9", letters: "WXYZ", on_click: append_digit("9") }
                DialButton { digit: "*", letters: "", on_click: append_digit("*") }
                DialButton { digit: "0", letters: "+", on_click: append_digit("0") }
                DialButton { digit: "#", letters: "", on_click: append_digit("#") }
            }

            // Action buttons
            div { class: "flex justify-center gap-2",
                if in_call {
                    // In-call controls
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
                        disabled: phone_number().is_empty() || !registered,
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
