//! Recording database operations

use sqlx::PgPool;
use chrono::{NaiveDate, Utc};
use crate::models::recording::{CallRecording, StorageUsage, RecordingSearchParams, RecordingRetentionPolicy, CreateRetentionPolicyRequest};

/// Insert a new call recording
pub async fn insert_recording(
    pool: &PgPool,
    call_id: i64,
    file_path: &str,
    file_size: i64,
    duration_seconds: i32,
    format: &str,
    encryption_key_id: &str,
    retention_until: chrono::DateTime<Utc>,
    metadata: Option<serde_json::Value>,
) -> Result<CallRecording, sqlx::Error> {
    sqlx::query_as::<_, CallRecording>(
        r#"
        INSERT INTO call_recordings (
            call_id, file_path, file_size, duration_seconds, format,
            encryption_key_id, uploaded_at, retention_until, compliance_hold, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, NOW(), $7, false, $8)
        RETURNING id, call_id, file_path, file_size, duration_seconds, format,
                  encryption_key_id, uploaded_at, retention_until, compliance_hold,
                  metadata, created_at
        "#
    )
    .bind(call_id)
    .bind(file_path)
    .bind(file_size)
    .bind(duration_seconds)
    .bind(format)
    .bind(encryption_key_id)
    .bind(retention_until)
    .bind(metadata)
    .fetch_one(pool)
    .await
}

/// Get a recording by ID
pub async fn get_recording(pool: &PgPool, id: i64) -> Result<Option<CallRecording>, sqlx::Error> {
    sqlx::query_as::<_, CallRecording>(
        r#"
        SELECT id, call_id, file_path, file_size, duration_seconds, format,
               encryption_key_id, uploaded_at, retention_until, compliance_hold,
               metadata, created_at
        FROM call_recordings
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Delete a recording by ID
pub async fn delete_recording(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM call_recordings WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Search recordings with filters
pub async fn search_recordings(
    pool: &PgPool,
    params: &RecordingSearchParams,
) -> Result<Vec<CallRecording>, sqlx::Error> {
    let mut query = String::from(
        r#"
        SELECT id, call_id, file_path, file_size, duration_seconds, format,
               encryption_key_id, uploaded_at, retention_until, compliance_hold,
               metadata, created_at
        FROM call_recordings
        WHERE 1=1
        "#
    );

    let mut bind_count = 1;

    if params.agent_id.is_some() {
        query.push_str(&format!(" AND call_id IN (SELECT id FROM calls WHERE agent_id = ${})", bind_count));
        bind_count += 1;
    }

    if params.campaign_id.is_some() {
        query.push_str(&format!(" AND call_id IN (SELECT id FROM calls WHERE campaign_id = ${})", bind_count));
        bind_count += 1;
    }

    if params.lead_id.is_some() {
        query.push_str(&format!(" AND call_id IN (SELECT id FROM calls WHERE lead_id = ${})", bind_count));
        bind_count += 1;
    }

    if params.start_date.is_some() {
        query.push_str(&format!(" AND uploaded_at >= ${}", bind_count));
        bind_count += 1;
    }

    if params.end_date.is_some() {
        query.push_str(&format!(" AND uploaded_at <= ${}", bind_count));
        bind_count += 1;
    }

    if params.disposition.is_some() {
        query.push_str(&format!(" AND call_id IN (SELECT id FROM calls WHERE disposition = ${})", bind_count));
        bind_count += 1;
    }

    if params.compliance_hold.is_some() {
        query.push_str(&format!(" AND compliance_hold = ${}", bind_count));
        bind_count += 1;
    }

    query.push_str(" ORDER BY uploaded_at DESC");

    if let Some(limit) = params.limit {
        query.push_str(&format!(" LIMIT ${}", bind_count));
        bind_count += 1;
    }

    if let Some(offset) = params.offset {
        query.push_str(&format!(" OFFSET ${}", bind_count));
    }

    let mut query_builder = sqlx::query_as::<_, CallRecording>(&query);

    if let Some(agent_id) = params.agent_id {
        query_builder = query_builder.bind(agent_id);
    }
    if let Some(campaign_id) = params.campaign_id {
        query_builder = query_builder.bind(campaign_id);
    }
    if let Some(lead_id) = params.lead_id {
        query_builder = query_builder.bind(lead_id);
    }
    if let Some(start_date) = params.start_date {
        query_builder = query_builder.bind(start_date);
    }
    if let Some(end_date) = params.end_date {
        query_builder = query_builder.bind(end_date);
    }
    if let Some(disposition) = &params.disposition {
        query_builder = query_builder.bind(disposition);
    }
    if let Some(compliance_hold) = params.compliance_hold {
        query_builder = query_builder.bind(compliance_hold);
    }
    if let Some(limit) = params.limit {
        query_builder = query_builder.bind(limit);
    }
    if let Some(offset) = params.offset {
        query_builder = query_builder.bind(offset);
    }

    query_builder.fetch_all(pool).await
}

/// Update compliance hold status
pub async fn set_compliance_hold(
    pool: &PgPool,
    id: i64,
    compliance_hold: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE call_recordings SET compliance_hold = $2 WHERE id = $1")
        .bind(id)
        .bind(compliance_hold)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update retention_until date
pub async fn update_retention(
    pool: &PgPool,
    id: i64,
    retention_until: chrono::DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE call_recordings SET retention_until = $2 WHERE id = $1")
        .bind(id)
        .bind(retention_until)
        .execute(pool)
        .await?;
    Ok(())
}

// Storage Usage Tracking Functions

/// Get or create today's storage usage entry
pub async fn get_or_create_today_usage(pool: &PgPool) -> Result<StorageUsage, sqlx::Error> {
    let today = Utc::now().date_naive();

    // Try to get existing entry for today
    if let Some(usage) = get_usage_by_date(pool, today).await? {
        return Ok(usage);
    }

    // Create new entry for today
    sqlx::query_as::<_, StorageUsage>(
        r#"
        INSERT INTO storage_usage_log (date, total_files, total_size_bytes, recordings_added, recordings_deleted)
        VALUES ($1, 0, 0, 0, 0)
        RETURNING id, date, total_files, total_size_bytes, recordings_added, recordings_deleted, created_at
        "#
    )
    .bind(today)
    .fetch_one(pool)
    .await
}

/// Get storage usage for a specific date
pub async fn get_usage_by_date(
    pool: &PgPool,
    date: NaiveDate,
) -> Result<Option<StorageUsage>, sqlx::Error> {
    sqlx::query_as::<_, StorageUsage>(
        r#"
        SELECT id, date, total_files, total_size_bytes, recordings_added, recordings_deleted, created_at
        FROM storage_usage_log
        WHERE date = $1
        "#
    )
    .bind(date)
    .fetch_optional(pool)
    .await
}

/// Update daily storage stats
pub async fn update_daily_storage_stats(
    pool: &PgPool,
    date: NaiveDate,
    total_files: i64,
    total_size_bytes: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO storage_usage_log (date, total_files, total_size_bytes, recordings_added, recordings_deleted)
        VALUES ($1, $2, $3, 0, 0)
        ON CONFLICT (date) DO UPDATE
        SET total_files = EXCLUDED.total_files,
            total_size_bytes = EXCLUDED.total_size_bytes
        "#
    )
    .bind(date)
    .bind(total_files)
    .bind(total_size_bytes)
    .execute(pool)
    .await?;
    Ok(())
}

/// Increment recordings added count for today
pub async fn increment_recordings_added(pool: &PgPool) -> Result<(), sqlx::Error> {
    let today = Utc::now().date_naive();

    sqlx::query(
        r#"
        INSERT INTO storage_usage_log (date, total_files, total_size_bytes, recordings_added, recordings_deleted)
        VALUES ($1, 0, 0, 1, 0)
        ON CONFLICT (date) DO UPDATE
        SET recordings_added = storage_usage_log.recordings_added + 1
        "#
    )
    .bind(today)
    .execute(pool)
    .await?;
    Ok(())
}

/// Increment recordings deleted count for today
pub async fn increment_recordings_deleted(pool: &PgPool) -> Result<(), sqlx::Error> {
    let today = Utc::now().date_naive();

    sqlx::query(
        r#"
        INSERT INTO storage_usage_log (date, total_files, total_size_bytes, recordings_added, recordings_deleted)
        VALUES ($1, 0, 0, 0, 1)
        ON CONFLICT (date) DO UPDATE
        SET recordings_deleted = storage_usage_log.recordings_deleted + 1
        "#
    )
    .bind(today)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get recent storage usage history
pub async fn get_usage_history(
    pool: &PgPool,
    days: i32,
) -> Result<Vec<StorageUsage>, sqlx::Error> {
    sqlx::query_as::<_, StorageUsage>(
        r#"
        SELECT id, date, total_files, total_size_bytes, recordings_added, recordings_deleted, created_at
        FROM storage_usage_log
        WHERE date >= CURRENT_DATE - $1
        ORDER BY date DESC
        "#
    )
    .bind(days)
    .fetch_all(pool)
    .await
}

/// Get total storage statistics from the database
pub async fn get_total_storage_stats(pool: &PgPool) -> Result<(i64, i64), sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as "count!",
            COALESCE(SUM(file_size), 0) as "total_size!"
        FROM call_recordings
        "#
    )
    .fetch_one(pool)
    .await?;

    Ok((result.count, result.total_size))
}

// Retention Policy Functions

/// Get all retention policies
pub async fn get_all_retention_policies(pool: &PgPool) -> Result<Vec<RecordingRetentionPolicy>, sqlx::Error> {
    sqlx::query_as::<_, RecordingRetentionPolicy>(
        r#"
        SELECT id, name, retention_days, applies_to, campaign_id, agent_id, is_default, created_at, updated_at
        FROM recording_retention_policies
        ORDER BY is_default DESC, created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
}

/// Get a retention policy by ID
pub async fn get_retention_policy(pool: &PgPool, id: i64) -> Result<Option<RecordingRetentionPolicy>, sqlx::Error> {
    sqlx::query_as::<_, RecordingRetentionPolicy>(
        r#"
        SELECT id, name, retention_days, applies_to, campaign_id, agent_id, is_default, created_at, updated_at
        FROM recording_retention_policies
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Create a new retention policy
pub async fn create_retention_policy(
    pool: &PgPool,
    req: &CreateRetentionPolicyRequest,
) -> Result<RecordingRetentionPolicy, sqlx::Error> {
    sqlx::query_as::<_, RecordingRetentionPolicy>(
        r#"
        INSERT INTO recording_retention_policies (name, retention_days, applies_to, campaign_id, agent_id, is_default)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, name, retention_days, applies_to, campaign_id, agent_id, is_default, created_at, updated_at
        "#
    )
    .bind(&req.name)
    .bind(req.retention_days)
    .bind(&req.applies_to)
    .bind(req.campaign_id)
    .bind(req.agent_id)
    .bind(req.is_default)
    .fetch_one(pool)
    .await
}

/// Update a retention policy
pub async fn update_retention_policy(
    pool: &PgPool,
    id: i64,
    req: &CreateRetentionPolicyRequest,
) -> Result<RecordingRetentionPolicy, sqlx::Error> {
    sqlx::query_as::<_, RecordingRetentionPolicy>(
        r#"
        UPDATE recording_retention_policies
        SET name = $2,
            retention_days = $3,
            applies_to = $4,
            campaign_id = $5,
            agent_id = $6,
            is_default = $7,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, retention_days, applies_to, campaign_id, agent_id, is_default, created_at, updated_at
        "#
    )
    .bind(id)
    .bind(&req.name)
    .bind(req.retention_days)
    .bind(&req.applies_to)
    .bind(req.campaign_id)
    .bind(req.agent_id)
    .bind(req.is_default)
    .fetch_one(pool)
    .await
}

/// Delete a retention policy
pub async fn delete_retention_policy(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM recording_retention_policies WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get the default retention policy
pub async fn get_default_retention_policy(pool: &PgPool) -> Result<Option<RecordingRetentionPolicy>, sqlx::Error> {
    sqlx::query_as::<_, RecordingRetentionPolicy>(
        r#"
        SELECT id, name, retention_days, applies_to, campaign_id, agent_id, is_default, created_at, updated_at
        FROM recording_retention_policies
        WHERE is_default = true
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await
}

/// Get retention policy for a specific campaign
pub async fn get_campaign_retention_policy(pool: &PgPool, campaign_id: i64) -> Result<Option<RecordingRetentionPolicy>, sqlx::Error> {
    sqlx::query_as::<_, RecordingRetentionPolicy>(
        r#"
        SELECT id, name, retention_days, applies_to, campaign_id, agent_id, is_default, created_at, updated_at
        FROM recording_retention_policies
        WHERE applies_to = 'Campaign' AND campaign_id = $1
        LIMIT 1
        "#
    )
    .bind(campaign_id)
    .fetch_optional(pool)
    .await
}

/// Get retention policy for a specific agent
pub async fn get_agent_retention_policy(pool: &PgPool, agent_id: i64) -> Result<Option<RecordingRetentionPolicy>, sqlx::Error> {
    sqlx::query_as::<_, RecordingRetentionPolicy>(
        r#"
        SELECT id, name, retention_days, applies_to, campaign_id, agent_id, is_default, created_at, updated_at
        FROM recording_retention_policies
        WHERE applies_to = 'Agent' AND agent_id = $1
        LIMIT 1
        "#
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a test database to run
    // They are marked as ignored by default

    #[tokio::test]
    #[ignore]
    async fn test_storage_usage_tracking() {
        // This would require setting up a test database
        // For now, it's a placeholder for future integration tests
    }
}
