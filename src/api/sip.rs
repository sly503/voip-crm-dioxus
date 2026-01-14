//! SIP trunk API functions

use serde::{Deserialize, Serialize};
use super::client::{api_client, ApiError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SipStatus {
    pub status: String,
    pub registered: bool,
    pub trunk_host: Option<String>,
    pub caller_id: Option<String>,
}

impl SipStatus {
    pub fn is_ready(&self) -> bool {
        self.registered
    }

    pub fn display_status(&self) -> &str {
        match self.status.as_str() {
            "registered" => "Phone Ready",
            "registering" => "Connecting...",
            "connecting" => "Connecting...",
            "failed" => "Connection Failed",
            "not_configured" => "Not Configured",
            _ => "Disconnected",
        }
    }
}

/// Fetch SIP trunk status from server
pub async fn get_sip_status() -> Result<SipStatus, ApiError> {
    api_client().get::<SipStatus>("/api/sip/status").await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SipDialResponse {
    pub success: bool,
    pub call_id: Option<String>,
    pub error: Option<String>,
}

/// Dial a phone number via SIP trunk
pub async fn sip_dial(phone_number: &str) -> Result<SipDialResponse, ApiError> {
    #[derive(Serialize)]
    struct DialRequest {
        phone_number: String,
    }

    api_client()
        .post::<SipDialResponse, _>("/api/sip/dial", &DialRequest {
            phone_number: phone_number.to_string(),
        })
        .await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SipHangupResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Hangup a SIP call
pub async fn sip_hangup(call_id: &str) -> Result<SipHangupResponse, ApiError> {
    #[derive(Serialize)]
    struct HangupRequest {
        call_id: String,
    }

    api_client()
        .post::<SipHangupResponse, _>("/api/sip/hangup", &HangupRequest {
            call_id: call_id.to_string(),
        })
        .await
}
