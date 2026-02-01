use sqlx::PgPool;
use std::env;

use crate::error::{AppError, Result};

/// Database connection pool
#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    pub async fn new() -> Result<Self> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| AppError::Internal("DATABASE_URL not set".to_string()))?;

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Run migrations
    pub async fn migrate(&self) -> Result<()> {
        // Migrations are in the project root
        // Note: sqlx::migrate! requires a literal string path relative to CARGO_MANIFEST_DIR
        // For now, we'll skip automatic migrations and rely on sqlx-cli
        tracing::info!("Migrations should be run with: sqlx migrate run");
        Ok(())
    }

    /// Test the database connection
    pub async fn test_connection(&self) -> Result<String> {
        let row: (String,) = sqlx::query_as("SELECT version()")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(row.0)
    }
}
