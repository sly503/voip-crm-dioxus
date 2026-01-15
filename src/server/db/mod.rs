//! Database access layer using sqlx with PostgreSQL

pub mod leads;
pub mod agents;
pub mod campaigns;
pub mod calls;
pub mod users;
pub mod stats;
pub mod ai;
pub mod invitations;
pub mod recordings;

use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

/// Initialize the database connection pool
pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
