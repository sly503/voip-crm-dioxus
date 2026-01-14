use dioxus::prelude::*;
use crate::api;
use crate::models::{Agent, AgentStatus};
use crate::components::common::{LoadingSpinner, Card};

#[derive(Clone, Default)]
struct DashboardStats {
    active_calls: i64,
    agents_ready: i64,
    calls_today: i64,
    avg_handle_time: String,
    agents: Vec<Agent>,
}

#[component]
pub fn SupervisorDashboard() -> Element {
    let mut stats = use_signal(DashboardStats::default);
    let mut is_loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    // Fetch realtime stats and agents
    use_effect(move || {
        spawn(async move {
            is_loading.set(true);
            error.set(None);

            let mut dashboard_stats = DashboardStats::default();

            // Fetch realtime stats
            match api::campaigns::get_realtime_stats().await {
                Ok(data) => {
                    dashboard_stats.active_calls = data.get("activeCalls")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    dashboard_stats.agents_ready = data.get("agentsReady")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    dashboard_stats.calls_today = data.get("callsToday")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    let avg_seconds = data.get("avgHandleTime")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    dashboard_stats.avg_handle_time = format!("{}:{:02}", avg_seconds / 60, avg_seconds % 60);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch realtime stats: {}", e);
                    dashboard_stats.avg_handle_time = "0:00".to_string();
                }
            }

            // Fetch agents
            if let Ok(agents) = api::agents::get_all_agents().await {
                dashboard_stats.agents_ready = agents.iter()
                    .filter(|a| a.status == AgentStatus::Ready)
                    .count() as i64;
                dashboard_stats.agents = agents;
            }

            stats.set(dashboard_stats);
            is_loading.set(false);
        });
    });

    let stats_data = stats.read();

    rsx! {
        div { class: "h-full overflow-y-auto p-6",
            h1 { class: "text-2xl font-bold mb-6", "Dashboard" }

            if *is_loading.read() {
                LoadingSpinner {}
            } else {
                // Stats grid
                div { class: "grid gap-6 md:grid-cols-2 lg:grid-cols-4 mb-6",
                    StatCard {
                        title: "Active Calls",
                        value: stats_data.active_calls.to_string(),
                        icon: "\u{1F4DE}",
                        color: "blue",
                    }
                    StatCard {
                        title: "Agents Ready",
                        value: stats_data.agents_ready.to_string(),
                        icon: "\u{2705}",
                        color: "green",
                    }
                    StatCard {
                        title: "Calls Today",
                        value: stats_data.calls_today.to_string(),
                        icon: "\u{1F4CA}",
                        color: "purple",
                    }
                    StatCard {
                        title: "Avg Handle Time",
                        value: stats_data.avg_handle_time.clone(),
                        icon: "\u{23F1}",
                        color: "yellow",
                    }
                }

                // Charts row
                div { class: "grid gap-6 md:grid-cols-2 mb-6",
                    Card {
                        h3 { class: "font-semibold mb-4", "Call Volume" }
                        div { class: "h-48 flex items-center justify-center text-gray-400",
                            "Chart placeholder"
                        }
                    }
                    Card {
                        h3 { class: "font-semibold mb-4", "Agent Performance" }
                        div { class: "h-48 flex items-center justify-center text-gray-400",
                            "Chart placeholder"
                        }
                    }
                }

                // Active calls / agents
                div { class: "grid gap-6 md:grid-cols-2",
                    Card {
                        h3 { class: "font-semibold mb-4", "Active Calls" }
                        if stats_data.active_calls == 0 {
                            div { class: "text-center text-gray-500 py-8",
                                "No active calls"
                            }
                        } else {
                            div { class: "text-center py-8",
                                span { class: "text-4xl font-bold text-blue-600", "{stats_data.active_calls}" }
                                p { class: "text-gray-500 mt-2", "calls in progress" }
                            }
                        }
                    }
                    Card {
                        h3 { class: "font-semibold mb-4", "Agent Status" }
                        div { class: "space-y-2",
                            if stats_data.agents.is_empty() {
                                div { class: "text-center text-gray-500 py-4",
                                    "No agents configured"
                                }
                            } else {
                                for agent in stats_data.agents.iter().take(5) {
                                    AgentStatusRow {
                                        key: "{agent.id}",
                                        name: agent.name.clone(),
                                        status: agent.status.display_name().to_string(),
                                        color: match agent.status {
                                            AgentStatus::Ready => "green",
                                            AgentStatus::OnCall => "blue",
                                            AgentStatus::AfterCall => "yellow",
                                            AgentStatus::Break => "yellow",
                                            AgentStatus::Offline => "gray",
                                        }.to_string(),
                                    }
                                }
                                if stats_data.agents.len() > 5 {
                                    div { class: "text-center text-sm text-gray-500 pt-2",
                                        "+ {stats_data.agents.len() - 5} more agents"
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
fn StatCard(title: String, value: String, icon: String, color: String) -> Element {
    let bg_color = match color.as_str() {
        "blue" => "bg-blue-100 text-blue-600",
        "green" => "bg-green-100 text-green-600",
        "purple" => "bg-purple-100 text-purple-600",
        "yellow" => "bg-yellow-100 text-yellow-600",
        "red" => "bg-red-100 text-red-600",
        _ => "bg-gray-100 text-gray-600",
    };

    rsx! {
        Card {
            div { class: "flex items-center justify-between",
                div {
                    p { class: "text-sm text-gray-500", "{title}" }
                    p { class: "text-3xl font-bold", "{value}" }
                }
                div { class: "w-12 h-12 rounded-full flex items-center justify-center text-2xl {bg_color}",
                    "{icon}"
                }
            }
        }
    }
}

#[component]
fn AgentStatusRow(name: String, status: String, color: String) -> Element {
    let status_color = match color.as_str() {
        "green" => "bg-green-500",
        "red" => "bg-red-500",
        "yellow" => "bg-yellow-500",
        _ => "bg-gray-400",
    };

    rsx! {
        div { class: "flex items-center justify-between p-2 hover:bg-gray-50 rounded",
            div { class: "flex items-center gap-2",
                div { class: "w-8 h-8 bg-blue-100 rounded-full flex items-center justify-center text-blue-600 text-sm font-semibold",
                    "{name.chars().next().unwrap_or('?')}"
                }
                span { "{name}" }
            }
            div { class: "flex items-center gap-2",
                div { class: "w-2 h-2 rounded-full {status_color}" }
                span { class: "text-sm text-gray-500", "{status}" }
            }
        }
    }
}
