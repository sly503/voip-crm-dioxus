//! Statistics database operations

use sqlx::PgPool;
use crate::models::AgentStats;

pub async fn get_realtime(pool: &PgPool) -> Result<serde_json::Value, sqlx::Error> {
    // Get active calls count
    let active_calls: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM calls WHERE status IN ('Initiated', 'Ringing', 'Answered', 'Bridged')"
    )
    .fetch_one(pool)
    .await?;

    // Get ready agents count
    let ready_agents: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM agents WHERE status = 'Ready'"
    )
    .fetch_one(pool)
    .await?;

    // Get today's calls count
    let calls_today: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM calls WHERE DATE(started_at) = CURRENT_DATE"
    )
    .fetch_one(pool)
    .await?;

    // Get average handle time today
    let avg_handle_time: (Option<f64>,) = sqlx::query_as(
        "SELECT AVG(duration_seconds)::float8 FROM calls WHERE DATE(started_at) = CURRENT_DATE AND duration_seconds > 0"
    )
    .fetch_one(pool)
    .await?;

    Ok(serde_json::json!({
        "active_calls": active_calls.0,
        "ready_agents": ready_agents.0,
        "calls_today": calls_today.0,
        "avg_handle_time": avg_handle_time.0.unwrap_or(0.0)
    }))
}

pub async fn get_agent_stats(pool: &PgPool, agent_id: i64) -> Result<AgentStats, sqlx::Error> {
    let stats: (i64, i64, i64, i64, f64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*),
            COUNT(*) FILTER (WHERE status = 'Completed' AND answered_at IS NOT NULL),
            COUNT(*) FILTER (WHERE status IN ('NoAnswer', 'Busy', 'Failed')),
            COALESCE(SUM(duration_seconds), 0),
            COALESCE(AVG(duration_seconds) FILTER (WHERE duration_seconds > 0), 0)::float8
        FROM calls
        WHERE agent_id = $1 AND DATE(started_at) = CURRENT_DATE
        "#
    )
    .bind(agent_id)
    .fetch_one(pool)
    .await?;

    Ok(AgentStats {
        agent_id,
        total_calls: stats.0 as i32,
        answered_calls: stats.1 as i32,
        missed_calls: stats.2 as i32,
        total_talk_time: stats.3 as i32,
        average_handle_time: stats.4,
    })
}

pub async fn get_campaign_stats(pool: &PgPool, campaign_id: i64) -> Result<serde_json::Value, sqlx::Error> {
    let stats: (i64, i64, f64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*),
            COUNT(*) FILTER (WHERE status = 'Completed' AND answered_at IS NOT NULL),
            COALESCE(AVG(duration_seconds) FILTER (WHERE duration_seconds > 0), 0)::float8
        FROM calls
        WHERE campaign_id = $1
        "#
    )
    .bind(campaign_id)
    .fetch_one(pool)
    .await?;

    Ok(serde_json::json!({
        "total_calls": stats.0,
        "connected_calls": stats.1,
        "avg_duration": stats.2,
        "connect_rate": if stats.0 > 0 {
            stats.1 as f64 / stats.0 as f64 * 100.0
        } else {
            0.0
        }
    }))
}
