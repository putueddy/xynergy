use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub changes: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

async fn get_audit_logs(
    State(pool): State<PgPool>,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<Vec<AuditLogResponse>>> {
    let limit = query.limit.unwrap_or(50).max(1).min(200);

    let rows = sqlx::query!(
        r#"
        SELECT
            al.id,
            al.user_id,
            al.action,
            al.entity_type,
            al.entity_id,
            al.changes,
            al.created_at,
            u.first_name AS "first_name?",
            u.last_name AS "last_name?"
        FROM audit_logs al
        LEFT JOIN users u ON al.user_id = u.id
        ORDER BY al.created_at DESC
        LIMIT $1
        "#,
        limit
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let response = rows
        .into_iter()
        .map(|row| {
            let first_name = row.first_name.unwrap_or_default();
            let last_name = row.last_name.unwrap_or_default();
            let user_name = {
                let full_name = format!("{} {}", first_name, last_name).trim().to_string();
                if full_name.is_empty() {
                    None
                } else {
                    Some(full_name)
                }
            };

            AuditLogResponse {
                id: row.id,
                user_id: row.user_id,
                user_name,
                action: row.action,
                entity_type: row.entity_type,
                entity_id: row.entity_id,
                changes: row.changes.unwrap_or(serde_json::Value::Null),
                created_at: row.created_at.unwrap_or_else(Utc::now),
            }
        })
        .collect();

    Ok(Json(response))
}

pub fn audit_log_routes() -> Router<PgPool> {
    Router::new().route("/audit-logs", get(get_audit_logs))
}
