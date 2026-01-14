use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    pub id: i64,
    #[serde(rename = "callControlId")]
    pub call_control_id: Option<String>,
    #[serde(rename = "leadId")]
    pub lead_id: Option<i64>,
    #[serde(rename = "agentId")]
    pub agent_id: Option<i64>,
    #[serde(rename = "campaignId")]
    pub campaign_id: Option<i64>,
    pub direction: CallDirection,
    pub status: CallStatus,
    #[serde(rename = "fromNumber")]
    pub from_number: Option<String>,
    #[serde(rename = "toNumber")]
    pub to_number: Option<String>,
    #[serde(rename = "startedAt")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(rename = "answeredAt")]
    pub answered_at: Option<DateTime<Utc>>,
    #[serde(rename = "endedAt")]
    pub ended_at: Option<DateTime<Utc>>,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: Option<i32>,
    pub disposition: Option<String>,
    #[serde(rename = "recordingUrl")]
    pub recording_url: Option<String>,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(not(target_arch = "wasm32"), sqlx(type_name = "call_direction", rename_all = "PascalCase"))]
pub enum CallDirection {
    Inbound,
    Outbound,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(not(target_arch = "wasm32"), sqlx(type_name = "call_status", rename_all = "PascalCase"))]
pub enum CallStatus {
    Initiated,
    Ringing,
    Answered,
    Bridged,
    Completed,
    NoAnswer,
    Busy,
    Failed,
}

impl CallStatus {
    pub fn display_name(&self) -> &str {
        match self {
            CallStatus::Initiated => "Dialing...",
            CallStatus::Ringing => "Ringing",
            CallStatus::Answered => "Connected",
            CallStatus::Bridged => "In Call",
            CallStatus::Completed => "Completed",
            CallStatus::NoAnswer => "No Answer",
            CallStatus::Busy => "Busy",
            CallStatus::Failed => "Failed",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, CallStatus::Initiated | CallStatus::Ringing | CallStatus::Answered | CallStatus::Bridged)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialRequest {
    #[serde(rename = "leadId")]
    pub lead_id: i64,
    #[serde(rename = "agentId")]
    pub agent_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialResponse {
    #[serde(rename = "callId")]
    pub call_id: i64,
    #[serde(rename = "callControlId")]
    pub call_control_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    #[serde(rename = "callId")]
    pub call_id: i64,
    #[serde(rename = "targetAgentId")]
    pub target_agent_id: Option<i64>,
    #[serde(rename = "targetNumber")]
    pub target_number: Option<String>,
}
