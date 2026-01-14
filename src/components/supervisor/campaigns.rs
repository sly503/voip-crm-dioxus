use dioxus::prelude::*;
use crate::models::{Campaign, CampaignStatus, DialerMode, CreateCampaignRequest};
use crate::api;
use crate::components::common::{LoadingSpinner, Card};

#[component]
pub fn CampaignList() -> Element {
    let mut campaigns = use_signal(Vec::<Campaign>::new);
    let mut is_loading = use_signal(|| true);
    let mut show_create_modal = use_signal(|| false);

    // Fetch campaigns
    use_effect(move || {
        spawn(async move {
            is_loading.set(true);
            if let Ok(data) = api::campaigns::get_all_campaigns().await {
                campaigns.set(data);
            }
            is_loading.set(false);
        });
    });

    rsx! {
        div { class: "h-full flex flex-col",
            // Header
            div { class: "flex items-center justify-between p-4 border-b",
                h2 { class: "text-xl font-semibold", "Campaigns" }
                button {
                    class: "px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700",
                    onclick: move |_| show_create_modal.set(true),
                    "+ New Campaign"
                }
            }

            // Content
            div { class: "flex-1 overflow-y-auto p-4",
                if *is_loading.read() {
                    LoadingSpinner {}
                } else if campaigns.read().is_empty() {
                    div { class: "text-center text-gray-500 p-8",
                        "No campaigns yet. Create your first campaign!"
                    }
                } else {
                    div { class: "space-y-4",
                        for campaign in campaigns.read().iter() {
                            CampaignCard {
                                key: "{campaign.id}",
                                campaign: campaign.clone(),
                            }
                        }
                    }
                }
            }

            // Create Modal
            if *show_create_modal.read() {
                CreateCampaignModal {
                    on_close: move |_| show_create_modal.set(false),
                    on_created: move |c| {
                        campaigns.write().push(c);
                        show_create_modal.set(false);
                    },
                }
            }
        }
    }
}

#[component]
fn CampaignCard(campaign: Campaign) -> Element {
    let campaign_id = campaign.id;
    let campaign_for_modal = campaign.clone();
    let mut is_loading = use_signal(|| false);
    let mut dialer_running = use_signal(|| campaign.status == CampaignStatus::Active);
    let mut show_settings = use_signal(|| false);

    let toggle_dialer = move |_| {
        is_loading.set(true);
        let running = *dialer_running.read();

        spawn(async move {
            let result = if running {
                api::campaigns::pause_dialer(campaign_id).await
            } else {
                api::campaigns::start_dialer(campaign_id).await
            };

            if result.is_ok() {
                dialer_running.set(!running);
            }
            is_loading.set(false);
        });
    };

    let progress = campaign.dialed_leads.unwrap_or(0) as f64 /
        campaign.total_leads.unwrap_or(1).max(1) as f64 * 100.0;

    rsx! {
        Card {
            div { class: "flex items-start justify-between mb-4",
                div {
                    h3 { class: "font-semibold text-lg", "{campaign.name}" }
                    if let Some(desc) = &campaign.description {
                        p { class: "text-sm text-gray-500", "{desc}" }
                    }
                }
                span { class: "px-2 py-1 rounded-full text-xs {campaign.status.color_class()}",
                    "{campaign.status.display_name()}"
                }
            }

            // Stats
            div { class: "grid grid-cols-3 gap-4 mb-4",
                div { class: "text-center",
                    div { class: "text-2xl font-bold text-blue-600",
                        "{campaign.total_leads.unwrap_or(0)}"
                    }
                    div { class: "text-xs text-gray-500", "Total Leads" }
                }
                div { class: "text-center",
                    div { class: "text-2xl font-bold text-yellow-600",
                        "{campaign.dialed_leads.unwrap_or(0)}"
                    }
                    div { class: "text-xs text-gray-500", "Dialed" }
                }
                div { class: "text-center",
                    div { class: "text-2xl font-bold text-green-600",
                        "{campaign.connected_leads.unwrap_or(0)}"
                    }
                    div { class: "text-xs text-gray-500", "Connected" }
                }
            }

            // Progress bar
            div { class: "mb-4",
                div { class: "flex justify-between text-sm text-gray-600 mb-1",
                    span { "Progress" }
                    span { "{progress:.1}%" }
                }
                div { class: "h-2 bg-gray-200 rounded-full overflow-hidden",
                    div {
                        class: "h-full bg-blue-600 transition-all",
                        style: "width: {progress}%",
                    }
                }
            }

            // Dialer mode
            div { class: "flex items-center justify-between text-sm mb-4",
                span { class: "text-gray-500", "Dialer Mode:" }
                span { class: "font-medium", "{campaign.dialer_mode.display_name()}" }
            }

            // Actions
            div { class: "flex gap-2",
                button {
                    class: "flex-1 py-2 rounded-lg font-medium transition-colors",
                    class: if *dialer_running.read() {
                        "bg-yellow-500 hover:bg-yellow-600 text-white"
                    } else {
                        "bg-green-500 hover:bg-green-600 text-white"
                    },
                    disabled: *is_loading.read(),
                    onclick: toggle_dialer,
                    if *is_loading.read() {
                        "..."
                    } else if *dialer_running.read() {
                        "Pause Dialer"
                    } else {
                        "Start Dialer"
                    }
                }
                button {
                    class: "px-4 py-2 bg-gray-100 hover:bg-gray-200 rounded-lg",
                    onclick: move |_| show_settings.set(true),
                    "Settings"
                }
            }
        }

        // Settings Modal
        if *show_settings.read() {
            CampaignSettingsModal {
                campaign: campaign_for_modal.clone(),
                on_close: move |_| show_settings.set(false),
            }
        }
    }
}

#[component]
fn CreateCampaignModal(
    on_close: EventHandler<MouseEvent>,
    on_created: EventHandler<Campaign>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut dialer_mode = use_signal(|| DialerMode::Progressive);
    let mut is_creating = use_signal(|| false);

    let create = move |_| {
        if name().is_empty() {
            return;
        }

        is_creating.set(true);
        let request = CreateCampaignRequest {
            name: name(),
            description: if description().is_empty() { None } else { Some(description()) },
            dialer_mode: dialer_mode(),
            caller_id: None,
            start_time: None,
            end_time: None,
            max_attempts: Some(3),
            retry_delay_minutes: Some(30),
        };

        spawn(async move {
            match api::campaigns::create_campaign(request).await {
                Ok(campaign) => {
                    on_created.call(campaign);
                }
                Err(e) => {
                    tracing::error!("Failed to create campaign: {}", e);
                }
            }
            is_creating.set(false);
        });
    };

    rsx! {
        div { class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
            div { class: "bg-white rounded-lg p-6 w-full max-w-md",
                h3 { class: "text-lg font-semibold mb-4", "Create Campaign" }

                div { class: "space-y-4",
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Name *" }
                        input {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                            r#type: "text",
                            placeholder: "Campaign name",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Description" }
                        textarea {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                            rows: 3,
                            placeholder: "Campaign description",
                            value: "{description}",
                            oninput: move |e| description.set(e.value()),
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Dialer Mode" }
                        select {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                            onchange: move |e| {
                                dialer_mode.set(match e.value().as_str() {
                                    "PREVIEW" => DialerMode::Preview,
                                    "PROGRESSIVE" => DialerMode::Progressive,
                                    "PREDICTIVE" => DialerMode::Predictive,
                                    _ => DialerMode::Progressive,
                                });
                            },
                            option { value: "PREVIEW", "Preview - Agent reviews before dial" }
                            option { value: "PROGRESSIVE", selected: true, "Progressive - Auto-dial when ready" }
                            option { value: "PREDICTIVE", "Predictive - Multi-line dialing" }
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

#[component]
fn CampaignSettingsModal(
    campaign: Campaign,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    let mut dialer_mode = use_signal(|| campaign.dialer_mode.clone());
    let mut max_attempts = use_signal(|| campaign.max_attempts.unwrap_or(3).to_string());
    let mut retry_delay = use_signal(|| campaign.retry_delay_minutes.unwrap_or(30).to_string());
    let mut is_saving = use_signal(|| false);
    let campaign_id = campaign.id;
    let campaign_name = campaign.name.clone();
    let campaign_desc = campaign.description.clone();
    let campaign_caller_id = campaign.caller_id.clone();

    let save_settings = move |_| {
        is_saving.set(true);
        let mode = dialer_mode();
        let attempts: i32 = max_attempts().parse().unwrap_or(3);
        let delay: i32 = retry_delay().parse().unwrap_or(30);
        let name = campaign_name.clone();
        let desc = campaign_desc.clone();
        let caller_id = campaign_caller_id.clone();

        spawn(async move {
            let request = CreateCampaignRequest {
                name,
                description: desc,
                dialer_mode: mode,
                caller_id,
                start_time: None,
                end_time: None,
                max_attempts: Some(attempts),
                retry_delay_minutes: Some(delay),
            };

            match api::campaigns::update_campaign(campaign_id, request).await {
                Ok(_) => {
                    crate::state::show_notification("Campaign settings saved", crate::state::NotificationType::Success);
                }
                Err(e) => {
                    crate::state::show_notification(&format!("Failed to save: {}", e), crate::state::NotificationType::Error);
                }
            }
            is_saving.set(false);
        });
    };

    rsx! {
        div { class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
            onclick: move |e| on_close.call(e),
            div {
                class: "bg-white rounded-lg p-6 w-full max-w-md",
                onclick: move |e| e.stop_propagation(),

                h3 { class: "text-lg font-semibold mb-4", "Campaign Settings" }
                p { class: "text-sm text-gray-500 mb-4", "{campaign.name}" }

                div { class: "space-y-4",
                    // Dialer Mode
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Dialer Mode" }
                        select {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                            onchange: move |e| {
                                dialer_mode.set(match e.value().as_str() {
                                    "PREVIEW" => DialerMode::Preview,
                                    "PROGRESSIVE" => DialerMode::Progressive,
                                    "PREDICTIVE" => DialerMode::Predictive,
                                    _ => DialerMode::Progressive,
                                });
                            },
                            option {
                                value: "PREVIEW",
                                selected: matches!(dialer_mode(), DialerMode::Preview),
                                "Preview - Agent reviews before dial"
                            }
                            option {
                                value: "PROGRESSIVE",
                                selected: matches!(dialer_mode(), DialerMode::Progressive),
                                "Progressive - Auto-dial when ready"
                            }
                            option {
                                value: "PREDICTIVE",
                                selected: matches!(dialer_mode(), DialerMode::Predictive),
                                "Predictive - Multi-line dialing"
                            }
                        }
                    }

                    // Max Attempts
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Max Call Attempts" }
                        input {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                            r#type: "number",
                            min: "1",
                            max: "10",
                            value: "{max_attempts}",
                            oninput: move |e| max_attempts.set(e.value()),
                        }
                        p { class: "text-xs text-gray-500 mt-1", "Number of times to attempt calling each lead" }
                    }

                    // Retry Delay
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Retry Delay (minutes)" }
                        input {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                            r#type: "number",
                            min: "5",
                            max: "1440",
                            value: "{retry_delay}",
                            oninput: move |e| retry_delay.set(e.value()),
                        }
                        p { class: "text-xs text-gray-500 mt-1", "Time between retry attempts" }
                    }

                    // Campaign Status Info
                    div { class: "bg-gray-50 rounded-lg p-3",
                        div { class: "flex justify-between text-sm",
                            span { class: "text-gray-500", "Status:" }
                            span { class: "font-medium {campaign.status.color_class()}", "{campaign.status.display_name()}" }
                        }
                        div { class: "flex justify-between text-sm mt-2",
                            span { class: "text-gray-500", "Total Leads:" }
                            span { class: "font-medium", "{campaign.total_leads.unwrap_or(0)}" }
                        }
                        div { class: "flex justify-between text-sm mt-2",
                            span { class: "text-gray-500", "Dialed:" }
                            span { class: "font-medium", "{campaign.dialed_leads.unwrap_or(0)}" }
                        }
                    }
                }

                div { class: "flex justify-end gap-2 mt-6",
                    button {
                        class: "px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg",
                        onclick: move |e| on_close.call(e),
                        "Close"
                    }
                    button {
                        class: "px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50",
                        disabled: *is_saving.read(),
                        onclick: save_settings,
                        if *is_saving.read() { "Saving..." } else { "Save Changes" }
                    }
                }
            }
        }
    }
}
