use dioxus::prelude::*;
use crate::state::CALL_STATE;

#[component]
pub fn CallStatusBar() -> Element {
    let call_state = CALL_STATE.read();

    // Show bar if dialing, ringing, or answered
    if !call_state.is_dialing && !call_state.is_ringing && !call_state.is_answered && !call_state.is_in_call() {
        return rsx! {};
    }

    let is_dialing = call_state.is_dialing;
    let is_ringing = call_state.is_ringing;
    let is_answered = call_state.is_answered;

    let status_text = if is_dialing {
        "Initiating...".to_string()
    } else if is_ringing {
        "Ringing...".to_string()
    } else if is_answered {
        "Connected".to_string()
    } else {
        call_state.call_status()
            .map(|s| s.display_name().to_string())
            .unwrap_or_else(|| "In Call".to_string())
    };

    let duration = call_state.call_duration;
    let duration_text = format!("{:02}:{:02}", duration / 60, duration % 60);

    // Show lead name or "Direct Call" with the dialed number
    let lead_name = call_state.lead_name().unwrap_or_else(|| "Direct Call".to_string());
    let phone = call_state.phone_number()
        .map(|s| s.to_string())
        .or_else(|| call_state.dialed_number.clone())
        .unwrap_or_default();

    // Yellow for dialing/ringing, green for connected
    let bar_color = if is_dialing || is_ringing { "bg-yellow-500" } else { "bg-green-600" };

    rsx! {
        div { class: "fixed bottom-0 left-0 right-0 {bar_color} text-white p-4 shadow-lg z-50",
            div { class: "container mx-auto flex items-center justify-between",
                // Call info
                div { class: "flex items-center gap-4",
                    // Pulse indicator
                    div { class: "w-3 h-3 bg-white rounded-full animate-pulse" }

                    div {
                        div { class: "font-semibold", "{lead_name}" }
                        div { class: "text-sm opacity-90", "{phone}" }
                    }
                }

                // Status and duration
                div { class: "text-center",
                    if is_dialing || is_ringing {
                        // Show pulsing text when dialing/ringing
                        div { class: "text-lg font-mono animate-pulse", "{status_text}" }
                    } else {
                        div { class: "text-lg font-mono", "{duration_text}" }
                    }
                    if !is_dialing && !is_ringing {
                        div { class: "text-sm opacity-90", "{status_text}" }
                    }
                }

                // Quick actions
                div { class: "flex items-center gap-2",
                    // Mute button
                    button {
                        class: "p-2 rounded-full hover:bg-green-700 transition-colors",
                        class: if call_state.is_muted { "bg-red-500" } else { "" },
                        onclick: move |_| crate::state::toggle_mute(),
                        title: if call_state.is_muted { "Unmute" } else { "Mute" },
                        if call_state.is_muted { "\u{1F507}" } else { "\u{1F50A}" }
                    }

                    // Hold button
                    button {
                        class: "p-2 rounded-full hover:bg-green-700 transition-colors",
                        class: if call_state.is_on_hold { "bg-yellow-500" } else { "" },
                        onclick: move |_| crate::state::toggle_hold(),
                        title: if call_state.is_on_hold { "Resume" } else { "Hold" },
                        if call_state.is_on_hold { "\u{25B6}" } else { "\u{23F8}" }
                    }

                    // End call button
                    button {
                        class: "p-2 bg-red-500 hover:bg-red-600 rounded-full transition-colors",
                        onclick: move |_| {
                            crate::state::end_call();
                        },
                        title: "End Call",
                        "\u{260E}"
                    }
                }
            }
        }
    }
}
