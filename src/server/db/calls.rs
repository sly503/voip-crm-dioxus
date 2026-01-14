//! Call database operations

use sqlx::PgPool;
use crate::models::{Call, CallStatus};

pub async fn get_by_id(pool: &PgPool, id: i64) -> Result<Option<Call>, sqlx::Error> {
    sqlx::query_as::<_, Call>(
        r#"
        SELECT id, call_control_id, lead_id, agent_id, campaign_id,
               direction, status, from_number, to_number,
               started_at, answered_at, ended_at,
               duration_seconds, disposition, recording_url
        FROM calls
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_by_control_id(pool: &PgPool, call_control_id: &str) -> Result<Option<Call>, sqlx::Error> {
    sqlx::query_as::<_, Call>(
        r#"
        SELECT id, call_control_id, lead_id, agent_id, campaign_id,
               direction, status, from_number, to_number,
               started_at, answered_at, ended_at,
               duration_seconds, disposition, recording_url
        FROM calls
        WHERE call_control_id = $1
        "#
    )
    .bind(call_control_id)
    .fetch_optional(pool)
    .await
}


/// Create a call without a lead (direct dial)
pub async fn create_direct(
    pool: &PgPool,
    agent_id: Option<i64>,
    call_control_id: &str,
    from_number: &str,
    to_number: &str,
) -> Result<Call, sqlx::Error> {
    sqlx::query_as::<_, Call>(
        r#"
        INSERT INTO calls (agent_id, call_control_id, direction, status, from_number, to_number, started_at)
        VALUES ($1, $2, 'Outbound', 'Initiated', $3, $4, NOW())
        RETURNING id, call_control_id, lead_id, agent_id, campaign_id,
                  direction, status, from_number, to_number,
                  started_at, answered_at, ended_at,
                  duration_seconds, disposition, recording_url
        "#
    )
    .bind(agent_id)
    .bind(call_control_id)
    .bind(from_number)
    .bind(to_number)
    .fetch_one(pool)
    .await
}

pub async fn update_status(pool: &PgPool, id: i64, status: CallStatus) -> Result<Call, sqlx::Error> {
    sqlx::query_as::<_, Call>(
        r#"
        UPDATE calls
        SET status = $2
        WHERE id = $1
        RETURNING id, call_control_id, lead_id, agent_id, campaign_id,
                  direction, status, from_number, to_number,
                  started_at, answered_at, ended_at,
                  duration_seconds, disposition, recording_url
        "#
    )
    .bind(id)
    .bind(status)
    .fetch_one(pool)
    .await
}

pub async fn set_answered(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE calls SET status = 'Answered', answered_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_ended(pool: &PgPool, id: i64, disposition: Option<&str>) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE calls
        SET status = 'Completed',
            ended_at = NOW(),
            duration_seconds = EXTRACT(EPOCH FROM (NOW() - COALESCE(answered_at, started_at)))::int,
            disposition = $2
        WHERE id = $1
        "#
    )
    .bind(id)
    .bind(disposition)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_active_by_agent(pool: &PgPool, agent_id: i64) -> Result<Option<Call>, sqlx::Error> {
    sqlx::query_as::<_, Call>(
        r#"
        SELECT id, call_control_id, lead_id, agent_id, campaign_id,
               direction, status, from_number, to_number,
               started_at, answered_at, ended_at,
               duration_seconds, disposition, recording_url
        FROM calls
        WHERE agent_id = $1 AND status IN ('Initiated', 'Ringing', 'Answered', 'Bridged')
        ORDER BY started_at DESC
        LIMIT 1
        "#
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_by_lead(pool: &PgPool, lead_id: i64) -> Result<Vec<Call>, sqlx::Error> {
    sqlx::query_as::<_, Call>(
        r#"
        SELECT id, call_control_id, lead_id, agent_id, campaign_id,
               direction, status, from_number, to_number,
               started_at, answered_at, ended_at,
               duration_seconds, disposition, recording_url
        FROM calls
        WHERE lead_id = $1
        ORDER BY started_at DESC
        "#
    )
    .bind(lead_id)
    .fetch_all(pool)
    .await
}

pub async fn get_recent(pool: &PgPool, limit: i64) -> Result<Vec<Call>, sqlx::Error> {
    sqlx::query_as::<_, Call>(
        r#"
        SELECT id, call_control_id, lead_id, agent_id, campaign_id,
               direction, status, from_number, to_number,
               started_at, answered_at, ended_at,
               duration_seconds, disposition, recording_url
        FROM calls
        ORDER BY started_at DESC
        LIMIT $1
        "#
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Create a call with required lead and agent (legacy, for direct dial from UI)
pub async fn create(
    pool: &PgPool,
    lead_id: i64,
    agent_id: i64,
    call_control_id: &str,
    from_number: &str,
    to_number: &str,
) -> Result<Call, sqlx::Error> {
    sqlx::query_as::<_, Call>(
        r#"
        INSERT INTO calls (lead_id, agent_id, call_control_id, direction, status, from_number, to_number, started_at)
        VALUES ($1, $2, $3, 'Outbound', 'Initiated', $4, $5, NOW())
        RETURNING id, call_control_id, lead_id, agent_id, campaign_id,
                  direction, status, from_number, to_number,
                  started_at, answered_at, ended_at,
                  duration_seconds, disposition, recording_url
        "#
    )
    .bind(lead_id)
    .bind(agent_id)
    .bind(call_control_id)
    .bind(from_number)
    .bind(to_number)
    .fetch_one(pool)
    .await
}

/// Create a call with optional lead, agent, and campaign (for automation)
pub async fn create_for_automation(
    pool: &PgPool,
    lead_id: Option<i64>,
    agent_id: Option<i64>,
    campaign_id: Option<i64>,
    from_number: &str,
    to_number: &str,
) -> Result<Call, sqlx::Error> {
    sqlx::query_as::<_, Call>(
        r"
        INSERT INTO calls (lead_id, agent_id, campaign_id, direction, status, from_number, to_number, started_at)
        VALUES ($1, $2, $3, 'Outbound', 'Initiated', $4, $5, NOW())
        RETURNING id, call_control_id, lead_id, agent_id, campaign_id,
                  direction, status, from_number, to_number,
                  started_at, answered_at, ended_at,
                  duration_seconds, disposition, recording_url
        "
    )
    .bind(lead_id)
    .bind(agent_id)
    .bind(campaign_id)
    .bind(from_number)
    .bind(to_number)
    .fetch_one(pool)
    .await
}

/// Set the Telnyx call control ID
pub async fn set_control_id(pool: &PgPool, id: i64, call_control_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE calls SET call_control_id = $2 WHERE id = $1")
        .bind(id)
        .bind(call_control_id)
        .execute(pool)
        .await?;
    Ok(())
}
