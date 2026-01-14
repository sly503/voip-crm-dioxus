use dioxus::prelude::*;

/// Global UI state
pub static UI_STATE: GlobalSignal<UiState> = Signal::global(UiState::default);

#[derive(Clone, Default)]
pub struct UiState {
    pub selected_lead_id: Option<i64>,
    pub notification: Option<Notification>,
}

#[derive(Clone)]
pub struct Notification {
    pub message: String,
    pub notification_type: NotificationType,
}

#[derive(Clone, PartialEq)]
pub enum NotificationType {
    Success,
    Error,
    Warning,
    Info,
}

impl NotificationType {
    pub fn color_class(&self) -> &str {
        match self {
            NotificationType::Success => "bg-green-500",
            NotificationType::Error => "bg-red-500",
            NotificationType::Warning => "bg-yellow-500",
            NotificationType::Info => "bg-blue-500",
        }
    }
}

pub fn select_lead(lead_id: Option<i64>) {
    UI_STATE.write().selected_lead_id = lead_id;
}

pub fn show_notification(message: &str, notification_type: NotificationType) {
    UI_STATE.write().notification = Some(Notification {
        message: message.to_string(),
        notification_type,
    });
}

pub fn clear_notification() {
    UI_STATE.write().notification = None;
}
