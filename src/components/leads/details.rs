use dioxus::prelude::*;
use crate::models::{Lead, LeadStatus};
use crate::api;
use crate::state::UI_STATE;
use crate::components::common::LoadingSpinner;

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

    // Fetch lead details
    use_effect(move || {
        spawn(async move {
            is_loading.set(true);
            if let Ok(data) = api::leads::get_lead(lead_id).await {
                lead.set(Some(data));
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
                    h3 { class: "font-medium mb-3", "Call History" }
                    div { class: "space-y-2 text-sm",
                        div { class: "flex justify-between",
                            span { class: "text-gray-500", "Call Attempts:" }
                            span { "{lead_data.call_attempts}" }
                        }
                        if let Some(last_call) = &last_call_str {
                            div { class: "flex justify-between",
                                span { class: "text-gray-500", "Last Call:" }
                                span { "{last_call}" }
                            }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-gray-500", "Created:" }
                            span { "{created_at_str}" }
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
