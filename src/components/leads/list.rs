use dioxus::prelude::*;
use crate::models::Lead;
use crate::api;
use crate::components::leads::LeadCard;
use crate::components::common::LoadingSpinner;
use crate::state::{AUTH_STATE, NotificationType, show_notification};

#[component]
pub fn LeadList() -> Element {
    let mut leads = use_signal(Vec::<Lead>::new);
    let mut is_loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut search_query = use_signal(String::new);

    let auth_state = AUTH_STATE.read();
    let agent_id = auth_state.user_id().unwrap_or(0);

    // Fetch leads on mount
    use_effect(move || {
        spawn(async move {
            is_loading.set(true);
            match api::leads::get_my_leads().await {
                Ok(data) => {
                    let is_empty = data.is_empty();
                    leads.set(data);
                    error.set(None);
                    if is_empty {
                        show_notification("No leads assigned to you yet", NotificationType::Info);
                    }
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    show_notification(&format!("Failed to load leads: {}", e), NotificationType::Error);
                }
            }
            is_loading.set(false);
        });
    });

    // Filter leads based on search
    let filtered_leads = {
        let query = search_query().to_lowercase();
        if query.is_empty() {
            leads.read().clone()
        } else {
            leads.read()
                .iter()
                .filter(|l| {
                    l.first_name.as_ref().map(|n| n.to_lowercase().contains(&query)).unwrap_or(false)
                        || l.last_name.as_ref().map(|n| n.to_lowercase().contains(&query)).unwrap_or(false)
                        || l.phone.to_lowercase().contains(&query)
                        || l.company.as_ref().map(|c| c.to_lowercase().contains(&query)).unwrap_or(false)
                })
                .cloned()
                .collect()
        }
    };

    rsx! {
        div { class: "h-full flex flex-col",
            // Header
            div { class: "flex items-center justify-between p-4 border-b",
                h2 { class: "text-xl font-semibold", "My Leads" }
                span { class: "text-sm text-gray-500",
                    "{filtered_leads.len()} leads"
                }
            }

            // Search
            div { class: "p-4 border-b",
                input {
                    class: "w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "search",
                    placeholder: "Search leads...",
                    value: "{search_query}",
                    oninput: move |e| search_query.set(e.value()),
                }
            }

            // Content
            div { class: "flex-1 overflow-y-auto p-4",
                if *is_loading.read() {
                    LoadingSpinner {}
                } else if let Some(err) = error.read().as_ref() {
                    div { class: "text-center text-red-500 p-4",
                        "Error loading leads: {err}"
                    }
                } else if filtered_leads.is_empty() {
                    div { class: "text-center text-gray-500 p-8",
                        if search_query().is_empty() {
                            "No leads assigned to you"
                        } else {
                            "No leads match your search"
                        }
                    }
                } else {
                    div { class: "space-y-3",
                        for lead in filtered_leads.iter() {
                            LeadCard {
                                key: "{lead.id}",
                                lead: lead.clone(),
                                agent_id: agent_id,
                            }
                        }
                    }
                }
            }
        }
    }
}
