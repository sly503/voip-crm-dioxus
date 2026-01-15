use dioxus::prelude::*;

use crate::components::{
    leads::{LeadList, LeadDetails},
    supervisor::{CampaignList, AgentList, SupervisorDashboard},
    ai::PromptEditor,
};
use crate::state::{AUTH_STATE, UI_STATE};
use crate::AppLayout;

#[derive(Routable, Clone, PartialEq, Debug)]
#[rustfmt::skip]
pub enum Route {
    // All routes use AppLayout which includes Sidebar and TopBar
    #[layout(AppLayout)]
        #[route("/")]
        Home {},

        #[route("/leads")]
        Leads {},

        #[route("/campaigns")]
        Campaigns {},

        #[route("/agents")]
        Agents {},

        #[route("/ai-settings")]
        AISettings {},

        #[route("/settings")]
        Settings {},
    #[end_layout]

    // Login and Register are outside the layout (no sidebar/topbar)
    #[route("/login")]
    Login {},

    #[route("/register")]
    Register {},

    #[route("/verify-email?:token")]
    VerifyEmail { token: String },

    #[route("/accept-invitation?:token")]
    AcceptInvitation { token: String },
}

// Route handler components
#[component]
fn Home() -> Element {
    let auth_state = AUTH_STATE.read();

    if auth_state.is_supervisor_or_above() {
        rsx! { SupervisorDashboard {} }
    } else {
        rsx! { AgentDashboard {} }
    }
}

#[component]
fn AgentDashboard() -> Element {
    let selected_lead = UI_STATE.read().selected_lead_id;

    rsx! {
        div { class: "flex-1 flex",
            div { class: "w-96 border-r bg-white",
                LeadList {}
            }
            div { class: "flex-1 bg-gray-50",
                if selected_lead.is_some() {
                    LeadDetails {}
                } else {
                    div { class: "h-full flex flex-col items-center justify-center text-gray-500",
                        span { class: "text-4xl mb-4", "\u{1F4DE}" }
                        p { "Select a lead to view details and call" }
                    }
                }
            }
        }
    }
}

#[component]
fn Login() -> Element {
    rsx! {
        crate::LoginPage {}
    }
}

#[component]
fn Register() -> Element {
    rsx! {
        crate::RegistrationPage {}
    }
}

#[component]
fn VerifyEmail(token: String) -> Element {
    rsx! {
        crate::VerifyEmailPage { token }
    }
}

#[component]
fn AcceptInvitation(token: String) -> Element {
    rsx! {
        crate::AcceptInvitationPage { token }
    }
}

#[component]
fn Leads() -> Element {
    let selected_lead = UI_STATE.read().selected_lead_id;

    rsx! {
        div { class: "flex-1 flex",
            div { class: "w-96 border-r bg-white",
                LeadList {}
            }
            div { class: "flex-1 bg-gray-50",
                if selected_lead.is_some() {
                    LeadDetails {}
                } else {
                    div { class: "h-full flex items-center justify-center text-gray-500",
                        "Select a lead to view details"
                    }
                }
            }
        }
    }
}

#[component]
fn Campaigns() -> Element {
    rsx! {
        div { class: "flex-1 bg-white",
            CampaignList {}
        }
    }
}

#[component]
fn Agents() -> Element {
    rsx! {
        div { class: "flex-1 bg-white",
            AgentList {}
        }
    }
}

#[component]
fn AISettings() -> Element {
    let mut view = use_signal(|| "settings");

    rsx! {
        div { class: "flex-1 flex flex-col",
            div { class: "bg-white border-b px-6",
                div { class: "flex gap-4",
                    button {
                        class: "py-4 px-2 border-b-2 transition-colors",
                        class: if view() == "settings" { "border-blue-600 text-blue-600" } else { "border-transparent text-gray-500 hover:text-gray-700" },
                        onclick: move |_| view.set("settings"),
                        "Settings"
                    }
                    button {
                        class: "py-4 px-2 border-b-2 transition-colors",
                        class: if view() == "prompts" { "border-blue-600 text-blue-600" } else { "border-transparent text-gray-500 hover:text-gray-700" },
                        onclick: move |_| view.set("prompts"),
                        "Prompts"
                    }
                }
            }
            div { class: "flex-1 bg-gray-50",
                match view() {
                    "prompts" => rsx! { PromptEditor {} },
                    _ => rsx! { crate::components::ai::AISettings {} },
                }
            }
        }
    }
}

#[component]
fn Settings() -> Element {
    rsx! {
        div { class: "flex-1 p-6",
            h1 { class: "text-2xl font-bold mb-6", "Settings" }
            p { class: "text-gray-500", "Application settings coming soon..." }
        }
    }
}
