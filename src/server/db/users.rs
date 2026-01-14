//! User database operations

use sqlx::PgPool;
use crate::models::{User, UserRole};

pub async fn get_by_id(pool: &PgPool, id: i64) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, email, role, first_name, last_name, password_hash
        FROM users
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_by_username(pool: &PgPool, username: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, email, role, first_name, last_name, password_hash
        FROM users
        WHERE username = $1
        "#
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

pub async fn create(
    pool: &PgPool,
    username: &str,
    email: &str,
    password_hash: &str,
    role: UserRole,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, email, password_hash, role)
        VALUES ($1, $2, $3, $4)
        RETURNING id, username, email, role, first_name, last_name, password_hash
        "#
    )
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .fetch_one(pool)
    .await
}

pub async fn update_password(pool: &PgPool, id: i64, password_hash: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
        .bind(id)
        .bind(password_hash)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, email, role, first_name, last_name, password_hash
        FROM users
        WHERE email = $1
        "#
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}

pub async fn check_email_verified(pool: &PgPool, user_id: i64) -> Result<bool, sqlx::Error> {
    let result: (bool,) = sqlx::query_as(
        "SELECT email_verified FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(result.0)
}

pub async fn create_verification_token(
    pool: &PgPool,
    user_id: i64,
    email: &str,
    token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO verification_tokens (token, user_id, email, expires_at)
        VALUES ($1, $2, $3, NOW() + INTERVAL '24 hours')
        "#
    )
    .bind(token)
    .bind(user_id)
    .bind(email)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct VerificationToken {
    user_id: i64,
    email: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    used_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn get_verification_token(pool: &PgPool, token: &str) -> Result<Option<(i64, String, bool)>, sqlx::Error> {
    let result: Option<VerificationToken> = sqlx::query_as(
        r#"
        SELECT user_id, email, expires_at, used_at
        FROM verification_tokens
        WHERE token = $1
        "#
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|vt| {
        let is_valid = vt.used_at.is_none() && vt.expires_at > chrono::Utc::now();
        (vt.user_id, vt.email, is_valid)
    }))
}

pub async fn mark_token_used(pool: &PgPool, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE verification_tokens SET used_at = NOW() WHERE token = $1"
    )
    .bind(token)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn verify_email(pool: &PgPool, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET email_verified = TRUE WHERE id = $1"
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}
