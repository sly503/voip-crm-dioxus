use crate::api::{api_client, ApiError};
use crate::models::{Campaign, CreateCampaignRequest, DialerStatus};

pub async fn get_all_campaigns() -> Result<Vec<Campaign>, ApiError> {
    api_client().get("/api/campaigns").await
}

pub async fn create_campaign(request: CreateCampaignRequest) -> Result<Campaign, ApiError> {
    api_client().post("/api/campaigns", &request).await
}

pub async fn update_campaign(id: i64, request: CreateCampaignRequest) -> Result<Campaign, ApiError> {
    api_client().put(&format!("/api/campaigns/{}", id), &request).await
}

pub async fn start_dialer(campaign_id: i64) -> Result<DialerStatus, ApiError> {
    api_client().post_empty(&format!("/api/campaigns/{}/start", campaign_id)).await
}

pub async fn pause_dialer(campaign_id: i64) -> Result<DialerStatus, ApiError> {
    api_client().post_empty(&format!("/api/campaigns/{}/pause", campaign_id)).await
}

pub async fn get_realtime_stats() -> Result<serde_json::Value, ApiError> {
    api_client().get("/api/statistics/realtime").await
}
