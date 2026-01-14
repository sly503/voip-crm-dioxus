//! VoIP CRM - Full Stack Dioxus Application
//!
//! A VoIP CRM application built with Dioxus for managing calls,
//! leads, campaigns, and AI agents.
//!
//! Runs in fullstack mode with Axum backend and Dioxus frontend.

mod components;
mod models;
mod routes;
mod state;
mod api;

#[cfg(not(target_arch = "wasm32"))]
mod server;

use dioxus::prelude::*;
use routes::Route;
use state::AUTH_STATE;
use components::{
    phone::{CallStatusBar, SipDialer},
    common::Notification,
};

fn main() {
    // On wasm, just run the app
    #[cfg(target_arch = "wasm32")]
    {
        run_app();
    }

    // On native, handle server vs app mode
    #[cfg(not(target_arch = "wasm32"))]
    {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("voip_crm=info".parse().unwrap()))
            .init();

        // Load environment variables
        dotenvy::dotenv().ok();

        // Determine run mode
        let args: Vec<String> = std::env::args().collect();

        if args.contains(&"--server".to_string()) {
            // Run server only
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(run_server());
        } else {
            // Run frontend (desktop mode) with embedded server

            // Start server in background thread
            let database_url = std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://voipcrm:voipcrm123@localhost:5432/voipcrm".to_string());
            let port: u16 = std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000);

            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async {
                    tracing::info!("Starting embedded server on port {}", port);
                    if let Err(e) = server::run_server(&database_url, port).await {
                        tracing::error!("Embedded server error: {}", e);
                    }
                });
            });

            // Give server time to start
            std::thread::sleep(std::time::Duration::from_millis(500));

            // Run frontend
            run_app();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_server() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);

    tracing::info!("Starting VoIP CRM server on port {}", port);

    if let Err(e) = server::run_server(&database_url, port).await {
        tracing::error!("Server error: {}", e);
    }
}

fn run_app() {
    // Get API URL - on wasm use window location, on native use env var
    #[cfg(target_arch = "wasm32")]
    let api_url = {
        // On web, use the same origin as the page (for same-origin API requests)
        web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_else(|| "http://localhost:3000".to_string())
    };

    #[cfg(not(target_arch = "wasm32"))]
    let api_url = std::env::var("API_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Initialize API client
    api::init_api_client(&api_url);

    // Launch the Dioxus app
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let auth_state = AUTH_STATE.read();

    rsx! {
        // Global styles
        style { {include_str!("../assets/styles.css")} }

        // Notification toast
        Notification {}

        // Main content
        if auth_state.is_authenticated() {
            AuthenticatedApp {}
        } else {
            LoginPage {}
        }
    }
}

#[component]
fn AuthenticatedApp() -> Element {
    // Wrap everything in Router so Link components work
    rsx! {
        Router::<Route> {}
    }
}

/// Layout component that wraps all authenticated routes
#[component]
pub fn AppLayout() -> Element {
    let auth_state = AUTH_STATE.read();
    let is_supervisor = auth_state.is_supervisor_or_above();

    rsx! {
        div { class: "h-screen flex flex-col bg-gray-100",
            // Top bar
            TopBar {}

            // Main content area
            div { class: "flex-1 flex overflow-hidden",
                // Sidebar
                Sidebar { is_supervisor: is_supervisor }

                // Main content - Outlet renders the matched route
                div { class: "flex-1 flex overflow-hidden",
                    Outlet::<Route> {}
                }
            }

            // Call status bar (shows during calls)
            CallStatusBar {}
        }
    }
}

#[component]
fn TopBar() -> Element {
    let auth_state = AUTH_STATE.read();
    let username = auth_state.username().unwrap_or("User");

    let logout = move |_| {
        spawn(async move {
            api::auth::logout().await;
            state::clear_auth();
        });
    };

    rsx! {
        header { class: "bg-white border-b px-6 py-3 flex items-center justify-between",
            // Logo
            div { class: "flex items-center gap-3",
                span { class: "text-2xl", "\u{1F4DE}" }
                h1 { class: "text-xl font-bold text-gray-800", "VoIP CRM" }
            }

            // User menu
            div { class: "flex items-center gap-4",
                span { class: "text-gray-600", "Welcome, {username}" }
                button {
                    class: "px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg",
                    onclick: logout,
                    "Logout"
                }
            }
        }
    }
}

#[component]
fn Sidebar(is_supervisor: bool) -> Element {
    let current_route = use_route::<Route>();

    let nav_items = if is_supervisor {
        vec![
            (Route::Home {}, "Dashboard", "\u{1F4CA}"),
            (Route::Leads {}, "Leads", "\u{1F4CB}"),
            (Route::Campaigns {}, "Campaigns", "\u{1F4E2}"),
            (Route::Agents {}, "Agents", "\u{1F465}"),
            (Route::AISettings {}, "AI Settings", "\u{1F916}"),
        ]
    } else {
        vec![
            (Route::Home {}, "My Dashboard", "\u{1F3E0}"),
            (Route::Leads {}, "My Leads", "\u{1F4CB}"),
        ]
    };

    rsx! {
        nav { class: "w-64 bg-white border-r flex flex-col",
            // Navigation
            div { class: "flex-1 py-4",
                for (route, label, icon) in nav_items.iter() {
                    Link {
                        to: route.clone(),
                        class: if std::mem::discriminant(&current_route) == std::mem::discriminant(route) {
                            "flex items-center gap-3 px-6 py-3 bg-blue-50 text-blue-600 border-r-4 border-blue-600 font-medium"
                        } else {
                            "flex items-center gap-3 px-6 py-3 text-gray-700 hover:bg-gray-100 transition-colors"
                        },
                        span { class: "text-xl", "{icon}" }
                        span { "{label}" }
                    }
                }
            }

            // Phone dialer (always visible for agents)
            // SIP trunk dialer - server-side calling via DIDLogic
            div { class: "p-4 border-t",
                SipDialer {}
            }
        }
    }
}

#[component]
fn LoginPage() -> Element {
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let mut login = move |_| {
        let user = username();
        let pass = password();

        if user.is_empty() || pass.is_empty() {
            error.set(Some("Please enter username and password".to_string()));
            return;
        }

        is_loading.set(true);
        error.set(None);

        spawn(async move {
            match api::auth::login(&user, &pass).await {
                Ok(response) => {
                    state::set_auth(response.user, response.token);
                }
                Err(e) => {
                    error.set(Some(format!("Login failed: {}", e)));
                }
            }
            is_loading.set(false);
        });
    };

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-gray-100",
            div { class: "bg-white rounded-lg shadow-lg p-8 w-full max-w-md",
                // Logo
                div { class: "text-center mb-8",
                    span { class: "text-5xl", "\u{1F4DE}" }
                    h1 { class: "text-2xl font-bold mt-4", "VoIP CRM" }
                    p { class: "text-gray-500", "Sign in to continue" }
                }

                // Error message
                if let Some(err) = error.read().as_ref() {
                    div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                        "{err}"
                    }
                }

                // Form
                form {
                    onsubmit: move |e| {
                        e.prevent_default();
                        login(e);
                    },

                    div { class: "mb-4",
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Username" }
                        input {
                            class: "w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "text",
                            placeholder: "Enter your username",
                            value: "{username}",
                            oninput: move |e| username.set(e.value()),
                        }
                    }

                    div { class: "mb-6",
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Password" }
                        input {
                            class: "w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "password",
                            placeholder: "Enter your password",
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                        }
                    }

                    button {
                        class: "w-full py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 font-medium disabled:opacity-50",
                        r#type: "submit",
                        disabled: *is_loading.read(),
                        if *is_loading.read() { "Signing in..." } else { "Sign In" }
                    }
                }

                // Demo credentials
                div { class: "mt-6 text-center text-sm text-gray-500",
                    p { "Demo credentials:" }
                    p { class: "font-mono", "admin / admin123" }
                }
            }
        }
    }
}
