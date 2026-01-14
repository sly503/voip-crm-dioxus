//! WebRTC state management

use dioxus::prelude::*;

/// Global WebRTC state
pub static WEBRTC_STATE: GlobalSignal<WebRTCState> = Signal::global(WebRTCState::default);

#[derive(Clone, Default)]
pub struct WebRTCState {
    pub is_connecting: bool,
    pub is_connected: bool,
    pub call_state: Option<String>,
    pub is_in_call: bool,
}

// Functions only used in wasm32 builds
#[cfg(target_arch = "wasm32")]
pub fn set_webrtc_connecting() {
    let mut state = WEBRTC_STATE.write();
    state.is_connecting = true;
}

#[cfg(target_arch = "wasm32")]
pub fn set_webrtc_connected() {
    let mut state = WEBRTC_STATE.write();
    state.is_connecting = false;
    state.is_connected = true;
}

#[cfg(target_arch = "wasm32")]
pub fn set_webrtc_error(_error: String) {
    let mut state = WEBRTC_STATE.write();
    state.is_connecting = false;
    state.is_connected = false;
}

#[cfg(target_arch = "wasm32")]
pub fn set_webrtc_call_state(call_state: String) {
    let mut state = WEBRTC_STATE.write();
    state.call_state = Some(call_state.clone());
    state.is_in_call = matches!(call_state.as_str(), "active" | "ringing" | "trying" | "new" | "held");
}

#[cfg(target_arch = "wasm32")]
pub fn clear_webrtc_call() {
    let mut state = WEBRTC_STATE.write();
    state.call_state = None;
    state.is_in_call = false;
}
