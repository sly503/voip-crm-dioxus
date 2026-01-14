//! Agent database operations

use sqlx::PgPool;
use crate::models::{Agent, AgentStatus, CreateAgentRequest};

pub async fn get_all(pool: &PgPool) -> Result<Vec<Agent>, sqlx::Error> {
    sqlx::query_as::<_, Agent>(
        r#"
        SELECT id, name, extension, user_id, agent_type, status,
               sip_username, current_call_id, last_status_change, created_at
        FROM agents
        ORDER BY name
        "#
    )
    .fetch_all(pool)
    .await
}

pub async fn get_by_id(pool: &PgPool, id: i64) -> Result<Option<Agent>, sqlx::Error> {
    sqlx::query_as::<_, Agent>(
        r#"
        SELECT id, name, extension, user_id, agent_type, status,
               sip_username, current_call_id, last_status_change, created_at
        FROM agents
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_by_user(pool: &PgPool, user_id: i64) -> Result<Option<Agent>, sqlx::Error> {
    sqlx::query_as::<_, Agent>(
        r#"
        SELECT id, name, extension, user_id, agent_type, status,
               sip_username, current_call_id, last_status_change, created_at
        FROM agents
        WHERE user_id = $1
        "#
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_ready(pool: &PgPool) -> Result<Vec<Agent>, sqlx::Error> {
    sqlx::query_as::<_, Agent>(
        r#"
        SELECT id, name, extension, user_id, agent_type, status,
               sip_username, current_call_id, last_status_change, created_at
        FROM agents
        WHERE status = 'Ready'
        ORDER BY name
        "#
    )
    .fetch_all(pool)
    .await
}

pub async fn create(pool: &PgPool, req: CreateAgentRequest) -> Result<Agent, sqlx::Error> {
    sqlx::query_as::<_, Agent>(
        r#"
        INSERT INTO agents (name, extension, user_id, agent_type, status)
        VALUES ($1, $2, $3, $4, 'Offline')
        RETURNING id, name, extension, user_id, agent_type, status,
                  sip_username, current_call_id, last_status_change, created_at
        "#
    )
    .bind(&req.name)
    .bind(&req.extension)
    .bind(req.user_id)
    .bind(&req.agent_type)
    .fetch_one(pool)
    .await
}

pub async fn update(pool: &PgPool, id: i64, req: CreateAgentRequest) -> Result<Agent, sqlx::Error> {
    sqlx::query_as::<_, Agent>(
        r#"
        UPDATE agents
        SET name = $2, extension = $3
        WHERE id = $1
        RETURNING id, name, extension, user_id, agent_type, status,
                  sip_username, current_call_id, last_status_change, created_at
        "#
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.extension)
    .fetch_one(pool)
    .await
}

pub async fn update_status(pool: &PgPool, id: i64, status: AgentStatus) -> Result<Agent, sqlx::Error> {
    sqlx::query_as::<_, Agent>(
        r#"
        UPDATE agents
        SET status = $2, last_status_change = NOW()
        WHERE id = $1
        RETURNING id, name, extension, user_id, agent_type, status,
                  sip_username, current_call_id, last_status_change, created_at
        "#
    )
    .bind(id)
    .bind(status)
    .fetch_one(pool)
    .await
}

pub async fn set_current_call(pool: &PgPool, id: i64, call_id: Option<i64>) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE agents SET current_call_id = $2 WHERE id = $1")
        .bind(id)
        .bind(call_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get agents that are ready and assigned to a campaign
pub async fn get_ready_for_campaign(pool: &PgPool, campaign_id: i64) -> Result<Vec<Agent>, sqlx::Error> {
    sqlx::query_as::<_, Agent>(
        r"
        SELECT a.id, a.name, a.extension, a.user_id, a.agent_type, a.status,
               a.sip_username, a.current_call_id, a.last_status_change, a.created_at
        FROM agents a
        INNER JOIN campaign_agents ca ON a.id = ca.agent_id
        WHERE ca.campaign_id = $1 AND a.status = 'Ready'
        ORDER BY a.last_status_change ASC
        "
    )
    .bind(campaign_id)
    .fetch_all(pool)
    .await
}
