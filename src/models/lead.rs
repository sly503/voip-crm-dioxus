use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Lead {
    pub id: i64,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    pub phone: String,
    pub email: Option<String>,
    pub company: Option<String>,
    pub status: LeadStatus,
    pub notes: Option<String>,
    #[serde(rename = "assignedAgentId")]
    pub assigned_agent_id: Option<i64>,
    #[serde(rename = "campaignId")]
    pub campaign_id: Option<i64>,
    #[serde(rename = "callAttempts")]
    pub call_attempts: i32,
    #[serde(rename = "lastCallAt")]
    pub last_call_at: Option<DateTime<Utc>>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Lead {
    pub fn full_name(&self) -> String {
        let first = self.first_name.as_deref().unwrap_or("");
        let last = self.last_name.as_deref().unwrap_or("");
        format!("{} {}", first, last).trim().to_string()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(not(target_arch = "wasm32"), sqlx(type_name = "lead_status", rename_all = "PascalCase"))]
pub enum LeadStatus {
    New,
    Contacted,
    Qualified,
    Converted,
    Lost,
    DoNotCall,
}

impl LeadStatus {
    pub fn display_name(&self) -> &str {
        match self {
            LeadStatus::New => "New",
            LeadStatus::Contacted => "Contacted",
            LeadStatus::Qualified => "Qualified",
            LeadStatus::Converted => "Converted",
            LeadStatus::Lost => "Lost",
            LeadStatus::DoNotCall => "Do Not Call",
        }
    }

    pub fn color_class(&self) -> &str {
        match self {
            LeadStatus::New => "bg-blue-100 text-blue-800",
            LeadStatus::Contacted => "bg-yellow-100 text-yellow-800",
            LeadStatus::Qualified => "bg-green-100 text-green-800",
            LeadStatus::Converted => "bg-emerald-100 text-emerald-800",
            LeadStatus::Lost => "bg-gray-100 text-gray-800",
            LeadStatus::DoNotCall => "bg-red-100 text-red-800",
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadNote {
    pub id: i64,
    #[serde(rename = "leadId")]
    pub lead_id: i64,
    #[serde(rename = "agentId")]
    pub agent_id: Option<i64>,
    pub content: String,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLeadRequest {
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
    pub phone: String,
    pub email: Option<String>,
    pub company: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "campaignId")]
    pub campaign_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddNoteRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: LeadStatus,
}
