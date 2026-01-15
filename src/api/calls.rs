use crate::api::{api_client, ApiError};
use crate::models::{DialRequest, DialResponse};
#[cfg(target_arch = "wasm32")]
use crate::models::Call;

pub async fn dial(lead_id: i64, agent_id: i64) -> Result<DialResponse, ApiError> {
    let request = DialRequest { lead_id, agent_id };
    api_client().post("/api/calls/dial", &request).await
}

/// Direct dial a phone number without a lead
#[cfg(target_arch = "wasm32")]
pub async fn dial_direct(phone_number: &str, agent_id: Option<i64>) -> Result<DialResponse, ApiError> {
    #[derive(serde::Serialize)]
    struct DirectDialRequest {
        #[serde(rename = "phoneNumber")]
        phone_number: String,
        #[serde(rename = "agentId", skip_serializing_if = "Option::is_none")]
        agent_id: Option<i64>,
    }
    let request = DirectDialRequest {
        phone_number: phone_number.to_string(),
        agent_id,
    };
    api_client().post("/api/calls/direct", &request).await
}

#[cfg(target_arch = "wasm32")]
pub async fn get_call_status(call_id: i64) -> Result<Call, ApiError> {
    api_client().get(&format!("/api/calls/{}", call_id)).await
}
