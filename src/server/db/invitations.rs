//! Invitation database operations

use sqlx::PgPool;
use crate::models::UserRole;

#[derive(sqlx::FromRow)]
pub struct Invitation {
    pub id: i64,
    pub token: String,
    pub email: String,
    pub role: UserRole,
    pub invited_by: i64,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub used_by: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Create a new invitation with 7-day expiration
pub async fn create_invitation(
    pool: &PgPool,
    token: &str,
    email: &str,
    role: UserRole,
    invited_by: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO invitations (token, email, role, invited_by, expires_at)
        VALUES ($1, $2, $3, $4, NOW() + INTERVAL '7 days')
        "#
    )
    .bind(token)
    .bind(email)
    .bind(role)
    .bind(invited_by)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct InvitationToken {
    email: String,
    role: UserRole,
    invited_by: i64,
    expires_at: chrono::DateTime<chrono::Utc>,
    used_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Get invitation by token and validate it
/// Returns (email, role, invited_by, is_valid)
pub async fn get_invitation_by_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<(String, UserRole, i64, bool)>, sqlx::Error> {
    let result: Option<InvitationToken> = sqlx::query_as(
        r#"
        SELECT email, role, invited_by, expires_at, used_at
        FROM invitations
        WHERE token = $1
        "#
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|inv| {
        let is_valid = inv.used_at.is_none() && inv.expires_at > chrono::Utc::now();
        (inv.email, inv.role, inv.invited_by, is_valid)
    }))
}

/// Mark an invitation as used
pub async fn mark_invitation_used(
    pool: &PgPool,
    token: &str,
    used_by: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE invitations
        SET used_at = NOW(), used_by = $2
        WHERE token = $1
        "#
    )
    .bind(token)
    .bind(used_by)
    .execute(pool)
    .await?;
    Ok(())
}

/// List all invitations created by a specific user
pub async fn list_invitations_by_inviter(
    pool: &PgPool,
    invited_by: i64,
) -> Result<Vec<Invitation>, sqlx::Error> {
    sqlx::query_as::<_, Invitation>(
        r#"
        SELECT id, token, email, role, invited_by, expires_at, used_at, used_by, created_at
        FROM invitations
        WHERE invited_by = $1
        ORDER BY created_at DESC
        "#
    )
    .bind(invited_by)
    .fetch_all(pool)
    .await
}
