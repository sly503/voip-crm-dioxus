use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDate, Utc};

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallRecording {
    pub id: i64,
    #[serde(rename = "callId")]
    pub call_id: i64,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "fileSize")]
    pub file_size: i64,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: i32,
    pub format: String,
    #[serde(rename = "encryptionKeyId")]
    pub encryption_key_id: String,
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: DateTime<Utc>,
    #[serde(rename = "retentionUntil")]
    pub retention_until: DateTime<Utc>,
    #[serde(rename = "complianceHold")]
    pub compliance_hold: bool,
    pub metadata: Option<serde_json::Value>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingRetentionPolicy {
    pub id: i64,
    pub name: String,
    #[serde(rename = "retentionDays")]
    pub retention_days: i32,
    #[serde(rename = "appliesTo")]
    pub applies_to: RetentionAppliesTo,
    #[serde(rename = "campaignId")]
    pub campaign_id: Option<i64>,
    #[serde(rename = "agentId")]
    pub agent_id: Option<i64>,
    #[serde(rename = "isDefault")]
    pub is_default: bool,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(not(target_arch = "wasm32"), sqlx(type_name = "retention_applies_to", rename_all = "PascalCase"))]
pub enum RetentionAppliesTo {
    All,
    Campaign,
    Agent,
}

impl RetentionAppliesTo {
    pub fn display_name(&self) -> &str {
        match self {
            RetentionAppliesTo::All => "All Recordings",
            RetentionAppliesTo::Campaign => "Campaign",
            RetentionAppliesTo::Agent => "Agent",
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsage {
    pub id: i64,
    pub date: NaiveDate,
    #[serde(rename = "totalFiles")]
    pub total_files: i64,
    #[serde(rename = "totalSizeBytes")]
    pub total_size_bytes: i64,
    #[serde(rename = "recordingsAdded")]
    pub recordings_added: i32,
    #[serde(rename = "recordingsDeleted")]
    pub recordings_deleted: i32,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSearchParams {
    #[serde(rename = "agentId")]
    pub agent_id: Option<i64>,
    #[serde(rename = "campaignId")]
    pub campaign_id: Option<i64>,
    #[serde(rename = "leadId")]
    pub lead_id: Option<i64>,
    #[serde(rename = "startDate")]
    pub start_date: Option<DateTime<Utc>>,
    #[serde(rename = "endDate")]
    pub end_date: Option<DateTime<Utc>>,
    pub disposition: Option<String>,
    #[serde(rename = "complianceHold")]
    pub compliance_hold: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetadata {
    #[serde(rename = "agentName")]
    pub agent_name: Option<String>,
    #[serde(rename = "leadName")]
    pub lead_name: Option<String>,
    #[serde(rename = "campaignName")]
    pub campaign_name: Option<String>,
    pub disposition: Option<String>,
    #[serde(rename = "callDurationSeconds")]
    pub call_duration_seconds: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRecordingRequest {
    #[serde(rename = "callId")]
    pub call_id: i64,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "fileSize")]
    pub file_size: i64,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: i32,
    pub format: String,
    #[serde(rename = "encryptionKeyId")]
    pub encryption_key_id: String,
    #[serde(rename = "retentionUntil")]
    pub retention_until: DateTime<Utc>,
    pub metadata: Option<RecordingMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateComplianceHoldRequest {
    #[serde(rename = "complianceHold")]
    pub compliance_hold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRetentionPolicyRequest {
    pub name: String,
    #[serde(rename = "retentionDays")]
    pub retention_days: i32,
    #[serde(rename = "appliesTo")]
    pub applies_to: RetentionAppliesTo,
    #[serde(rename = "campaignId")]
    pub campaign_id: Option<i64>,
    #[serde(rename = "agentId")]
    pub agent_id: Option<i64>,
    #[serde(rename = "isDefault")]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    #[serde(rename = "totalFiles")]
    pub total_files: i64,
    #[serde(rename = "totalSizeBytes")]
    pub total_size_bytes: i64,
    #[serde(rename = "totalSizeGB")]
    pub total_size_gb: f64,
    #[serde(rename = "quotaGB")]
    pub quota_gb: f64,
    #[serde(rename = "quotaPercentage")]
    pub quota_percentage: f64,
    #[serde(rename = "dailyUsage")]
    pub daily_usage: Vec<StorageUsage>,
}
