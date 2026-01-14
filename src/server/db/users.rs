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
