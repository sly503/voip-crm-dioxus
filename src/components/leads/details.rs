use dioxus::prelude::*;
use crate::models::{Lead, LeadStatus, Call, CallRecording};
use crate::models::recording::RecordingSearchParams;
use crate::api;
use crate::state::UI_STATE;
use crate::components::common::LoadingSpinner;
use crate::components::recordings::AudioPlayer;

#[component]
pub fn LeadDetails() -> Element {
    let selected_id = UI_STATE.read().selected_lead_id;

    if selected_id.is_none() {
        return rsx! {
            div { class: "flex items-center justify-center h-full text-gray-500",
                "Select a lead to view details"
            }
        };
    }

    let lead_id = selected_id.unwrap();
    let mut lead = use_signal(|| None::<Lead>);
    let mut is_loading = use_signal(|| true);
    let mut new_note = use_signal(String::new);
    let mut is_adding_note = use_signal(|| false);
    let mut calls = use_signal(|| Vec::<Call>::new());
    let mut recordings = use_signal(|| Vec::<CallRecording>::new());
    let mut selected_recording_id = use_signal(|| None::<i64>);
    let mut show_player_modal = use_signal(|| false);

    // Fetch lead details, calls, and recordings
    use_effect(move || {
        spawn(async move {
            is_loading.set(true);

            // Fetch lead
            if let Ok(data) = api::leads::get_lead(lead_id).await {
                lead.set(Some(data));
            }

            // Fetch calls for this lead
            if let Ok(call_data) = api::calls::get_lead_calls(lead_id).await {
                calls.set(call_data);
            }

            // Fetch recordings for this lead
            let search_params = RecordingSearchParams {
                lead_id: Some(lead_id),
                ..Default::default()
            };
            if let Ok(recording_data) = api::recordings::search_recordings(search_params).await {
                recordings.set(recording_data);
            }

            is_loading.set(false);
        });
    });

    let add_note = move |_| {
        let content = new_note();
        if content.is_empty() {
            return;
        }

        is_adding_note.set(true);
        spawn(async move {
            if api::leads::add_note(lead_id, &content).await.is_ok() {
                new_note.set(String::new());
                // Refresh lead
                if let Ok(data) = api::leads::get_lead(lead_id).await {
                    lead.set(Some(data));
                }
            }
            is_adding_note.set(false);
        });
    };

    let close_details = move |_| {
        crate::state::select_lead(None);
    };

    if *is_loading.read() {
        return rsx! { LoadingSpinner {} };
    }

    let lead_data = match lead.read().as_ref() {
        Some(l) => l.clone(),
        None => return rsx! {
            div { class: "text-center text-red-500 p-4",
                "Lead not found"
            }
        },
    };

    // Format timestamps outside of rsx
    let created_at_str = lead_data.created_at
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default();
    let last_call_str = lead_data.last_call_at
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string());

    rsx! {
        div { class: "h-full flex flex-col bg-white",
            // Header
            div { class: "flex items-center justify-between p-4 border-b",
                h2 { class: "text-xl font-semibold", "{lead_data.full_name()}" }
                button {
                    class: "text-gray-500 hover:text-gray-700",
                    onclick: close_details,
                    "\u{2715}"
                }
            }

            // Content
            div { class: "flex-1 overflow-y-auto p-4",
                // Status badge
                div { class: "mb-4",
                    span { class: "px-3 py-1 rounded-full text-sm {lead_data.status.color_class()}",
                        "{lead_data.status.display_name()}"
                    }
                }

                // Contact info
                div { class: "bg-gray-50 rounded-lg p-4 mb-4",
                    h3 { class: "font-medium mb-3", "Contact Information" }
                    div { class: "space-y-2",
                        div { class: "flex items-center gap-2",
                            span { class: "text-gray-400 w-6", "\u{1F4DE}" }
                            span { "{lead_data.phone}" }
                        }
                        if let Some(email) = &lead_data.email {
                            div { class: "flex items-center gap-2",
                                span { class: "text-gray-400 w-6", "\u{2709}" }
                                span { "{email}" }
                            }
                        }
                        if let Some(company) = &lead_data.company {
                            div { class: "flex items-center gap-2",
                                span { class: "text-gray-400 w-6", "\u{1F3E2}" }
                                span { "{company}" }
                            }
                        }
                    }
                }

                // Call history
                div { class: "bg-gray-50 rounded-lg p-4 mb-4",
                    h3 { class: "font-medium mb-3", "Call History ({calls.read().len()})" }
                    if calls.read().is_empty() {
                        p { class: "text-gray-500 text-sm text-center py-4",
                            "No calls yet"
                        }
                    } else {
                        div { class: "space-y-2",
                            for call in calls.read().iter() {
                                CallHistoryRow {
                                    key: "{call.id}",
                                    call: call.clone(),
                                    recording: recordings.read().iter().find(|r| r.call_id == call.id).cloned(),
                                    on_play: move |recording_id| {
                                        selected_recording_id.set(Some(recording_id));
                                        show_player_modal.set(true);
                                    }
                                }
                            }
                        }
                    }
                }

                // Recording player modal
                if *show_player_modal.read() {
                    if let Some(recording_id) = *selected_recording_id.read() {
                        div {
                            class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
                            onclick: move |_| show_player_modal.set(false),
                            div {
                                class: "bg-white rounded-lg p-6 max-w-2xl w-full mx-4",
                                onclick: move |e| e.stop_propagation(),
                                div { class: "flex items-center justify-between mb-4",
                                    h3 { class: "text-lg font-semibold", "Recording Playback" }
                                    button {
                                        class: "text-gray-500 hover:text-gray-700 text-2xl",
                                        onclick: move |_| show_player_modal.set(false),
                                        "\u{2715}"
                                    }
                                }
                                AudioPlayer {
                                    recording_id: recording_id
                                }
                            }
                        }
                    }
                }

                // Notes
                div { class: "mb-4",
                    h3 { class: "font-medium mb-3", "Notes" }

                    // Add note form
                    div { class: "flex gap-2 mb-4",
                        textarea {
                            class: "flex-1 px-3 py-2 border border-gray-300 rounded-lg resize-none",
                            rows: "2",
                            placeholder: "Add a note...",
                            value: "{new_note}",
                            oninput: move |e| new_note.set(e.value()),
                        }
                        button {
                            class: "px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50",
                            disabled: new_note().is_empty() || *is_adding_note.read(),
                            onclick: add_note,
                            if *is_adding_note.read() { "..." } else { "Add" }
                        }
                    }

                    // Notes display
                    if let Some(notes) = &lead_data.notes {
                        if !notes.is_empty() {
                            div { class: "bg-gray-50 rounded-lg p-3",
                                p { class: "text-sm whitespace-pre-wrap", "{notes}" }
                            }
                        } else {
                            p { class: "text-gray-500 text-sm", "No notes yet" }
                        }
                    } else {
                        p { class: "text-gray-500 text-sm", "No notes yet" }
                    }
                }

                // Status update
                div {
                    h3 { class: "font-medium mb-3", "Update Status" }
                    div { class: "flex flex-wrap gap-2",
                        StatusButton { lead_id: lead_id, status: LeadStatus::Contacted, current: lead_data.status }
                        StatusButton { lead_id: lead_id, status: LeadStatus::Qualified, current: lead_data.status }
                        StatusButton { lead_id: lead_id, status: LeadStatus::Converted, current: lead_data.status }
                        StatusButton { lead_id: lead_id, status: LeadStatus::Lost, current: lead_data.status }
                        StatusButton { lead_id: lead_id, status: LeadStatus::DoNotCall, current: lead_data.status }
                    }
                }
            }

            // Actions
            div { class: "p-4 border-t",
                button {
                    class: "w-full py-3 bg-green-500 hover:bg-green-600 text-white rounded-lg font-medium",
                    "\u{1F4DE} Call Now"
                }
            }
        }
    }
}

#[component]
fn StatusButton(lead_id: i64, status: LeadStatus, current: LeadStatus) -> Element {
    let is_current = status == current;
    let mut is_updating = use_signal(|| false);

    let update_status = move |_| {
        if is_current {
            return;
        }

        is_updating.set(true);
        spawn(async move {
            let request = crate::models::UpdateStatusRequest { status };
            if let Err(e) = api::leads::update_status(lead_id, request).await {
                tracing::error!("Failed to update status: {}", e);
            }
            is_updating.set(false);
        });
    };

    rsx! {
        button {
            class: "px-3 py-1 rounded-full text-sm transition-colors",
            class: if is_current { "bg-blue-600 text-white" } else { "bg-gray-100 hover:bg-gray-200" },
            disabled: is_current || *is_updating.read(),
            onclick: update_status,
            "{status.display_name()}"
        }
    }
}

#[component]
fn CallHistoryRow(
    call: Call,
    recording: Option<CallRecording>,
    on_play: EventHandler<i64>,
) -> Element {
    // Format duration
    let duration_str = if let Some(duration) = call.duration_seconds {
        let mins = duration / 60;
        let secs = duration % 60;
        format!("{}:{:02}", mins, secs)
    } else {
        "-".to_string()
    };

    // Format date
    let date_str = call.started_at
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "-".to_string());

    // Get status badge color
    let status_color = match call.status {
        crate::models::CallStatus::Completed => "bg-green-100 text-green-800",
        crate::models::CallStatus::NoAnswer | crate::models::CallStatus::Busy => "bg-yellow-100 text-yellow-800",
        crate::models::CallStatus::Failed => "bg-red-100 text-red-800",
        _ => "bg-blue-100 text-blue-800",
    };

    rsx! {
        div { class: "bg-white rounded border p-3 hover:shadow-sm transition-shadow",
            div { class: "flex items-center justify-between",
                div { class: "flex-1 space-y-1",
                    div { class: "flex items-center gap-2",
                        span { class: "text-xs px-2 py-0.5 rounded-full {status_color}",
                            "{call.status.display_name()}"
                        }
                        if let Some(disposition) = &call.disposition {
                            span { class: "text-xs text-gray-500",
                                " • {disposition}"
                            }
                        }
                    }
                    div { class: "text-sm text-gray-600",
                        span { "\u{1F4C5} {date_str}" }
                        span { class: "mx-2", "•" }
                        span { "\u{23F1} {duration_str}" }
                    }
                }

                // Recording play button
                if let Some(rec) = recording {
                    button {
                        class: "flex items-center gap-1 px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm transition-colors",
                        onclick: move |_| on_play.call(rec.id),
                        title: "Play recording",
                        span { class: "text-base", "\u{25B6}" }
                        span { "Play" }
                    }
                } else {
                    span { class: "text-xs text-gray-400 px-3",
                        "No recording"
                    }
                }
            }
        }
    }
}
