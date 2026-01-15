use crate::api::{api_client, ApiError};
use crate::models::{
    CallRecording, CreateRetentionPolicyRequest, RecordingRetentionPolicy,
    RecordingSearchParams, StorageStats, UpdateComplianceHoldRequest,
};

/// Search recordings with optional filters
pub async fn search_recordings(
    params: RecordingSearchParams,
) -> Result<Vec<CallRecording>, ApiError> {
    // Build query string from params
    let mut query_params = Vec::new();

    if let Some(agent_id) = params.agent_id {
        query_params.push(format!("agentId={}", agent_id));
    }
    if let Some(campaign_id) = params.campaign_id {
        query_params.push(format!("campaignId={}", campaign_id));
    }
    if let Some(lead_id) = params.lead_id {
        query_params.push(format!("leadId={}", lead_id));
    }
    if let Some(start_date) = params.start_date {
        query_params.push(format!("startDate={}", start_date.to_rfc3339()));
    }
    if let Some(end_date) = params.end_date {
        query_params.push(format!("endDate={}", end_date.to_rfc3339()));
    }
    if let Some(disposition) = params.disposition {
        query_params.push(format!("disposition={}", disposition));
    }
    if let Some(compliance_hold) = params.compliance_hold {
        query_params.push(format!("complianceHold={}", compliance_hold));
    }
    if let Some(limit) = params.limit {
        query_params.push(format!("limit={}", limit));
    }
    if let Some(offset) = params.offset {
        query_params.push(format!("offset={}", offset));
    }

    let query_string = if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    };

    api_client()
        .get(&format!("/api/recordings{}", query_string))
        .await
}

/// Get recording details by ID
pub async fn get_recording(id: i64) -> Result<CallRecording, ApiError> {
    api_client()
        .get(&format!("/api/recordings/{}", id))
        .await
}

/// Delete a recording by ID
pub async fn delete_recording(id: i64) -> Result<(), ApiError> {
    api_client()
        .delete(&format!("/api/recordings/{}", id))
        .await
}

/// Update compliance hold status for a recording
pub async fn update_compliance_hold(
    id: i64,
    compliance_hold: bool,
) -> Result<CallRecording, ApiError> {
    let request = UpdateComplianceHoldRequest { compliance_hold };
    api_client()
        .put(&format!("/api/recordings/{}/compliance-hold", id), &request)
        .await
}

/// Get storage statistics
pub async fn get_storage_stats() -> Result<StorageStats, ApiError> {
    api_client()
        .get("/api/recordings/storage/stats")
        .await
}

/// Get all retention policies
pub async fn get_retention_policies() -> Result<Vec<RecordingRetentionPolicy>, ApiError> {
    api_client().get("/api/retention-policies").await
}

/// Get a specific retention policy by ID
pub async fn get_retention_policy(id: i64) -> Result<RecordingRetentionPolicy, ApiError> {
    api_client()
        .get(&format!("/api/retention-policies/{}", id))
        .await
}

/// Create a new retention policy
pub async fn create_retention_policy(
    request: CreateRetentionPolicyRequest,
) -> Result<RecordingRetentionPolicy, ApiError> {
    api_client()
        .post("/api/retention-policies", &request)
        .await
}

/// Update an existing retention policy
pub async fn update_retention_policy(
    id: i64,
    request: CreateRetentionPolicyRequest,
) -> Result<RecordingRetentionPolicy, ApiError> {
    api_client()
        .put(&format!("/api/retention-policies/{}", id), &request)
        .await
}

/// Delete a retention policy
pub async fn delete_retention_policy(id: i64) -> Result<(), ApiError> {
    api_client()
        .delete(&format!("/api/retention-policies/{}", id))
        .await
}

/// Get download URL for a recording
/// This function constructs the download URL that can be used with an <a> tag
/// or to initiate a download in the browser
#[cfg(target_arch = "wasm32")]
pub fn get_download_url(id: i64) -> String {
    // Get the base URL from the API client
    // In production, this would be constructed from the current window location
    format!("/api/recordings/{}/download", id)
}

/// Get stream URL for a recording
/// This function constructs the stream URL that can be used with an <audio> tag
#[cfg(target_arch = "wasm32")]
pub fn get_stream_url(id: i64) -> String {
    format!("/api/recordings/{}/stream", id)
}
