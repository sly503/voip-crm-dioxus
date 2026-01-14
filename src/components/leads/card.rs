use dioxus::prelude::*;
use crate::models::Lead;
use crate::components::phone::QuickDial;

#[component]
pub fn LeadCard(lead: Lead, agent_id: i64) -> Element {
    let lead_id = lead.id;

    let select_lead = move |_| {
        crate::state::select_lead(Some(lead_id));
    };

    rsx! {
        div {
            class: "bg-white border rounded-lg p-4 hover:shadow-md transition-shadow cursor-pointer",
            onclick: select_lead,

            // Header
            div { class: "flex items-start justify-between mb-3",
                div {
                    h3 { class: "font-semibold text-lg", "{lead.full_name()}" }
                    if let Some(company) = &lead.company {
                        p { class: "text-sm text-gray-500", "{company}" }
                    }
                }
                span { class: "px-2 py-1 rounded-full text-xs {lead.status.color_class()}",
                    "{lead.status.display_name()}"
                }
            }

            // Contact info
            div { class: "space-y-2 mb-3",
                div { class: "flex items-center gap-2 text-sm",
                    span { class: "text-gray-400", "\u{1F4DE}" }
                    span { "{lead.phone}" }
                }
                if let Some(email) = &lead.email {
                    div { class: "flex items-center gap-2 text-sm",
                        span { class: "text-gray-400", "\u{2709}" }
                        span { class: "text-gray-600", "{email}" }
                    }
                }
            }

            // Actions
            div { class: "flex items-center justify-between pt-3 border-t",
                QuickDial {
                    lead: lead.clone(),
                    agent_id: agent_id,
                }

                div { class: "flex gap-2",
                    button {
                        class: "text-sm text-gray-500 hover:text-gray-700",
                        onclick: move |e| {
                            e.stop_propagation();
                            // Add note action
                        },
                        "Add Note"
                    }
                }
            }
        }
    }
}

#[component]
pub fn LeadCardCompact(lead: Lead, agent_id: i64) -> Element {
    let initial = lead.first_name
        .as_ref()
        .and_then(|n| n.chars().next())
        .unwrap_or('?');

    rsx! {
        div { class: "flex items-center justify-between p-3 bg-white border rounded-lg hover:bg-gray-50",
            div { class: "flex items-center gap-3",
                div { class: "w-10 h-10 bg-blue-100 rounded-full flex items-center justify-center text-blue-600 font-semibold",
                    "{initial}"
                }
                div {
                    div { class: "font-medium", "{lead.full_name()}" }
                    div { class: "text-sm text-gray-500", "{lead.phone}" }
                }
            }

            div { class: "flex items-center gap-2",
                span { class: "px-2 py-1 rounded-full text-xs {lead.status.color_class()}",
                    "{lead.status.display_name()}"
                }
                QuickDial {
                    lead: lead.clone(),
                    agent_id: agent_id,
                }
            }
        }
    }
}
