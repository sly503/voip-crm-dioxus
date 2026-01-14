//! Configuration API functions

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use super::client::{api_client, ApiError};

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRTCConfig {
    pub sip_username: String,
    pub sip_password: String,
    pub caller_id: String,
}

/// Fetch WebRTC configuration from server
#[cfg(target_arch = "wasm32")]
pub async fn get_webrtc_config() -> Result<WebRTCConfig, ApiError> {
    api_client().get::<WebRTCConfig>("/api/config/webrtc").await
}
