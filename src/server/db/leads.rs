//! Lead database operations

use sqlx::PgPool;
use crate::models::{Lead, LeadStatus, CreateLeadRequest};

pub async fn get_all(pool: &PgPool) -> Result<Vec<Lead>, sqlx::Error> {
    sqlx::query_as::<_, Lead>(
        r#"
        SELECT id, first_name, last_name, phone, email, company,
               status, notes, assigned_agent_id, campaign_id,
               call_attempts, last_call_at, created_at, updated_at
        FROM leads
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
}

pub async fn get_by_id(pool: &PgPool, id: i64) -> Result<Option<Lead>, sqlx::Error> {
    sqlx::query_as::<_, Lead>(
        r#"
        SELECT id, first_name, last_name, phone, email, company,
               status, notes, assigned_agent_id, campaign_id,
               call_attempts, last_call_at, created_at, updated_at
        FROM leads
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_by_agent(pool: &PgPool, agent_id: i64) -> Result<Vec<Lead>, sqlx::Error> {
    sqlx::query_as::<_, Lead>(
        r#"
        SELECT id, first_name, last_name, phone, email, company,
               status, notes, assigned_agent_id, campaign_id,
               call_attempts, last_call_at, created_at, updated_at
        FROM leads
        WHERE assigned_agent_id = $1
        ORDER BY created_at DESC
        "#
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
}

pub async fn get_by_campaign(pool: &PgPool, campaign_id: i64) -> Result<Vec<Lead>, sqlx::Error> {
    sqlx::query_as::<_, Lead>(
        r#"
        SELECT id, first_name, last_name, phone, email, company,
               status, notes, assigned_agent_id, campaign_id,
               call_attempts, last_call_at, created_at, updated_at
        FROM leads
        WHERE campaign_id = $1
        ORDER BY created_at DESC
        "#
    )
    .bind(campaign_id)
    .fetch_all(pool)
    .await
}

pub async fn create(pool: &PgPool, req: CreateLeadRequest) -> Result<Lead, sqlx::Error> {
    sqlx::query_as::<_, Lead>(
        r#"
        INSERT INTO leads (first_name, last_name, phone, email, company, campaign_id, status)
        VALUES ($1, $2, $3, $4, $5, $6, 'New')
        RETURNING id, first_name, last_name, phone, email, company,
                  status, notes, assigned_agent_id, campaign_id,
                  call_attempts, last_call_at, created_at, updated_at
        "#
    )
    .bind(&req.first_name)
    .bind(&req.last_name)
    .bind(&req.phone)
    .bind(&req.email)
    .bind(&req.company)
    .bind(req.campaign_id)
    .fetch_one(pool)
    .await
}

pub async fn update(pool: &PgPool, id: i64, req: CreateLeadRequest) -> Result<Lead, sqlx::Error> {
    sqlx::query_as::<_, Lead>(
        r#"
        UPDATE leads
        SET first_name = $2, last_name = $3, phone = $4, email = $5, company = $6, updated_at = NOW()
        WHERE id = $1
        RETURNING id, first_name, last_name, phone, email, company,
                  status, notes, assigned_agent_id, campaign_id,
                  call_attempts, last_call_at, created_at, updated_at
        "#
    )
    .bind(id)
    .bind(&req.first_name)
    .bind(&req.last_name)
    .bind(&req.phone)
    .bind(&req.email)
    .bind(&req.company)
    .fetch_one(pool)
    .await
}

pub async fn delete(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM leads WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_status(pool: &PgPool, id: i64, status: LeadStatus) -> Result<Lead, sqlx::Error> {
    sqlx::query_as::<_, Lead>(
        r#"
        UPDATE leads
        SET status = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, first_name, last_name, phone, email, company,
                  status, notes, assigned_agent_id, campaign_id,
                  call_attempts, last_call_at, created_at, updated_at
        "#
    )
    .bind(id)
    .bind(status)
    .fetch_one(pool)
    .await
}

pub async fn assign(pool: &PgPool, id: i64, agent_id: i64) -> Result<Lead, sqlx::Error> {
    sqlx::query_as::<_, Lead>(
        r#"
        UPDATE leads
        SET assigned_agent_id = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, first_name, last_name, phone, email, company,
                  status, notes, assigned_agent_id, campaign_id,
                  call_attempts, last_call_at, created_at, updated_at
        "#
    )
    .bind(id)
    .bind(agent_id)
    .fetch_one(pool)
    .await
}

pub async fn add_note(pool: &PgPool, lead_id: i64, content: &str) -> Result<Lead, sqlx::Error> {
    // Append note to existing notes
    sqlx::query_as::<_, Lead>(
        r#"
        UPDATE leads
        SET notes = COALESCE(notes, '') || E'\n' || $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, first_name, last_name, phone, email, company,
                  status, notes, assigned_agent_id, campaign_id,
                  call_attempts, last_call_at, created_at, updated_at
        "#
    )
    .bind(lead_id)
    .bind(content)
    .fetch_one(pool)
    .await
}

pub async fn increment_call_attempts(pool: &PgPool, id: i64) -> Result<Lead, sqlx::Error> {
    sqlx::query_as::<_, Lead>(
        r#"
        UPDATE leads
        SET call_attempts = call_attempts + 1, last_call_at = NOW(), updated_at = NOW()
        WHERE id = $1
        RETURNING id, first_name, last_name, phone, email, company,
                  status, notes, assigned_agent_id, campaign_id,
                  call_attempts, last_call_at, created_at, updated_at
        "#
    )
    .bind(id)
    .fetch_one(pool)
    .await
}
