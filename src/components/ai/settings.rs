use dioxus::prelude::*;
use crate::components::common::Card;
use crate::api::ai;
use crate::models::GlobalAiConfig;

#[component]
pub fn AISettings() -> Element {
    let mut config = use_signal(GlobalAiConfig::default);
    let mut is_loading = use_signal(|| true);
    let mut is_saving = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

    // Load config on mount
    use_effect(move || {
        spawn(async move {
            match ai::get_global_config().await {
                Ok(loaded_config) => {
                    config.set(loaded_config);
                }
                Err(e) => {
                    // Use default config if not found
                    tracing::warn!("Failed to load AI config: {}, using defaults", e);
                }
            }
            is_loading.set(false);
        });
    });

    let save_settings = move |_| {
        is_saving.set(true);
        error_message.set(None);

        let current_config = config.read().clone();
        spawn(async move {
            match ai::update_global_config(current_config).await {
                Ok(updated) => {
                    config.set(updated);
                    crate::state::show_notification("Settings saved", crate::state::NotificationType::Success);
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to save: {}", e)));
                    crate::state::show_notification("Failed to save settings", crate::state::NotificationType::Error);
                }
            }
            is_saving.set(false);
        });
    };

    if *is_loading.read() {
        return rsx! {
            div { class: "h-full flex items-center justify-center",
                div { class: "text-gray-500", "Loading AI settings..." }
            }
        };
    }

    let current = config.read().clone();

    rsx! {
        div { class: "h-full overflow-y-auto p-6",
            div { class: "max-w-2xl mx-auto",
                h1 { class: "text-2xl font-bold mb-6", "AI Agent Settings" }

                // Error message
                if let Some(ref err) = *error_message.read() {
                    div { class: "mb-4 p-4 bg-red-50 text-red-700 rounded-lg",
                        "{err}"
                    }
                }

                // Model selection
                Card { class: "mb-6",
                    h3 { class: "font-semibold mb-4", "AI Model" }

                    div { class: "space-y-4",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "Model" }
                            select {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                                value: "{current.model}",
                                onchange: move |e| {
                                    config.write().model = e.value();
                                },
                                option { value: "claude-sonnet-4-5-20250514", "Claude Sonnet 4.5 (Recommended)" }
                                option { value: "claude-opus-4-5-20251101", "Claude Opus 4.5 (Best Quality)" }
                                option { value: "claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet" }
                            }
                        }

                        p { class: "text-sm text-gray-500",
                            "The AI model used for phone conversations. Better models provide more natural conversations but cost more."
                        }
                    }
                }

                // Cost optimization
                Card { class: "mb-6",
                    h3 { class: "font-semibold mb-4", "Cost Optimization" }

                    div { class: "space-y-4",
                        label { class: "flex items-center gap-3 p-3 bg-gray-50 rounded-lg cursor-pointer",
                            input {
                                r#type: "checkbox",
                                class: "w-5 h-5 rounded border-gray-300",
                                checked: current.use_claude_code,
                                onchange: move |e| {
                                    config.write().use_claude_code = e.checked();
                                },
                            }
                            div {
                                div { class: "font-medium", "Use Claude Code Agents (Lower Cost)" }
                                div { class: "text-sm text-gray-500",
                                    "Uses Claude Code CLI instead of API calls. Significantly reduces per-call costs."
                                }
                            }
                        }

                        label { class: "flex items-center gap-3 p-3 bg-gray-50 rounded-lg cursor-pointer",
                            input {
                                r#type: "checkbox",
                                class: "w-5 h-5 rounded border-gray-300",
                                checked: current.fallback_to_api,
                                onchange: move |e| {
                                    config.write().fallback_to_api = e.checked();
                                },
                            }
                            div {
                                div { class: "font-medium", "Fallback to API if Claude Code fails" }
                                div { class: "text-sm text-gray-500",
                                    "If Claude Code agent fails, automatically retry with direct API calls."
                                }
                            }
                        }

                        // Cost estimate
                        div { class: "p-4 bg-blue-50 rounded-lg",
                            h4 { class: "font-medium text-blue-800 mb-2", "Estimated Costs" }
                            div { class: "grid grid-cols-2 gap-4 text-sm",
                                div {
                                    div { class: "text-blue-600 font-medium", "Claude Code" }
                                    div { class: "text-blue-800", "~$0.00/call (subscription)" }
                                }
                                div {
                                    div { class: "text-blue-600 font-medium", "Direct API" }
                                    div { class: "text-blue-800", "~$0.05-0.15/call" }
                                }
                            }
                        }
                    }
                }

                // Voice settings
                Card { class: "mb-6",
                    h3 { class: "font-semibold mb-4", "Voice Settings" }

                    div { class: "space-y-4",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "TTS Voice" }
                            select {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                                value: "{current.default_voice}",
                                onchange: move |e| {
                                    config.write().default_voice = e.value();
                                },
                                option { value: "alloy", "Alloy (Neutral)" }
                                option { value: "echo", "Echo (Male)" }
                                option { value: "fable", "Fable (British)" }
                                option { value: "onyx", "Onyx (Deep Male)" }
                                option { value: "nova", "Nova (Female)" }
                                option { value: "shimmer", "Shimmer (Soft Female)" }
                            }
                        }

                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "STT Provider" }
                            select {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                                value: "{current.stt_provider}",
                                onchange: move |e| {
                                    config.write().stt_provider = e.value();
                                },
                                option { value: "deepgram", "Deepgram (Recommended)" }
                                option { value: "whisper", "OpenAI Whisper" }
                                option { value: "google", "Google Speech-to-Text" }
                            }
                        }

                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "TTS Provider" }
                            select {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                                value: "{current.tts_provider}",
                                onchange: move |e| {
                                    config.write().tts_provider = e.value();
                                },
                                option { value: "openai", "OpenAI (Recommended)" }
                                option { value: "elevenlabs", "ElevenLabs" }
                                option { value: "telnyx", "Telnyx TTS" }
                            }
                        }
                    }
                }

                // Call settings
                Card { class: "mb-6",
                    h3 { class: "font-semibold mb-4", "Call Settings" }

                    div { class: "space-y-4",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                "Max Call Duration (seconds)"
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                                r#type: "number",
                                min: 60,
                                max: 3600,
                                value: "{current.max_call_duration}",
                                oninput: move |e| {
                                    if let Ok(v) = e.value().parse() {
                                        config.write().max_call_duration = v;
                                    }
                                },
                            }
                            p { class: "text-sm text-gray-500 mt-1",
                                "{current.max_call_duration / 60} minutes {current.max_call_duration % 60} seconds"
                            }
                        }
                    }
                }

                // Save button
                div { class: "flex justify-end",
                    button {
                        class: "px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 font-medium disabled:opacity-50",
                        disabled: *is_saving.read(),
                        onclick: save_settings,
                        if *is_saving.read() { "Saving..." } else { "Save Settings" }
                    }
                }
            }
        }
    }
}
