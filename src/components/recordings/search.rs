use dioxus::prelude::*;
use chrono::{DateTime, Utc, NaiveDate};
use crate::models::{RecordingSearchParams, Agent, Campaign};
use crate::api;

#[component]
pub fn RecordingSearch(
    on_search: EventHandler<RecordingSearchParams>,
) -> Element {
    let mut agents = use_signal(Vec::<Agent>::new);
    let mut campaigns = use_signal(Vec::<Campaign>::new);

    // Filter state
    let mut agent_id = use_signal(|| None::<i64>);
    let mut campaign_id = use_signal(|| None::<i64>);
    let mut lead_search = use_signal(String::new);
    let mut disposition = use_signal(String::new);
    let mut start_date = use_signal(String::new);
    let mut end_date = use_signal(String::new);
    let mut compliance_hold = use_signal(|| None::<bool>);

    // Load agents and campaigns on mount
    use_effect(move || {
        spawn(async move {
            if let Ok(agents_data) = api::agents::get_all_agents().await {
                agents.set(agents_data);
            }
            if let Ok(campaigns_data) = api::campaigns::get_all_campaigns().await {
                campaigns.set(campaigns_data);
            }
        });
    });

    let handle_search = move |_| {
        // Parse dates
        let start = if start_date().is_empty() {
            None
        } else {
            NaiveDate::parse_from_str(&start_date(), "%Y-%m-%d")
                .ok()
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
        };

        let end = if end_date().is_empty() {
            None
        } else {
            NaiveDate::parse_from_str(&end_date(), "%Y-%m-%d")
                .ok()
                .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc())
        };

        let params = RecordingSearchParams {
            agent_id: agent_id(),
            campaign_id: campaign_id(),
            lead_id: None, // Note: lead_search is a text field, not an ID
            start_date: start,
            end_date: end,
            disposition: if disposition().is_empty() { None } else { Some(disposition()) },
            compliance_hold: compliance_hold(),
            limit: Some(100),
            offset: Some(0),
        };

        on_search.call(params);
    };

    let handle_clear = move |_| {
        agent_id.set(None);
        campaign_id.set(None);
        lead_search.set(String::new());
        disposition.set(String::new());
        start_date.set(String::new());
        end_date.set(String::new());
        compliance_hold.set(None);

        // Trigger search with cleared filters
        let params = RecordingSearchParams {
            agent_id: None,
            campaign_id: None,
            lead_id: None,
            start_date: None,
            end_date: None,
            disposition: None,
            compliance_hold: None,
            limit: Some(100),
            offset: Some(0),
        };
        on_search.call(params);
    };

    rsx! {
        div { class: "bg-white border-b p-4",
            h3 { class: "text-sm font-semibold text-gray-700 mb-3", "Search Filters" }

            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                // Date Range - Start Date
                div {
                    label { class: "block text-xs font-medium text-gray-700 mb-1",
                        "Start Date"
                    }
                    input {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                        r#type: "date",
                        value: "{start_date}",
                        oninput: move |e| start_date.set(e.value()),
                    }
                }

                // Date Range - End Date
                div {
                    label { class: "block text-xs font-medium text-gray-700 mb-1",
                        "End Date"
                    }
                    input {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                        r#type: "date",
                        value: "{end_date}",
                        oninput: move |e| end_date.set(e.value()),
                    }
                }

                // Agent Filter
                div {
                    label { class: "block text-xs font-medium text-gray-700 mb-1",
                        "Agent"
                    }
                    select {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                        onchange: move |e| {
                            let value = e.value();
                            agent_id.set(if value.is_empty() {
                                None
                            } else {
                                value.parse::<i64>().ok()
                            });
                        },
                        option { value: "", "All Agents" }
                        for agent in agents.read().iter() {
                            option {
                                value: "{agent.id}",
                                selected: agent_id() == Some(agent.id),
                                "{agent.name}"
                            }
                        }
                    }
                }

                // Campaign Filter
                div {
                    label { class: "block text-xs font-medium text-gray-700 mb-1",
                        "Campaign"
                    }
                    select {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                        onchange: move |e| {
                            let value = e.value();
                            campaign_id.set(if value.is_empty() {
                                None
                            } else {
                                value.parse::<i64>().ok()
                            });
                        },
                        option { value: "", "All Campaigns" }
                        for campaign in campaigns.read().iter() {
                            option {
                                value: "{campaign.id}",
                                selected: campaign_id() == Some(campaign.id),
                                "{campaign.name}"
                            }
                        }
                    }
                }

                // Lead Name/Phone Filter
                div {
                    label { class: "block text-xs font-medium text-gray-700 mb-1",
                        "Lead Name/Phone"
                    }
                    input {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                        r#type: "text",
                        placeholder: "Search by name or phone...",
                        value: "{lead_search}",
                        oninput: move |e| lead_search.set(e.value()),
                    }
                }

                // Disposition Filter
                div {
                    label { class: "block text-xs font-medium text-gray-700 mb-1",
                        "Disposition"
                    }
                    select {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                        onchange: move |e| disposition.set(e.value()),
                        option { value: "", "All Dispositions" }
                        option {
                            value: "Connected",
                            selected: disposition() == "Connected",
                            "Connected"
                        }
                        option {
                            value: "Sale",
                            selected: disposition() == "Sale",
                            "Sale"
                        }
                        option {
                            value: "Callback",
                            selected: disposition() == "Callback",
                            "Callback"
                        }
                        option {
                            value: "No Answer",
                            selected: disposition() == "No Answer",
                            "No Answer"
                        }
                        option {
                            value: "Busy",
                            selected: disposition() == "Busy",
                            "Busy"
                        }
                        option {
                            value: "Voicemail",
                            selected: disposition() == "Voicemail",
                            "Voicemail"
                        }
                        option {
                            value: "Failed",
                            selected: disposition() == "Failed",
                            "Failed"
                        }
                        option {
                            value: "Rejected",
                            selected: disposition() == "Rejected",
                            "Rejected"
                        }
                        option {
                            value: "Do Not Call",
                            selected: disposition() == "Do Not Call",
                            "Do Not Call"
                        }
                    }
                }

                // Compliance Hold Filter
                div {
                    label { class: "block text-xs font-medium text-gray-700 mb-1",
                        "Compliance Hold"
                    }
                    select {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                        onchange: move |e| {
                            let value = e.value();
                            compliance_hold.set(match value.as_str() {
                                "true" => Some(true),
                                "false" => Some(false),
                                _ => None,
                            });
                        },
                        option { value: "", "All Recordings" }
                        option {
                            value: "true",
                            selected: compliance_hold() == Some(true),
                            "On Hold"
                        }
                        option {
                            value: "false",
                            selected: compliance_hold() == Some(false),
                            "Not On Hold"
                        }
                    }
                }

                // Action Buttons (spans 2 columns on larger screens)
                div { class: "md:col-span-2 lg:col-span-1 flex gap-2",
                    button {
                        class: "flex-1 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm font-medium",
                        onclick: handle_search,
                        "Search"
                    }
                    button {
                        class: "flex-1 px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 focus:outline-none focus:ring-2 focus:ring-gray-400 text-sm font-medium",
                        onclick: handle_clear,
                        "Clear Filters"
                    }
                }
            }
        }
    }
}
