use dioxus::prelude::*;
use crate::models::{Agent, AgentType, AgentStatus, CreateAgentRequest};
use crate::api;
use crate::components::common::{LoadingSpinner, Card};

#[component]
pub fn AgentList() -> Element {
    let mut agents = use_signal(Vec::<Agent>::new);
    let mut is_loading = use_signal(|| true);
    let mut show_create_modal = use_signal(|| false);

    // Fetch agents
    use_effect(move || {
        spawn(async move {
            is_loading.set(true);
            if let Ok(data) = api::agents::get_all_agents().await {
                agents.set(data);
            }
            is_loading.set(false);
        });
    });

    let human_agents = agents.read().iter().filter(|a| a.agent_type == AgentType::Human).count();
    let ai_agents = agents.read().iter().filter(|a| a.agent_type == AgentType::Ai).count();
    let ready_agents = agents.read().iter().filter(|a| a.status == AgentStatus::Ready).count();

    rsx! {
        div { class: "h-full flex flex-col",
            // Header
            div { class: "flex items-center justify-between p-4 border-b",
                h2 { class: "text-xl font-semibold", "Agents" }
                button {
                    class: "px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700",
                    onclick: move |_| show_create_modal.set(true),
                    "+ Add Agent"
                }
            }

            // Stats
            div { class: "flex gap-4 p-4 border-b bg-gray-50",
                div { class: "text-center",
                    div { class: "text-2xl font-bold text-blue-600", "{agents.read().len()}" }
                    div { class: "text-xs text-gray-500", "Total" }
                }
                div { class: "text-center",
                    div { class: "text-2xl font-bold text-green-600", "{ready_agents}" }
                    div { class: "text-xs text-gray-500", "Ready" }
                }
                div { class: "text-center",
                    div { class: "text-2xl font-bold text-gray-600", "{human_agents}" }
                    div { class: "text-xs text-gray-500", "Human" }
                }
                div { class: "text-center",
                    div { class: "text-2xl font-bold text-purple-600", "{ai_agents}" }
                    div { class: "text-xs text-gray-500", "AI" }
                }
            }

            // Content
            div { class: "flex-1 overflow-y-auto p-4",
                if *is_loading.read() {
                    LoadingSpinner {}
                } else if agents.read().is_empty() {
                    div { class: "text-center text-gray-500 p-8",
                        "No agents yet. Add your first agent!"
                    }
                } else {
                    div { class: "grid gap-4 md:grid-cols-2 lg:grid-cols-3",
                        for agent in agents.read().iter() {
                            AgentCard {
                                key: "{agent.id}",
                                agent: agent.clone(),
                            }
                        }
                    }
                }
            }

            // Create Modal
            if *show_create_modal.read() {
                CreateAgentModal {
                    on_close: move |_| show_create_modal.set(false),
                    on_created: move |a| {
                        agents.write().push(a);
                        show_create_modal.set(false);
                    },
                }
            }
        }
    }
}

#[component]
fn AgentCard(agent: Agent) -> Element {
    let agent_id = agent.id;

    rsx! {
        Card {
            div { class: "flex items-start justify-between mb-3",
                div { class: "flex items-center gap-3",
                    div {
                        class: "w-12 h-12 rounded-full flex items-center justify-center text-white font-semibold",
                        class: if agent.agent_type == AgentType::Ai { "bg-purple-500" } else { "bg-blue-500" },
                        if agent.agent_type == AgentType::Ai {
                            "\u{1F916}"
                        } else {
                            "{agent.name.chars().next().unwrap_or('?')}"
                        }
                    }
                    div {
                        h3 { class: "font-semibold", "{agent.name}" }
                        span { class: "text-sm text-gray-500",
                            "{agent.agent_type.display_name()}"
                        }
                    }
                }

                div { class: "flex items-center gap-2",
                    div {
                        class: "w-3 h-3 rounded-full {agent.status.color_class()}"
                    }
                    span { class: "text-sm", "{agent.status.display_name()}" }
                }
            }

            if let Some(ext) = &agent.extension {
                div { class: "text-sm text-gray-500 mb-3",
                    "Extension: {ext}"
                }
            }

            // Status controls
            div { class: "flex gap-2",
                StatusButton { agent_id, status: AgentStatus::Ready, current: agent.status }
                StatusButton { agent_id, status: AgentStatus::Offline, current: agent.status }
                StatusButton { agent_id, status: AgentStatus::Break, current: agent.status }
            }
        }
    }
}

#[component]
fn StatusButton(agent_id: i64, status: AgentStatus, current: AgentStatus) -> Element {
    let is_current = status == current;
    let mut is_updating = use_signal(|| false);

    let update = move |_| {
        if is_current {
            return;
        }

        is_updating.set(true);
        spawn(async move {
            if let Err(e) = api::agents::update_agent_status(agent_id, status).await {
                tracing::error!("Failed to update status: {}", e);
            }
            is_updating.set(false);
        });
    };

    rsx! {
        button {
            class: "px-3 py-1 rounded text-xs transition-colors",
            class: if is_current { "bg-blue-600 text-white" } else { "bg-gray-100 hover:bg-gray-200" },
            disabled: is_current || *is_updating.read(),
            onclick: update,
            "{status.display_name()}"
        }
    }
}

#[component]
fn CreateAgentModal(
    on_close: EventHandler<MouseEvent>,
    on_created: EventHandler<Agent>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut agent_type = use_signal(|| AgentType::Human);
    let mut extension = use_signal(String::new);
    let mut is_creating = use_signal(|| false);

    let create = move |_| {
        if name().is_empty() {
            return;
        }

        is_creating.set(true);
        let request = CreateAgentRequest {
            name: name(),
            agent_type: agent_type(),
            user_id: None,
            extension: if extension().is_empty() { None } else { Some(extension()) },
        };

        spawn(async move {
            match api::agents::create_agent(request).await {
                Ok(agent) => {
                    on_created.call(agent);
                }
                Err(e) => {
                    tracing::error!("Failed to create agent: {}", e);
                }
            }
            is_creating.set(false);
        });
    };

    rsx! {
        div { class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
            div { class: "bg-white rounded-lg p-6 w-full max-w-md",
                h3 { class: "text-lg font-semibold mb-4", "Add Agent" }

                div { class: "space-y-4",
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Name *" }
                        input {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                            r#type: "text",
                            placeholder: "Agent name",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Agent Type" }
                        div { class: "flex gap-4",
                            label { class: "flex items-center gap-2 cursor-pointer",
                                input {
                                    r#type: "radio",
                                    name: "agent_type",
                                    checked: agent_type() == AgentType::Human,
                                    onchange: move |_| agent_type.set(AgentType::Human),
                                }
                                span { "\u{1F464} Human Agent" }
                            }
                            label { class: "flex items-center gap-2 cursor-pointer",
                                input {
                                    r#type: "radio",
                                    name: "agent_type",
                                    checked: agent_type() == AgentType::Ai,
                                    onchange: move |_| agent_type.set(AgentType::Ai),
                                }
                                span { "\u{1F916} AI Agent" }
                            }
                        }
                    }

                    if agent_type() == AgentType::Human {
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "Extension" }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                                r#type: "text",
                                placeholder: "SIP extension (optional)",
                                value: "{extension}",
                                oninput: move |e| extension.set(e.value()),
                            }
                        }
                    }

                    if agent_type() == AgentType::Ai {
                        div { class: "bg-purple-50 p-3 rounded-lg text-sm text-purple-700",
                            "AI agents will use the configured AI model and prompts. Configure AI settings in the AI Settings page."
                        }
                    }
                }

                div { class: "flex justify-end gap-2 mt-6",
                    button {
                        class: "px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg",
                        onclick: move |e| on_close.call(e),
                        "Cancel"
                    }
                    button {
                        class: "px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50",
                        disabled: name().is_empty() || *is_creating.read(),
                        onclick: create,
                        if *is_creating.read() { "Creating..." } else { "Create" }
                    }
                }
            }
        }
    }
}
