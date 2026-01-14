use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Agent {
    pub id: i64,
    pub name: String,
    pub extension: Option<String>,
    #[serde(rename = "userId")]
    pub user_id: Option<i64>,
    #[serde(rename = "agentType")]
    pub agent_type: AgentType,
    pub status: AgentStatus,
    #[serde(rename = "sipUsername")]
    pub sip_username: Option<String>,
    #[serde(rename = "currentCallId")]
    pub current_call_id: Option<i64>,
    #[serde(rename = "lastStatusChange")]
    pub last_status_change: Option<DateTime<Utc>>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(not(target_arch = "wasm32"), sqlx(type_name = "agent_type", rename_all = "PascalCase"))]
pub enum AgentType {
    Human,
    Ai,
}

impl AgentType {
    pub fn display_name(&self) -> &str {
        match self {
            AgentType::Human => "Human",
            AgentType::Ai => "AI Agent",
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(not(target_arch = "wasm32"), sqlx(type_name = "agent_status", rename_all = "PascalCase"))]
pub enum AgentStatus {
    Offline,
    Ready,
    OnCall,
    AfterCall,
    Break,
}

impl AgentStatus {
    pub fn display_name(&self) -> &str {
        match self {
            AgentStatus::Offline => "Offline",
            AgentStatus::Ready => "Ready",
            AgentStatus::OnCall => "On Call",
            AgentStatus::AfterCall => "After Call",
            AgentStatus::Break => "On Break",
        }
    }

    pub fn color_class(&self) -> &str {
        match self {
            AgentStatus::Ready => "bg-green-500",
            AgentStatus::Offline => "bg-gray-400",
            AgentStatus::OnCall => "bg-red-500",
            AgentStatus::AfterCall => "bg-orange-500",
            AgentStatus::Break => "bg-blue-500",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    #[serde(rename = "agentType")]
    pub agent_type: AgentType,
    #[serde(rename = "userId")]
    pub user_id: Option<i64>,
    pub extension: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentStatusRequest {
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    #[serde(rename = "agentId")]
    pub agent_id: i64,
    #[serde(rename = "totalCalls")]
    pub total_calls: i32,
    #[serde(rename = "answeredCalls")]
    pub answered_calls: i32,
    #[serde(rename = "missedCalls")]
    pub missed_calls: i32,
    #[serde(rename = "totalTalkTime")]
    pub total_talk_time: i32,
    #[serde(rename = "averageHandleTime")]
    pub average_handle_time: f64,
}
