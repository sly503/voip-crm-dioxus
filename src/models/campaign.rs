use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveTime, Utc};

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Campaign {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub status: CampaignStatus,
    #[serde(rename = "dialerMode")]
    pub dialer_mode: DialerMode,
    #[serde(rename = "callerId")]
    pub caller_id: Option<String>,
    #[serde(rename = "startTime")]
    pub start_time: Option<NaiveTime>,
    #[serde(rename = "endTime")]
    pub end_time: Option<NaiveTime>,
    #[serde(rename = "maxAttempts")]
    pub max_attempts: Option<i32>,
    #[serde(rename = "retryDelayMinutes")]
    pub retry_delay_minutes: Option<i32>,
    #[serde(rename = "totalLeads")]
    pub total_leads: Option<i32>,
    #[serde(rename = "dialedLeads")]
    pub dialed_leads: Option<i32>,
    #[serde(rename = "connectedLeads")]
    pub connected_leads: Option<i32>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(not(target_arch = "wasm32"), sqlx(type_name = "campaign_status", rename_all = "PascalCase"))]
pub enum CampaignStatus {
    Draft,
    Active,
    Paused,
    Completed,
}

impl CampaignStatus {
    pub fn display_name(&self) -> &str {
        match self {
            CampaignStatus::Draft => "Draft",
            CampaignStatus::Active => "Active",
            CampaignStatus::Paused => "Paused",
            CampaignStatus::Completed => "Completed",
        }
    }

    pub fn color_class(&self) -> &str {
        match self {
            CampaignStatus::Draft => "bg-gray-100 text-gray-800",
            CampaignStatus::Active => "bg-green-100 text-green-800",
            CampaignStatus::Paused => "bg-yellow-100 text-yellow-800",
            CampaignStatus::Completed => "bg-blue-100 text-blue-800",
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(not(target_arch = "wasm32"), sqlx(type_name = "dialer_mode", rename_all = "PascalCase"))]
pub enum DialerMode {
    Preview,
    Progressive,
    Predictive,
}

impl DialerMode {
    pub fn display_name(&self) -> &str {
        match self {
            DialerMode::Preview => "Preview",
            DialerMode::Progressive => "Progressive",
            DialerMode::Predictive => "Predictive",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCampaignRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "dialerMode")]
    pub dialer_mode: DialerMode,
    #[serde(rename = "callerId")]
    pub caller_id: Option<String>,
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
    #[serde(rename = "endTime")]
    pub end_time: Option<String>,
    #[serde(rename = "maxAttempts")]
    pub max_attempts: Option<i32>,
    #[serde(rename = "retryDelayMinutes")]
    pub retry_delay_minutes: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialerStatus {
    #[serde(rename = "campaignId")]
    pub campaign_id: i64,
    pub running: bool,
    pub paused: bool,
    #[serde(rename = "processedLeads")]
    pub processed_leads: i32,
    #[serde(rename = "successfulCalls")]
    pub successful_calls: i32,
    #[serde(rename = "failedCalls")]
    pub failed_calls: i32,
}
