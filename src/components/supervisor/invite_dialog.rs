use dioxus::prelude::*;
use crate::models::{UserRole, InviteUserResponse};
use crate::api;

#[component]
pub fn InviteUserDialog(
    on_close: EventHandler<MouseEvent>,
    on_invited: EventHandler<InviteUserResponse>,
) -> Element {
    let mut email = use_signal(String::new);
    let mut role = use_signal(|| UserRole::Agent);
    let mut is_inviting = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    let mut success_message = use_signal(|| None::<String>);

    let invite = move |_| {
        // Clear previous messages
        error_message.set(None);
        success_message.set(None);

        // Validate email
        let email_value = email();
        if email_value.is_empty() {
            error_message.set(Some("Email is required".to_string()));
            return;
        }

        // Basic email format validation
        if !email_value.contains('@') || !email_value.contains('.') {
            error_message.set(Some("Please enter a valid email address".to_string()));
            return;
        }

        is_inviting.set(true);
        let invite_role = role();

        spawn(async move {
            match api::auth::invite_user(&email_value, invite_role).await {
                Ok(response) => {
                    success_message.set(Some(format!("Invitation sent to {}", response.email)));
                    // Clear form
                    email.set(String::new());
                    role.set(UserRole::Agent);
                    // Notify parent
                    on_invited.call(response);
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to send invitation: {}", e)));
                }
            }
            is_inviting.set(false);
        });
    };

    rsx! {
        div { class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
            div { class: "bg-white rounded-lg p-6 w-full max-w-md",
                h3 { class: "text-lg font-semibold mb-4", "Invite User" }

                div { class: "space-y-4",
                    // Email field
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Email *" }
                        input {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500",
                            r#type: "email",
                            placeholder: "user@example.com",
                            value: "{email}",
                            oninput: move |e| {
                                email.set(e.value());
                                error_message.set(None);
                            },
                        }
                    }

                    // Role selection
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Role" }
                        div { class: "flex gap-4",
                            label { class: "flex items-center gap-2 cursor-pointer",
                                input {
                                    r#type: "radio",
                                    name: "role",
                                    checked: role() == UserRole::Agent,
                                    onchange: move |_| role.set(UserRole::Agent),
                                }
                                span { "Agent" }
                            }
                            label { class: "flex items-center gap-2 cursor-pointer",
                                input {
                                    r#type: "radio",
                                    name: "role",
                                    checked: role() == UserRole::Supervisor,
                                    onchange: move |_| role.set(UserRole::Supervisor),
                                }
                                span { "Supervisor" }
                            }
                        }
                    }

                    // Info message
                    div { class: "bg-blue-50 p-3 rounded-lg text-sm text-blue-700",
                        "An invitation email will be sent to the user with a registration link. The invitation will expire in 7 days."
                    }

                    // Error message
                    if let Some(error) = error_message() {
                        div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm",
                            "{error}"
                        }
                    }

                    // Success message
                    if let Some(success) = success_message() {
                        div { class: "bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg text-sm",
                            "{success}"
                        }
                    }
                }

                // Action buttons
                div { class: "flex justify-end gap-2 mt-6",
                    button {
                        class: "px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg",
                        onclick: move |e| on_close.call(e),
                        disabled: *is_inviting.read(),
                        "Cancel"
                    }
                    button {
                        class: "px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed",
                        disabled: email().is_empty() || *is_inviting.read(),
                        onclick: invite,
                        if *is_inviting.read() { "Sending..." } else { "Send Invitation" }
                    }
                }
            }
        }
    }
}
