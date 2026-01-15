//! Campaign database operations

use sqlx::PgPool;
use crate::models::{Campaign, CampaignStatus, CreateCampaignRequest};

pub async fn get_all(pool: &PgPool) -> Result<Vec<Campaign>, sqlx::Error> {
    sqlx::query_as::<_, Campaign>(
        r#"
        SELECT id, name, description, status, dialer_mode, caller_id,
               start_time, end_time, max_attempts, retry_delay_minutes,
               total_leads, dialed_leads, connected_leads,
               consent_announcement, recording_enabled,
               created_at, updated_at
        FROM campaigns
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
}

pub async fn get_by_id(pool: &PgPool, id: i64) -> Result<Option<Campaign>, sqlx::Error> {
    sqlx::query_as::<_, Campaign>(
        r#"
        SELECT id, name, description, status, dialer_mode, caller_id,
               start_time, end_time, max_attempts, retry_delay_minutes,
               total_leads, dialed_leads, connected_leads,
               consent_announcement, recording_enabled,
               created_at, updated_at
        FROM campaigns
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_active(pool: &PgPool) -> Result<Vec<Campaign>, sqlx::Error> {
    sqlx::query_as::<_, Campaign>(
        r#"
        SELECT id, name, description, status, dialer_mode, caller_id,
               start_time, end_time, max_attempts, retry_delay_minutes,
               total_leads, dialed_leads, connected_leads,
               consent_announcement, recording_enabled,
               created_at, updated_at
        FROM campaigns
        WHERE status = 'Active'
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
}

pub async fn create(pool: &PgPool, req: CreateCampaignRequest) -> Result<Campaign, sqlx::Error> {
    sqlx::query_as::<_, Campaign>(
        r#"
        INSERT INTO campaigns (name, description, dialer_mode, caller_id, max_attempts, retry_delay_minutes,
                               consent_announcement, recording_enabled, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'Draft')
        RETURNING id, name, description, status, dialer_mode, caller_id,
                  start_time, end_time, max_attempts, retry_delay_minutes,
                  total_leads, dialed_leads, connected_leads,
                  consent_announcement, recording_enabled,
                  created_at, updated_at
        "#
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.dialer_mode)
    .bind(&req.caller_id)
    .bind(req.max_attempts.unwrap_or(3))
    .bind(req.retry_delay_minutes.unwrap_or(30))
    .bind(&req.consent_announcement)
    .bind(req.recording_enabled.unwrap_or(true))
    .fetch_one(pool)
    .await
}

pub async fn update(pool: &PgPool, id: i64, req: CreateCampaignRequest) -> Result<Campaign, sqlx::Error> {
    sqlx::query_as::<_, Campaign>(
        r#"
        UPDATE campaigns
        SET name = $2, description = $3, dialer_mode = $4,
            caller_id = $5, max_attempts = $6, retry_delay_minutes = $7,
            consent_announcement = $8, recording_enabled = $9,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, description, status, dialer_mode, caller_id,
                  start_time, end_time, max_attempts, retry_delay_minutes,
                  total_leads, dialed_leads, connected_leads,
                  consent_announcement, recording_enabled,
                  created_at, updated_at
        "#
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.dialer_mode)
    .bind(&req.caller_id)
    .bind(req.max_attempts.unwrap_or(3))
    .bind(req.retry_delay_minutes.unwrap_or(30))
    .bind(&req.consent_announcement)
    .bind(req.recording_enabled.unwrap_or(true))
    .fetch_one(pool)
    .await
}

pub async fn update_status(pool: &PgPool, id: i64, status: CampaignStatus) -> Result<Campaign, sqlx::Error> {
    sqlx::query_as::<_, Campaign>(
        r#"
        UPDATE campaigns
        SET status = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, description, status, dialer_mode, caller_id,
                  start_time, end_time, max_attempts, retry_delay_minutes,
                  total_leads, dialed_leads, connected_leads,
                  consent_announcement, recording_enabled,
                  created_at, updated_at
        "#
    )
    .bind(id)
    .bind(status)
    .fetch_one(pool)
    .await
}

pub async fn increment_dialed(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE campaigns SET dialed_leads = COALESCE(dialed_leads, 0) + 1 WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn increment_connected(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE campaigns SET connected_leads = COALESCE(connected_leads, 0) + 1 WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
