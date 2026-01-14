//! WebRTC integration with Telnyx
//!
//! This module provides Rust bindings for the Telnyx WebRTC JavaScript SDK.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = initTelnyxWebRTC)]
    pub async fn init_telnyx_webrtc(username: &str, password: &str) -> JsValue;

    #[wasm_bindgen(js_name = makeWebRTCCall)]
    pub async fn make_webrtc_call(destination: &str, caller_id: &str) -> JsValue;

    #[wasm_bindgen(js_name = answerWebRTCCall)]
    pub fn answer_webrtc_call() -> bool;

    #[wasm_bindgen(js_name = hangupWebRTCCall)]
    pub fn hangup_webrtc_call() -> bool;

    #[wasm_bindgen(js_name = toggleMuteWebRTC)]
    pub fn toggle_mute_webrtc() -> bool;

    #[wasm_bindgen(js_name = toggleHoldWebRTC)]
    pub fn toggle_hold_webrtc() -> bool;

    #[wasm_bindgen(js_name = sendDTMF)]
    pub fn send_dtmf(digit: &str) -> bool;

    #[wasm_bindgen(js_name = getWebRTCCallState)]
    pub fn get_webrtc_call_state() -> JsValue;

    #[wasm_bindgen(js_name = isTelnyxReady)]
    pub fn is_telnyx_ready() -> bool;

    #[wasm_bindgen(js_name = disconnectTelnyx)]
    pub fn disconnect_telnyx();
}

/// WebRTC call state
#[derive(Debug, Clone, PartialEq)]
pub enum WebRTCCallState {
    New,
    Trying,
    Ringing,
    Active,
    Held,
    Hangup,
    Destroy,
    Unknown(String),
}

impl From<&str> for WebRTCCallState {
    fn from(s: &str) -> Self {
        match s {
            "new" => WebRTCCallState::New,
            "trying" => WebRTCCallState::Trying,
            "ringing" => WebRTCCallState::Ringing,
            "active" => WebRTCCallState::Active,
            "held" => WebRTCCallState::Held,
            "hangup" => WebRTCCallState::Hangup,
            "destroy" => WebRTCCallState::Destroy,
            _ => WebRTCCallState::Unknown(s.to_string()),
        }
    }
}

impl WebRTCCallState {
    pub fn display_name(&self) -> &str {
        match self {
            WebRTCCallState::New => "Initiating...",
            WebRTCCallState::Trying => "Dialing...",
            WebRTCCallState::Ringing => "Ringing...",
            WebRTCCallState::Active => "Connected",
            WebRTCCallState::Held => "On Hold",
            WebRTCCallState::Hangup => "Call Ended",
            WebRTCCallState::Destroy => "Call Ended",
            WebRTCCallState::Unknown(_) => "Unknown",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, WebRTCCallState::Active | WebRTCCallState::Held)
    }

    pub fn is_ringing(&self) -> bool {
        matches!(self, WebRTCCallState::New | WebRTCCallState::Trying | WebRTCCallState::Ringing)
    }
}
