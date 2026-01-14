use dioxus::prelude::*;
use crate::state::{CALL_STATE, NotificationType, show_notification};
#[cfg(target_arch = "wasm32")]
use crate::state::{set_ringing, set_answered, end_call};
use crate::api;
use crate::models::Lead;

/// Play DTMF tone for a digit using Web Audio API
#[cfg(target_arch = "wasm32")]
fn play_dtmf_tone(digit: &str) {
    use web_sys::{AudioContext, OscillatorType};

    // DTMF frequencies (low, high)
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

    // Create audio context and oscillators
    if let Ok(ctx) = AudioContext::new() {
        let duration = 0.15; // 150ms tone
        let current_time = ctx.current_time();

        // Create gain node for volume control
        if let Ok(gain) = ctx.create_gain() {
            let _ = gain.gain().set_value(0.1); // Low volume
            let _ = gain.connect_with_audio_node(&ctx.destination());

            // Low frequency oscillator
            if let Ok(osc1) = ctx.create_oscillator() {
                osc1.set_type(OscillatorType::Sine);
                let _ = osc1.frequency().set_value(low_freq as f32);
                let _ = osc1.connect_with_audio_node(&gain);
                let _ = osc1.start();
                let _ = osc1.stop_with_when(current_time + duration);
            }

            // High frequency oscillator
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


/// Play a single ring tone burst (used repeatedly for ringing)
#[cfg(target_arch = "wasm32")]
fn play_ring_tone() {
    use web_sys::{AudioContext, OscillatorType};

    // US ring tone: 440Hz + 480Hz, on for 2s, off for 4s
    // We'll play a shorter burst: 400ms on
    if let Ok(ctx) = AudioContext::new() {
        let current_time = ctx.current_time();

        if let Ok(gain) = ctx.create_gain() {
            let _ = gain.gain().set_value(0.15);
            let _ = gain.connect_with_audio_node(&ctx.destination());

            // 440Hz tone
            if let Ok(osc1) = ctx.create_oscillator() {
                osc1.set_type(OscillatorType::Sine);
                let _ = osc1.frequency().set_value(440.0);
                let _ = osc1.connect_with_audio_node(&gain);
                let _ = osc1.start();
                let _ = osc1.stop_with_when(current_time + 0.4);
            }

            // 480Hz tone
            if let Ok(osc2) = ctx.create_oscillator() {
                osc2.set_type(OscillatorType::Sine);
                let _ = osc2.frequency().set_value(480.0);
                let _ = osc2.connect_with_audio_node(&gain);
                let _ = osc2.start();
                let _ = osc2.stop_with_when(current_time + 0.4);
            }
        }
    }
}

/// Format phone number to E.164 format (+1XXXXXXXXXX for US)
#[cfg(target_arch = "wasm32")]
fn format_e164(number: &str) -> String {
    // Remove all non-digit characters except +
    let cleaned: String = number.chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect();

    // If already starts with +, assume it's formatted
    if cleaned.starts_with('+') {
        return cleaned;
    }

    // If 10 digits, assume US number and add +1
    if cleaned.len() == 10 {
        return format!("+1{}", cleaned);
    }

    // If 11 digits starting with 1, add +
    if cleaned.len() == 11 && cleaned.starts_with('1') {
        return format!("+{}", cleaned);
    }

    // Otherwise just add + and hope for the best
    format!("+{}", cleaned)
}

/// Poll call status until answered, completed, or failed
#[cfg(target_arch = "wasm32")]
async fn poll_call_status(call_id: i64) {
    use gloo_timers::future::TimeoutFuture;
    use crate::models::CallStatus;

    let mut ring_count = 0;
    let max_rings = 30; // ~60 seconds of ringing (2 sec per ring cycle)

    loop {
        // Check if we're still in a call
        let state = CALL_STATE.read();
        if state.current_call_id != Some(call_id) {
            // Call was ended or changed
            break;
        }
        drop(state);

        // Play ring tone if still ringing
        if CALL_STATE.read().is_ringing {
            play_ring_tone();
        }

        // Poll for call status
        match api::calls::get_call_status(call_id).await {
            Ok(call) => {
                tracing::info!("Call status: {:?}", call.status);

                match call.status {
                    CallStatus::Answered | CallStatus::Bridged => {
                        set_answered();
                        show_notification("Call connected!", NotificationType::Success);
                        // Start duration timer
                        spawn(async move {
                            run_call_timer(call_id).await;
                        });
                        break;
                    }
                    CallStatus::Completed => {
                        show_notification("Call ended", NotificationType::Info);
                        end_call();
                        break;
                    }
                    CallStatus::Failed => {
                        show_notification("Call failed", NotificationType::Error);
                        end_call();
                        break;
                    }
                    CallStatus::Busy => {
                        show_notification("Line busy", NotificationType::Warning);
                        end_call();
                        break;
                    }
                    CallStatus::NoAnswer => {
                        show_notification("No answer", NotificationType::Warning);
                        end_call();
                        break;
                    }
                    _ => {
                        // Still ringing or initiating, continue polling
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to poll call status: {}", e);
            }
        }

        ring_count += 1;
        if ring_count >= max_rings {
            show_notification("No answer - call timed out", NotificationType::Warning);
            end_call();
            break;
        }

        // Wait 2 seconds before next poll (matches ring cadence)
        TimeoutFuture::new(2000).await;
    }
}

/// Run the call duration timer
#[cfg(target_arch = "wasm32")]
async fn run_call_timer(call_id: i64) {
    use gloo_timers::future::TimeoutFuture;
    use crate::state::increment_duration;

    loop {
        // Check if still in this call
        let state = CALL_STATE.read();
        if state.current_call_id != Some(call_id) || !state.is_answered {
            break;
        }
        drop(state);

        // Wait 1 second
        TimeoutFuture::new(1000).await;

        // Increment duration
        increment_duration();
    }
}

#[component]
pub fn PhoneDialer() -> Element {
    let mut phone_number = use_signal(String::new);
    let call_state = CALL_STATE.read();

    let mut append_digit = move |digit: &str| {
        #[cfg(target_arch = "wasm32")]
        play_dtmf_tone(digit);
        let digit = digit.to_string();
        phone_number.write().push_str(&digit);
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

        #[cfg(target_arch = "wasm32")]
        {
            // Format to E.164 (add +1 for US numbers if missing)
            let formatted_number = format_e164(&number);

            // Dial the number directly
            spawn(async move {
                // Set dialing state immediately
                CALL_STATE.write().is_dialing = true;
                show_notification(&format!("Calling {}...", formatted_number), NotificationType::Info);

                match api::calls::dial_direct(&formatted_number, None).await {
                    Ok(response) => {
                        tracing::info!("Call initiated: {:?}", response);
                        // Set ringing state and store call ID for polling
                        set_ringing(response.call_id, formatted_number.clone());
                        show_notification("Ringing...", NotificationType::Info);

                        // Start polling for call status
                        let call_id = response.call_id;
                        spawn(async move {
                            poll_call_status(call_id).await;
                        });
                    }
                    Err(e) => {
                        CALL_STATE.write().is_dialing = false;
                        show_notification(&format!("Call failed: {}", e), NotificationType::Error);
                        tracing::error!("Failed to dial: {}", e);
                    }
                }
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = number; // suppress unused warning
            show_notification("Calling is only available in the browser", NotificationType::Warning);
        }
    };

    rsx! {
        div { class: "bg-gray-50 rounded-lg p-3 w-full",
            // Display
            div { class: "bg-white rounded-lg p-3 mb-3 text-center border",
                input {
                    class: "text-xl font-mono w-full text-center bg-transparent outline-none",
                    r#type: "tel",
                    value: "{phone_number}",
                    placeholder: "Enter number",
                    oninput: move |e| phone_number.set(e.value()),
                }
            }

            // Dialpad - compact layout
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

            // Action buttons - compact
            div { class: "flex justify-center gap-2",
                // Clear button
                button {
                    class: "bg-gray-400 hover:bg-gray-500 text-white rounded-full w-10 h-10 flex items-center justify-center transition-colors text-sm",
                    onclick: clear_number,
                    title: "Clear",
                    "C"
                }

                // Call button
                button {
                    class: "bg-green-500 hover:bg-green-600 text-white rounded-full w-12 h-12 flex items-center justify-center transition-colors disabled:opacity-50",
                    disabled: phone_number().is_empty() || call_state.is_in_call(),
                    onclick: make_call,
                    title: "Call",
                    span { class: "text-xl", "\u{1F4DE}" }
                }

                // Backspace button
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

#[component]
pub fn QuickDial(lead: Lead, agent_id: i64) -> Element {
    let mut is_dialing = use_signal(|| false);
    let lead_clone = lead.clone();
    let lead_id = lead.id;
    let lead_name = lead.full_name();
    let phone = lead.phone.clone();

    let dial_lead = move |_| {
        let lead_for_call = lead_clone.clone();
        is_dialing.set(true);

        spawn(async move {
            match api::calls::dial(lead_id, agent_id).await {
                Ok(response) => {
                    tracing::info!("Call initiated: {:?}", response);
                    show_notification(
                        &format!("Calling {}...", lead_for_call.full_name()),
                        NotificationType::Success
                    );
                    // Update call state with the new call
                    crate::state::CALL_STATE.write().is_dialing = true;
                }
                Err(e) => {
                    show_notification(
                        &format!("Failed to dial: {}", e),
                        NotificationType::Error
                    );
                    tracing::error!("Failed to dial: {}", e);
                }
            }
            is_dialing.set(false);
        });
    };

    rsx! {
        button {
            class: "flex items-center gap-2 px-3 py-2 bg-green-500 hover:bg-green-600 text-white rounded-lg transition-colors disabled:opacity-50",
            disabled: *is_dialing.read(),
            onclick: dial_lead,
            title: "Call {lead_name}",
            if *is_dialing.read() {
                div { class: "animate-spin rounded-full h-4 w-4 border-b-2 border-white" }
            } else {
                span { "\u{1F4DE}" }
            }
            span { class: "text-sm", "{phone}" }
        }
    }
}
