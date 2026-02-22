use axum::http::HeaderMap;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::audit_log::user_claims_from_headers;

/// Begins a transaction and sets PostgreSQL session variables for Row-Level Security (RLS)
pub async fn begin_rls_transaction(
    pool: &PgPool,
    headers: &HeaderMap,
) -> Result<Transaction<'static, Postgres>> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let claims = user_claims_from_headers(headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    let role = claims.role;
    let user_id = claims.sub;
    let parsed_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    sqlx::query("SELECT set_config('app.current_role', $1, true)")
        .bind(&role)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    sqlx::query("SELECT set_config('app.current_user_id', $1, true)")
        .bind(&user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let row = sqlx::query!("SELECT department_id FROM users WHERE id = $1", parsed_uuid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::Authentication("User not found".to_string()))?;

    if let Some(dept_id) = row.department_id {
        sqlx::query("SELECT set_config('app.current_department_id', $1, true)")
            .bind(dept_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
    } else if role == "department_head" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    Ok(tx)
}
