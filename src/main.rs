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
    rsx! {
        // Global styles
        style { {include_str!("../assets/styles.css")} }

        // Notification toast
        Notification {}

        // Router wraps everything so Link components work everywhere
        Router::<Route> {}
    }
}

/// Layout component that wraps all authenticated routes
#[component]
pub fn AppLayout() -> Element {
    let auth_state = AUTH_STATE.read();
    let nav = use_navigator();

    // Redirect to login if not authenticated
    if !auth_state.is_authenticated() {
        nav.push(Route::Login {});
        return rsx! { div { "Redirecting to login..." } };
    }

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
pub fn LoginPage() -> Element {
    let nav = use_navigator();
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut is_unverified_error = use_signal(|| false);
    let mut resend_email = use_signal(String::new);
    let mut resend_loading = use_signal(|| false);
    let mut resend_success = use_signal(|| None::<String>);

    let mut login = move |_| {
        let user = username();
        let pass = password();

        if user.is_empty() || pass.is_empty() {
            error.set(Some("Please enter username and password".to_string()));
            is_unverified_error.set(false);
            return;
        }

        is_loading.set(true);
        error.set(None);
        is_unverified_error.set(false);
        resend_success.set(None);

        spawn(async move {
            match api::auth::login(&user, &pass).await {
                Ok(response) => {
                    state::set_auth(response.user, response.token);
                    // Navigate to home after successful login
                    nav.push(Route::Home {});
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    // Check if this is an unverified email error
                    if error_msg.contains("verify your email") || error_msg.contains("not verified") {
                        is_unverified_error.set(true);
                    }
                    error.set(Some(format!("Login failed: {}", e)));
                }
            }
            is_loading.set(false);
        });
    };

    let mut resend_verification = move |_| {
        let email = resend_email();

        if email.is_empty() {
            error.set(Some("Please enter your email address".to_string()));
            return;
        }

        resend_loading.set(true);
        error.set(None);
        resend_success.set(None);

        spawn(async move {
            match api::auth::resend_verification(&email).await {
                Ok(response) => {
                    resend_success.set(Some(response.message));
                    resend_email.set(String::new());
                }
                Err(e) => {
                    error.set(Some(format!("Failed to resend verification: {}", e)));
                }
            }
            resend_loading.set(false);
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

                // Resend verification form (shown when unverified email error)
                if *is_unverified_error.read() {
                    div { class: "mt-6 p-4 bg-yellow-50 border border-yellow-200 rounded-lg",
                        h3 { class: "text-sm font-medium text-yellow-800 mb-2", "Email Not Verified" }
                        p { class: "text-sm text-yellow-700 mb-4",
                            "Please verify your email address before logging in. Check your inbox for a verification link."
                        }

                        // Success message for resend
                        if let Some(msg) = resend_success.read().as_ref() {
                            div { class: "bg-green-100 border border-green-400 text-green-700 px-3 py-2 rounded mb-3 text-sm",
                                "{msg}"
                            }
                        }

                        // Resend verification form
                        form {
                            onsubmit: move |e| {
                                e.prevent_default();
                                resend_verification(e);
                            },

                            div { class: "flex gap-2",
                                input {
                                    class: "flex-1 px-3 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-yellow-500 text-sm",
                                    r#type: "email",
                                    placeholder: "Enter your email",
                                    value: "{resend_email}",
                                    oninput: move |e| resend_email.set(e.value()),
                                }
                                button {
                                    class: "px-4 py-2 bg-yellow-600 text-white rounded hover:bg-yellow-700 font-medium disabled:opacity-50 text-sm",
                                    r#type: "submit",
                                    disabled: *resend_loading.read(),
                                    if *resend_loading.read() { "Sending..." } else { "Resend Email" }
                                }
                            }
                        }
                    }
                }

                // Demo credentials
                div { class: "mt-6 text-center text-sm text-gray-500",
                    p { "Demo credentials:" }
                    p { class: "font-mono", "admin / admin123" }
                }

                // Registration link
                div { class: "mt-6 text-center text-sm",
                    span { class: "text-gray-600", "Don't have an account? " }
                    Link {
                        to: Route::Register {},
                        class: "text-blue-600 hover:text-blue-700 font-medium",
                        "Create one"
                    }
                }
            }
        }
    }
}

#[component]
pub fn RegistrationPage() -> Element {
    let mut email = use_signal(String::new);
    let mut username = use_signal(String::new);
    let mut first_name = use_signal(String::new);
    let mut last_name = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| None::<String>);

    // Client-side validation helper
    let validate_email = |email: &str| -> bool {
        email.contains('@') && email.contains('.') && email.len() > 5
    };

    let validate_password_strength = |password: &str| -> Result<(), String> {
        if password.len() < 8 {
            return Err("Password must be at least 8 characters".to_string());
        }
        if !password.chars().any(|c| c.is_numeric()) {
            return Err("Password must contain at least one number".to_string());
        }
        if !password.chars().any(|c| c.is_alphabetic()) {
            return Err("Password must contain at least one letter".to_string());
        }
        Ok(())
    };

    let mut register = move |_| {
        let email_val = email();
        let username_val = username();
        let first_name_val = first_name();
        let last_name_val = last_name();
        let password_val = password();
        let confirm_password_val = confirm_password();

        // Clear previous messages
        error.set(None);
        success.set(None);

        // Validate all fields
        if email_val.is_empty() || username_val.is_empty() || password_val.is_empty() {
            error.set(Some("Please fill in all required fields".to_string()));
            return;
        }

        // Validate email format
        if !validate_email(&email_val) {
            error.set(Some("Please enter a valid email address".to_string()));
            return;
        }

        // Validate password strength
        if let Err(e) = validate_password_strength(&password_val) {
            error.set(Some(e));
            return;
        }

        // Validate password match
        if password_val != confirm_password_val {
            error.set(Some("Passwords do not match".to_string()));
            return;
        }

        is_loading.set(true);

        spawn(async move {
            match api::auth::register(&username_val, &email_val, &password_val).await {
                Ok(response) => {
                    success.set(Some(response.message));
                    // Clear form on success
                    email.set(String::new());
                    username.set(String::new());
                    first_name.set(String::new());
                    last_name.set(String::new());
                    password.set(String::new());
                    confirm_password.set(String::new());
                }
                Err(e) => {
                    error.set(Some(format!("Registration failed: {}", e)));
                }
            }
            is_loading.set(false);
        });
    };

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-gray-100 py-8",
            div { class: "bg-white rounded-lg shadow-lg p-8 w-full max-w-md",
                // Logo
                div { class: "text-center mb-6",
                    span { class: "text-5xl", "\u{1F4DE}" }
                    h1 { class: "text-2xl font-bold mt-4", "VoIP CRM" }
                    p { class: "text-gray-500", "Create your account" }
                }

                // Success message
                if let Some(msg) = success.read().as_ref() {
                    div { class: "bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded mb-4",
                        p { class: "font-medium", "✓ Registration successful!" }
                        p { class: "text-sm mt-1", "{msg}" }
                    }
                }

                // Error message
                if let Some(err) = error.read().as_ref() {
                    div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                        "{err}"
                    }
                }

                // Registration Form
                form {
                    onsubmit: move |e| {
                        e.prevent_default();
                        register(e);
                    },

                    // Email field
                    div { class: "mb-4",
                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                            "Email ",
                            span { class: "text-red-500", "*" }
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "email",
                            placeholder: "your.email@example.com",
                            value: "{email}",
                            oninput: move |e| email.set(e.value()),
                            disabled: *is_loading.read(),
                        }
                    }

                    // Username field
                    div { class: "mb-4",
                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                            "Username ",
                            span { class: "text-red-500", "*" }
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "text",
                            placeholder: "Choose a username",
                            value: "{username}",
                            oninput: move |e| username.set(e.value()),
                            disabled: *is_loading.read(),
                        }
                    }

                    // First Name field
                    div { class: "mb-4",
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "First Name" }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "text",
                            placeholder: "Your first name (optional)",
                            value: "{first_name}",
                            oninput: move |e| first_name.set(e.value()),
                            disabled: *is_loading.read(),
                        }
                    }

                    // Last Name field
                    div { class: "mb-4",
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Last Name" }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "text",
                            placeholder: "Your last name (optional)",
                            value: "{last_name}",
                            oninput: move |e| last_name.set(e.value()),
                            disabled: *is_loading.read(),
                        }
                    }

                    // Password field
                    div { class: "mb-4",
                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                            "Password ",
                            span { class: "text-red-500", "*" }
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "password",
                            placeholder: "At least 8 characters",
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                            disabled: *is_loading.read(),
                        }
                        p { class: "text-xs text-gray-500 mt-1",
                            "Must be at least 8 characters with letters and numbers"
                        }
                    }

                    // Confirm Password field
                    div { class: "mb-6",
                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                            "Confirm Password ",
                            span { class: "text-red-500", "*" }
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "password",
                            placeholder: "Re-enter your password",
                            value: "{confirm_password}",
                            oninput: move |e| confirm_password.set(e.value()),
                            disabled: *is_loading.read(),
                        }
                    }

                    // Submit button
                    button {
                        class: "w-full py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 font-medium disabled:opacity-50 transition-colors",
                        r#type: "submit",
                        disabled: *is_loading.read(),
                        if *is_loading.read() { "Creating account..." } else { "Create Account" }
                    }
                }

                // Link to login page
                div { class: "mt-6 text-center text-sm",
                    span { class: "text-gray-600", "Already have an account? " }
                    Link {
                        to: Route::Login {},
                        class: "text-blue-600 hover:text-blue-700 font-medium",
                        "Sign in"
                    }
                }
            }
        }
    }
}

#[component]
pub fn VerifyEmailPage(token: String) -> Element {
    let mut status = use_signal(|| "verifying".to_string()); // verifying, success, error
    let mut message = use_signal(|| "Verifying your email address...".to_string());
    let mut email = use_signal(String::new);
    let mut resend_loading = use_signal(|| false);
    let mut resend_message = use_signal(|| None::<String>);

    // Verify email on mount
    use_effect(move || {
        let token_clone = token.clone();
        spawn(async move {
            match api::auth::verify_email(&token_clone).await {
                Ok(response) => {
                    // Auto-login happens in the API call (token is stored)
                    state::set_auth(response.user.clone(), response.token);
                    status.set("success".to_string());
                    message.set("Email verified successfully! Redirecting to dashboard...".to_string());

                    // Auto-redirect after 2 seconds
                    #[cfg(target_arch = "wasm32")]
                    {
                        use gloo_timers::future::TimeoutFuture;
                        TimeoutFuture::new(2000).await;
                        // Navigation will happen automatically when AUTH_STATE updates
                    }
                }
                Err(e) => {
                    status.set("error".to_string());
                    message.set(format!("Verification failed: {}", e));
                }
            }
        });
    });

    let mut resend_verification = move |_| {
        let email_val = email();

        if email_val.is_empty() {
            resend_message.set(Some("Please enter your email address".to_string()));
            return;
        }

        resend_loading.set(true);
        resend_message.set(None);

        spawn(async move {
            match api::auth::resend_verification(&email_val).await {
                Ok(response) => {
                    resend_message.set(Some(response.message));
                    email.set(String::new());
                }
                Err(e) => {
                    resend_message.set(Some(format!("Failed to resend: {}", e)));
                }
            }
            resend_loading.set(false);
        });
    };

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-gray-100",
            div { class: "bg-white rounded-lg shadow-lg p-8 w-full max-w-md",
                // Logo
                div { class: "text-center mb-8",
                    span { class: "text-5xl", "\u{1F4DE}" }
                    h1 { class: "text-2xl font-bold mt-4", "Email Verification" }
                }

                // Status message
                if status() == "verifying" {
                    div { class: "text-center py-8",
                        div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4" }
                        p { class: "text-gray-600", "{message()}" }
                    }
                } else if status() == "success" {
                    div { class: "bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded mb-4",
                        p { class: "font-medium", "✓ {message()}" }
                    }
                } else if status() == "error" {
                    div {
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-6",
                            p { class: "font-medium", "✗ {message()}" }
                            p { class: "text-sm mt-2", "The verification link may have expired or is invalid." }
                        }

                        // Resend verification form
                        div { class: "border-t pt-6",
                            h2 { class: "text-lg font-semibold mb-4", "Resend Verification Email" }

                            if let Some(msg) = resend_message.read().as_ref() {
                                div {
                                    class: if msg.contains("success") || msg.contains("sent") {
                                        "bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded mb-4"
                                    } else {
                                        "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4"
                                    },
                                    "{msg}"
                                }
                            }

                            form {
                                onsubmit: move |e| {
                                    e.prevent_default();
                                    resend_verification(());
                                },
                                div { class: "mb-4",
                                    label { class: "block text-gray-700 mb-2", "Email Address" }
                                    input {
                                        r#type: "email",
                                        class: "w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                                        placeholder: "your@email.com",
                                        value: "{email()}",
                                        oninput: move |e| email.set(e.value().clone()),
                                        disabled: resend_loading(),
                                    }
                                }

                                button {
                                    r#type: "submit",
                                    class: "w-full bg-blue-600 text-white py-2 rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors",
                                    disabled: resend_loading(),
                                    if resend_loading() {
                                        "Sending..."
                                    } else {
                                        "Resend Verification Email"
                                    }
                                }
                            }
                        }
                    }
                }

                // Link to login
                div { class: "text-center mt-6 pt-6 border-t",
                    p { class: "text-gray-600",
                        Link { to: Route::Login {}, class: "text-blue-600 hover:underline", "Back to Login" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AcceptInvitationPage(token: String) -> Element {
    let mut status = use_signal(|| "loading".to_string()); // loading, loaded, success, error
    let mut message = use_signal(|| "Loading invitation details...".to_string());
    let mut email = use_signal(String::new);
    let mut role = use_signal(|| "Agent".to_string());
    let mut invited_by = use_signal(String::new);
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Clone token for use in multiple closures
    let token_for_effect = token.clone();
    let token_for_accept = token.clone();

    // Fetch invitation details on mount
    use_effect(move || {
        let token_clone = token_for_effect.clone();
        spawn(async move {
            match api::auth::get_invitation_details(&token_clone).await {
                Ok(details) => {
                    if details.valid {
                        email.set(details.email);
                        role.set(format!("{:?}", details.role));
                        invited_by.set(details.invited_by_username);
                        status.set("loaded".to_string());
                        message.set(String::new());
                    } else {
                        status.set("error".to_string());
                        message.set("This invitation has expired or has already been used.".to_string());
                    }
                }
                Err(e) => {
                    status.set("error".to_string());
                    message.set(format!("Failed to load invitation: {}", e));
                }
            }
        });
    });

    // Client-side validation helper
    let validate_password_strength = |password: &str| -> Result<(), String> {
        if password.len() < 8 {
            return Err("Password must be at least 8 characters".to_string());
        }
        if !password.chars().any(|c| c.is_numeric()) {
            return Err("Password must contain at least one number".to_string());
        }
        if !password.chars().any(|c| c.is_alphabetic()) {
            return Err("Password must contain at least one letter".to_string());
        }
        Ok(())
    };

    let mut accept_invite = move |_| {
        let username_val = username();
        let password_val = password();
        let confirm_password_val = confirm_password();
        let token_val = token_for_accept.clone();

        // Clear previous messages
        error.set(None);

        // Validate all fields
        if username_val.is_empty() || password_val.is_empty() {
            error.set(Some("Please fill in all required fields".to_string()));
            return;
        }

        // Validate password strength
        if let Err(e) = validate_password_strength(&password_val) {
            error.set(Some(e));
            return;
        }

        // Validate password match
        if password_val != confirm_password_val {
            error.set(Some("Passwords do not match".to_string()));
            return;
        }

        is_loading.set(true);

        spawn(async move {
            match api::auth::accept_invitation(&token_val, &username_val, &password_val).await {
                Ok(response) => {
                    // Auto-login happens in the API call (token is stored)
                    state::set_auth(response.user.clone(), response.token);
                    status.set("success".to_string());
                    message.set("Account created successfully! Redirecting to dashboard...".to_string());

                    // Auto-redirect after 2 seconds
                    #[cfg(target_arch = "wasm32")]
                    {
                        use gloo_timers::future::TimeoutFuture;
                        TimeoutFuture::new(2000).await;
                        // Navigation will happen automatically when AUTH_STATE updates
                    }
                }
                Err(e) => {
                    error.set(Some(format!("Registration failed: {}", e)));
                    is_loading.set(false);
                }
            }
        });
    };

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-gray-100",
            div { class: "bg-white rounded-lg shadow-lg p-8 w-full max-w-md",
                // Logo
                div { class: "text-center mb-8",
                    span { class: "text-5xl", "\u{1F4DE}" }
                    h1 { class: "text-2xl font-bold mt-4", "Accept Invitation" }
                }

                // Loading state
                if status() == "loading" {
                    div { class: "text-center py-8",
                        div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4" }
                        p { class: "text-gray-600", "{message()}" }
                    }
                }
                // Success state
                else if status() == "success" {
                    div { class: "bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded mb-4",
                        p { class: "font-medium", "✓ {message()}" }
                    }
                }
                // Error state
                else if status() == "error" {
                    div {
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-6",
                            p { class: "font-medium", "✗ {message()}" }
                        }

                        // Link to login
                        div { class: "text-center mt-6",
                            p { class: "text-gray-600",
                                Link { to: Route::Login {}, class: "text-blue-600 hover:underline", "Back to Login" }
                            }
                        }
                    }
                }
                // Loaded state - show registration form
                else if status() == "loaded" {
                    div {
                        // Invitation info
                        div { class: "bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6",
                            p { class: "text-sm text-gray-700 mb-2",
                                "You've been invited by "
                                span { class: "font-semibold", "{invited_by()}" }
                                " to join as "
                                span { class: "font-semibold", "{role()}" }
                            }
                            p { class: "text-sm text-gray-600",
                                "Email: "
                                span { class: "font-medium", "{email()}" }
                            }
                        }

                        // Error message
                        if let Some(err) = error.read().as_ref() {
                            div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                                "{err}"
                            }
                        }

                        // Registration form
                        form {
                            onsubmit: move |e| {
                                e.prevent_default();
                                accept_invite(());
                            },

                            // Username field
                            div { class: "mb-4",
                                label { class: "block text-gray-700 mb-2", "Username *" }
                                input {
                                    r#type: "text",
                                    class: "w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                                    placeholder: "Choose a username",
                                    value: "{username()}",
                                    oninput: move |e| username.set(e.value().clone()),
                                    disabled: is_loading(),
                                    required: true,
                                }
                            }

                            // Password field
                            div { class: "mb-4",
                                label { class: "block text-gray-700 mb-2", "Password *" }
                                input {
                                    r#type: "password",
                                    class: "w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                                    placeholder: "At least 8 characters",
                                    value: "{password()}",
                                    oninput: move |e| password.set(e.value().clone()),
                                    disabled: is_loading(),
                                    required: true,
                                }
                                p { class: "text-xs text-gray-500 mt-1",
                                    "Must be at least 8 characters with letters and numbers"
                                }
                            }

                            // Confirm password field
                            div { class: "mb-6",
                                label { class: "block text-gray-700 mb-2", "Confirm Password *" }
                                input {
                                    r#type: "password",
                                    class: "w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                                    placeholder: "Re-enter your password",
                                    value: "{confirm_password()}",
                                    oninput: move |e| confirm_password.set(e.value().clone()),
                                    disabled: is_loading(),
                                    required: true,
                                }
                            }

                            // Submit button
                            button {
                                r#type: "submit",
                                class: "w-full bg-blue-600 text-white py-2 rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors font-medium",
                                disabled: is_loading(),
                                if is_loading() {
                                    "Creating Account..."
                                } else {
                                    "Accept Invitation & Create Account"
                                }
                            }
                        }

                        // Link to login
                        div { class: "text-center mt-6 pt-6 border-t",
                            p { class: "text-gray-600 text-sm",
                                "Already have an account? "
                                Link { to: Route::Login {}, class: "text-blue-600 hover:underline", "Login" }
                            }
                        }
                    }
                }
            }
        }
    }
}
