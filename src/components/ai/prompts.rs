use dioxus::prelude::*;
use crate::api::ai;
use crate::models::PromptTemplate;

#[component]
pub fn PromptEditor() -> Element {
    let mut prompts = use_signal(Vec::<PromptTemplate>::new);
    let mut selected_prompt_id = use_signal(|| None::<String>);
    let mut editing_content = use_signal(String::new);
    let mut show_create_modal = use_signal(|| false);
    let mut is_loading = use_signal(|| true);
    let mut is_saving = use_signal(|| false);

    // Load prompts on mount
    use_effect(move || {
        spawn(async move {
            match ai::get_templates().await {
                Ok(loaded_prompts) => {
                    prompts.set(loaded_prompts);
                }
                Err(e) => {
                    tracing::error!("Failed to load prompts: {}", e);
                }
            }
            is_loading.set(false);
        });
    });

    let prompt_list: Vec<PromptTemplate> = prompts.read().clone();
    let selected_id = selected_prompt_id.read().clone();
    let selected_prompt = selected_id.as_ref().and_then(|id| prompt_list.iter().find(|p| &p.id == id).cloned());

    let save_prompt = move |_| {
        let Some(id) = selected_prompt_id.read().clone() else { return };
        let content = editing_content.read().clone();

        // Clone the template before the async block
        let template_opt = prompts.read().iter().find(|p| p.id == id).cloned();

        is_saving.set(true);
        spawn(async move {
            if let Some(mut template) = template_opt {
                template.content = content;
                match ai::update_template(&id, template.clone()).await {
                    Ok(updated) => {
                        // Update the local list
                        let mut list = prompts.read().clone();
                        if let Some(idx) = list.iter().position(|p| p.id == id) {
                            list[idx] = updated;
                            prompts.set(list);
                        }
                        crate::state::show_notification("Prompt saved", crate::state::NotificationType::Success);
                    }
                    Err(e) => {
                        tracing::error!("Failed to save prompt: {}", e);
                        crate::state::show_notification("Failed to save prompt", crate::state::NotificationType::Error);
                    }
                }
            }
            is_saving.set(false);
        });
    };

    if *is_loading.read() {
        return rsx! {
            div { class: "h-full flex items-center justify-center",
                div { class: "text-gray-500", "Loading prompts..." }
            }
        };
    }

    rsx! {
        div { class: "h-full flex",
            // Prompt list
            div { class: "w-80 border-r overflow-y-auto",
                div { class: "p-4 border-b",
                    h2 { class: "font-semibold", "AI Prompts" }
                }
                div { class: "p-2",
                    button {
                        class: "w-full py-2 px-4 bg-blue-600 text-white rounded-lg hover:bg-blue-700 mb-4",
                        onclick: move |_| show_create_modal.set(true),
                        "+ New Prompt"
                    }

                    if prompt_list.is_empty() {
                        div { class: "text-gray-500 text-center py-4",
                            "No prompts yet. Create one to get started."
                        }
                    }

                    for prompt in prompt_list.iter() {
                        PromptListItem {
                            key: "{prompt.id}",
                            prompt: prompt.clone(),
                            is_selected: selected_id.as_ref() == Some(&prompt.id),
                            on_select: move |id: String| {
                                selected_prompt_id.set(Some(id.clone()));
                                if let Some(p) = prompts.read().iter().find(|p| p.id == id) {
                                    editing_content.set(p.content.clone());
                                }
                            },
                        }
                    }
                }
            }

            // Editor
            div { class: "flex-1 flex flex-col",
                if let Some(prompt) = selected_prompt {
                    // Header
                    div { class: "p-4 border-b flex items-center justify-between",
                        div {
                            h2 { class: "font-semibold text-lg", "{prompt.name}" }
                            span { class: "text-sm text-gray-500", "{prompt.category}" }
                        }
                        div { class: "flex gap-2",
                            button {
                                class: "px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50",
                                disabled: *is_saving.read(),
                                onclick: save_prompt,
                                if *is_saving.read() { "Saving..." } else { "Save" }
                            }
                        }
                    }

                    // Editor area
                    div { class: "flex-1 p-4",
                        label { class: "block text-sm font-medium text-gray-700 mb-2", "Prompt Content" }
                        textarea {
                            class: "w-full h-64 px-4 py-3 border border-gray-300 rounded-lg font-mono text-sm resize-none focus:outline-none focus:ring-2 focus:ring-blue-500",
                            value: "{editing_content}",
                            oninput: move |e| editing_content.set(e.value()),
                        }

                        // Variables help
                        div { class: "mt-4 p-4 bg-gray-50 rounded-lg",
                            h4 { class: "font-medium mb-2", "Available Variables" }
                            div { class: "grid grid-cols-2 gap-2 text-sm",
                                for var in prompt.variables.iter() {
                                    div { class: "font-mono text-blue-600", "{{{{{var}}}}}" }
                                    div { class: "text-gray-600", "{var}" }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "flex-1 flex items-center justify-center text-gray-500",
                        "Select a prompt to edit"
                    }
                }
            }

            // Create modal
            if *show_create_modal.read() {
                CreatePromptModal {
                    on_close: move |_| show_create_modal.set(false),
                    on_created: move |p: PromptTemplate| {
                        prompts.write().push(p);
                        show_create_modal.set(false);
                    },
                }
            }
        }
    }
}

#[component]
fn PromptListItem(
    prompt: PromptTemplate,
    is_selected: bool,
    on_select: EventHandler<String>,
) -> Element {
    let prompt_id = prompt.id.clone();

    rsx! {
        div {
            class: "p-3 rounded-lg cursor-pointer mb-2 transition-colors",
            class: if is_selected { "bg-blue-100" } else { "hover:bg-gray-100" },
            onclick: move |_| on_select.call(prompt_id.clone()),
            div { class: "flex items-center justify-between",
                span { class: "font-medium", "{prompt.name}" }
            }
            div { class: "text-sm text-gray-500 mt-1", "{prompt.category}" }
        }
    }
}

#[component]
fn CreatePromptModal(
    on_close: EventHandler<MouseEvent>,
    on_created: EventHandler<PromptTemplate>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut category = use_signal(|| "Sales".to_string());
    let mut content = use_signal(String::new);
    let mut is_creating = use_signal(|| false);

    let create = move |_| {
        if name().is_empty() {
            return;
        }

        is_creating.set(true);
        let template = PromptTemplate {
            id: format!("prompt_{}", chrono::Utc::now().timestamp()),
            name: name(),
            content: content(),
            category: category(),
            variables: vec!["lead.name".to_string(), "lead.company".to_string(), "agent.name".to_string()],
        };

        spawn(async move {
            match ai::create_template(template.clone()).await {
                Ok(created) => {
                    on_created.call(created);
                    crate::state::show_notification("Prompt created", crate::state::NotificationType::Success);
                }
                Err(e) => {
                    tracing::error!("Failed to create prompt: {}", e);
                    crate::state::show_notification("Failed to create prompt", crate::state::NotificationType::Error);
                }
            }
            is_creating.set(false);
        });
    };

    rsx! {
        div { class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
            div { class: "bg-white rounded-lg p-6 w-full max-w-lg",
                h3 { class: "text-lg font-semibold mb-4", "Create New Prompt" }

                div { class: "space-y-4",
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Name *" }
                        input {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                            r#type: "text",
                            placeholder: "Prompt name",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Category" }
                        select {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-lg",
                            onchange: move |e| category.set(e.value()),
                            option { value: "Sales", "Sales" }
                            option { value: "Follow-up", "Follow-up" }
                            option { value: "Support", "Support" }
                            option { value: "Survey", "Survey" }
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Content" }
                        textarea {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-lg h-32",
                            placeholder: "Enter the AI prompt content...",
                            value: "{content}",
                            oninput: move |e| content.set(e.value()),
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
