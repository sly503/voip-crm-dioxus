use crate::api::{api_client, ApiError};
use crate::models::{Lead, AddNoteRequest, UpdateStatusRequest, LeadNote};

pub async fn get_my_leads() -> Result<Vec<Lead>, ApiError> {
    api_client().get("/api/leads/my").await
}

pub async fn get_lead(id: i64) -> Result<Lead, ApiError> {
    api_client().get(&format!("/api/leads/{}", id)).await
}

pub async fn add_note(lead_id: i64, content: &str) -> Result<LeadNote, ApiError> {
    let request = AddNoteRequest {
        content: content.to_string(),
    };
    api_client().post(&format!("/api/leads/{}/notes", lead_id), &request).await
}

pub async fn update_status(lead_id: i64, request: UpdateStatusRequest) -> Result<Lead, ApiError> {
    api_client().put(&format!("/api/leads/{}/status", lead_id), &request).await
}
