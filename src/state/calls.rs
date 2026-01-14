use dioxus::prelude::*;
use crate::models::{Call, CallStatus, Lead};

/// Global call state
pub static CALL_STATE: GlobalSignal<CallState> = Signal::global(CallState::default);

#[derive(Clone, Default)]
pub struct CallState {
    pub current_call: Option<Call>,
    pub current_lead: Option<Lead>,
    pub current_call_id: Option<i64>,
    pub dialed_number: Option<String>,
    pub is_dialing: bool,
    pub is_ringing: bool,
    pub is_answered: bool,
    pub call_duration: u32,
    pub is_muted: bool,
    pub is_on_hold: bool,
}

impl CallState {
    pub fn is_in_call(&self) -> bool {
        self.current_call.as_ref()
            .map(|c| c.status.is_active())
            .unwrap_or(false)
    }

    pub fn call_status(&self) -> Option<&CallStatus> {
        self.current_call.as_ref().map(|c| &c.status)
    }

    pub fn lead_name(&self) -> Option<String> {
        self.current_lead.as_ref().map(|l| l.full_name())
    }

    pub fn phone_number(&self) -> Option<&str> {
        self.current_lead.as_ref().map(|l| l.phone.as_str())
    }
}

pub fn end_call() {
    let mut state = CALL_STATE.write();
    state.current_call = None;
    state.current_lead = None;
    state.current_call_id = None;
    state.dialed_number = None;
    state.is_dialing = false;
    state.is_ringing = false;
    state.is_answered = false;
    state.call_duration = 0;
    state.is_muted = false;
    state.is_on_hold = false;
}

pub fn toggle_mute() {
    let mut state = CALL_STATE.write();
    state.is_muted = !state.is_muted;
}

pub fn toggle_hold() {
    let mut state = CALL_STATE.write();
    state.is_on_hold = !state.is_on_hold;
}

// Functions only used in wasm32 builds
#[cfg(target_arch = "wasm32")]
pub fn set_ringing(call_id: i64, phone_number: String) {
    let mut state = CALL_STATE.write();
    state.current_call_id = Some(call_id);
    state.dialed_number = Some(phone_number);
    state.is_dialing = false;
    state.is_ringing = true;
    state.is_answered = false;
}

#[cfg(target_arch = "wasm32")]
pub fn set_answered() {
    let mut state = CALL_STATE.write();
    state.is_dialing = false;
    state.is_ringing = false;
    state.is_answered = true;
    state.call_duration = 0;
}

#[cfg(target_arch = "wasm32")]
pub fn increment_duration() {
    CALL_STATE.write().call_duration += 1;
}
