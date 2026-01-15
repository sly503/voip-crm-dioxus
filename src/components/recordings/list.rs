use dioxus::prelude::*;
use crate::models::{CallRecording, RecordingSearchParams};
use crate::api;
use crate::components::common::LoadingSpinner;
use crate::state::{show_notification, NotificationType};

#[component]
pub fn RecordingList(
    #[props(default = RecordingSearchParams {
        agent_id: None,
        campaign_id: None,
        lead_id: None,
        start_date: None,
        end_date: None,
        disposition: None,
        compliance_hold: None,
        limit: Some(100),
        offset: Some(0),
    })]
    search_params: RecordingSearchParams,
) -> Element {
    let mut recordings = use_signal(Vec::<CallRecording>::new);
    let mut is_loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    // Fetch recordings on mount and when search_params change
    use_effect(move || {
        spawn(async move {
            is_loading.set(true);
            match api::recordings::search_recordings(search_params.clone()).await {
                Ok(data) => {
                    recordings.set(data);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    show_notification(&format!("Failed to load recordings: {}", e), NotificationType::Error);
                }
            }
            is_loading.set(false);
        });
    });

    rsx! {
        div { class: "h-full flex flex-col",
            // Header
            div { class: "flex items-center justify-between p-4 border-b",
                h2 { class: "text-xl font-semibold", "Call Recordings" }
                span { class: "text-sm text-gray-500",
                    "{recordings.read().len()} recordings"
                }
            }

            // Content
            div { class: "flex-1 overflow-y-auto",
                if *is_loading.read() {
                    div { class: "p-4",
                        LoadingSpinner {}
                    }
                } else if let Some(err) = error.read().as_ref() {
                    div { class: "text-center text-red-500 p-4",
                        "Error loading recordings: {err}"
                    }
                } else if recordings.read().is_empty() {
                    div { class: "text-center text-gray-500 p-8",
                        "No recordings found"
                    }
                } else {
                    div { class: "overflow-x-auto",
                        table { class: "w-full",
                            thead { class: "bg-gray-50 border-b",
                                tr {
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Date"
                                    }
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Agent"
                                    }
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Lead"
                                    }
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Campaign"
                                    }
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Duration"
                                    }
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Disposition"
                                    }
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Actions"
                                    }
                                }
                            }
                            tbody { class: "bg-white divide-y divide-gray-200",
                                for recording in recordings.read().iter() {
                                    RecordingRow {
                                        key: "{recording.id}",
                                        recording: recording.clone(),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RecordingRow(recording: CallRecording) -> Element {
    let recording_id = recording.id;
    let mut is_deleting = use_signal(|| false);

    // Extract metadata
    let metadata = recording.metadata.as_ref()
        .and_then(|m| serde_json::from_value::<crate::models::RecordingMetadata>(m.clone()).ok());

    let agent_name = metadata.as_ref()
        .and_then(|m| m.agent_name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let lead_name = metadata.as_ref()
        .and_then(|m| m.lead_name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let campaign_name = metadata.as_ref()
        .and_then(|m| m.campaign_name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let disposition = metadata.as_ref()
        .and_then(|m| m.disposition.clone())
        .unwrap_or_else(|| "-".to_string());

    // Format date
    let date_str = recording.uploaded_at.format("%Y-%m-%d %H:%M").to_string();

    // Format duration (seconds to mm:ss)
    let duration_str = format_duration(recording.duration_seconds);

    let handle_delete = move |_| {
        if recording.compliance_hold {
            show_notification("Cannot delete recording with compliance hold", NotificationType::Error);
            return;
        }

        is_deleting.set(true);
        spawn(async move {
            match api::recordings::delete_recording(recording_id).await {
                Ok(_) => {
                    show_notification("Recording deleted successfully", NotificationType::Success);
                    // Note: In a real app, you'd want to refresh the list here
                }
                Err(e) => {
                    show_notification(&format!("Failed to delete recording: {}", e), NotificationType::Error);
                }
            }
            is_deleting.set(false);
        });
    };

    rsx! {
        tr { class: "hover:bg-gray-50",
            td { class: "px-4 py-3 text-sm text-gray-900",
                "{date_str}"
            }
            td { class: "px-4 py-3 text-sm text-gray-900",
                "{agent_name}"
            }
            td { class: "px-4 py-3 text-sm text-gray-900",
                "{lead_name}"
            }
            td { class: "px-4 py-3 text-sm text-gray-900",
                "{campaign_name}"
            }
            td { class: "px-4 py-3 text-sm text-gray-900",
                "{duration_str}"
            }
            td { class: "px-4 py-3 text-sm",
                span {
                    class: "px-2 py-1 text-xs rounded-full",
                    class: disposition_color_class(&disposition),
                    "{disposition}"
                }
            }
            td { class: "px-4 py-3 text-sm",
                div { class: "flex gap-2",
                    // Play button
                    a {
                        href: "{api::recordings::get_stream_url(recording_id)}",
                        target: "_blank",
                        class: "text-blue-600 hover:text-blue-800",
                        title: "Play recording",
                        "\u{25B6}"
                    }
                    // Download button
                    a {
                        href: "{api::recordings::get_download_url(recording_id)}",
                        download: true,
                        class: "text-green-600 hover:text-green-800",
                        title: "Download recording",
                        "\u{2B07}"
                    }
                    // Delete button
                    if !recording.compliance_hold {
                        button {
                            class: "text-red-600 hover:text-red-800 disabled:opacity-50",
                            disabled: *is_deleting.read(),
                            onclick: handle_delete,
                            title: "Delete recording",
                            "\u{1F5D1}"
                        }
                    } else {
                        span {
                            class: "text-yellow-600",
                            title: "Compliance hold - cannot delete",
                            "\u{1F512}"
                        }
                    }
                }
            }
        }
    }
}

/// Format duration in seconds to mm:ss format
fn format_duration(seconds: i32) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{}:{:02}", mins, secs)
}

/// Get color class for disposition badge
fn disposition_color_class(disposition: &str) -> &'static str {
    match disposition.to_lowercase().as_str() {
        "connected" | "sale" | "callback" => "bg-green-100 text-green-800",
        "no answer" | "busy" | "voicemail" => "bg-yellow-100 text-yellow-800",
        "failed" | "rejected" | "do not call" => "bg-red-100 text-red-800",
        _ => "bg-gray-100 text-gray-800",
    }
}
