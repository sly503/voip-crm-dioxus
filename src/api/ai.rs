//! AI API client functions
#![allow(dead_code)]

use crate::api::{api_client, ApiError};
use crate::models::{AiAgentSettings, GlobalAiConfig, UpsertAiSettingsRequest, PromptTemplate};

/// Get AI settings for an agent
pub async fn get_settings(agent_id: i64) -> Result<Option<AiAgentSettings>, ApiError> {
    api_client().get(&format!("/api/ai/settings/{}", agent_id)).await
}

/// Get all AI settings
pub async fn get_all_settings() -> Result<Vec<AiAgentSettings>, ApiError> {
    api_client().get("/api/ai/settings").await
}

/// Create or update AI settings for an agent
pub async fn upsert_settings(request: UpsertAiSettingsRequest) -> Result<AiAgentSettings, ApiError> {
    api_client().post("/api/ai/settings", &request).await
}

/// Delete AI settings for an agent
pub async fn delete_settings(agent_id: i64) -> Result<(), ApiError> {
    api_client().delete(&format!("/api/ai/settings/{}", agent_id)).await
}

/// Get global AI configuration
pub async fn get_global_config() -> Result<GlobalAiConfig, ApiError> {
    api_client().get("/api/ai/config").await
}

/// Update global AI configuration
pub async fn update_global_config(config: GlobalAiConfig) -> Result<GlobalAiConfig, ApiError> {
    api_client().put("/api/ai/config", &config).await
}

/// Get all prompt templates
pub async fn get_templates() -> Result<Vec<PromptTemplate>, ApiError> {
    api_client().get("/api/ai/templates").await
}

/// Get a specific prompt template
pub async fn get_template(id: &str) -> Result<PromptTemplate, ApiError> {
    api_client().get(&format!("/api/ai/templates/{}", id)).await
}

/// Create a new prompt template
pub async fn create_template(template: PromptTemplate) -> Result<PromptTemplate, ApiError> {
    api_client().post("/api/ai/templates", &template).await
}

/// Update a prompt template
pub async fn update_template(id: &str, template: PromptTemplate) -> Result<PromptTemplate, ApiError> {
    api_client().put(&format!("/api/ai/templates/{}", id), &template).await
}

/// Delete a prompt template
pub async fn delete_template(id: &str) -> Result<(), ApiError> {
    api_client().delete(&format!("/api/ai/templates/{}", id)).await
}

/// Start campaign automation
pub async fn start_automation(campaign_id: i64) -> Result<(), ApiError> {
    api_client().post_no_response(&format!("/api/campaigns/{}/automation/start", campaign_id)).await
}

/// Stop campaign automation
pub async fn stop_automation(campaign_id: i64) -> Result<(), ApiError> {
    api_client().post_no_response(&format!("/api/campaigns/{}/automation/stop", campaign_id)).await
}

/// Get campaign automation status
pub async fn get_automation_status(campaign_id: i64) -> Result<AutomationStatus, ApiError> {
    api_client().get(&format!("/api/campaigns/{}/automation/status", campaign_id)).await
}

/// Automation status response
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AutomationStatus {
    #[serde(rename = "isRunning")]
    pub is_running: bool,
    #[serde(rename = "callsInProgress")]
    pub calls_in_progress: i32,
    #[serde(rename = "leadsProcessed")]
    pub leads_processed: i32,
    #[serde(rename = "lastDialAt")]
    pub last_dial_at: Option<chrono::DateTime<chrono::Utc>>,
}
