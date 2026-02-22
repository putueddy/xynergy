use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::{audit_log::user_claims_from_headers, log_audit, recompute_entry_hash};

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub action_type: Option<String>,
    pub user_id: Option<Uuid>,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
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
    pub previous_hash: Option<String>,
    pub entry_hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditReportResponse {
    pub entries: Vec<AuditLogResponse>,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
pub struct ExportApprovalState {
    pub export_id: Uuid,
    pub status: String,
    pub requested_by: Uuid,
    pub note: String,
}

async fn require_audit_management_access(headers: &HeaderMap) -> Result<Uuid> {
    let claims = user_claims_from_headers(headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    if claims.role != "admin" && claims.role != "finance" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID".to_string()))
}

async fn get_audit_logs(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<AuditReportResponse>> {
    let requester_id = require_audit_management_access(&headers).await?;

    // Log the action of viewing the audit report
    log_audit(
        &pool,
        Some(requester_id),
        "VIEW_AUDIT_REPORT",
        "audit_logs",
        requester_id,
        json!({
            "filters": {
                "start_date": query.start_date,
                "end_date": query.end_date,
                "action_type": query.action_type,
                "user_id": query.user_id,
            }
        }),
    )
    .await?;

    let limit = query.limit.unwrap_or(50).max(1).min(200);
    let offset = query.offset.unwrap_or(0).max(0);

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
        SELECT
            al.id,
            al.user_id,
            al.action,
            al.entity_type,
            al.entity_id,
            al.changes,
            al.created_at,
            al.previous_hash,
            al.entry_hash,
            u.first_name AS first_name,
            u.last_name AS last_name
        FROM audit_logs al
        LEFT JOIN users u ON al.user_id = u.id
        WHERE 1=1
        "#,
    );

    if let Some(start) = query.start_date {
        query_builder.push(" AND al.created_at >= ");
        query_builder.push_bind(start);
    }
    if let Some(end) = query.end_date {
        query_builder.push(" AND al.created_at <= ");
        query_builder.push_bind(end);
    }
    if let Some(action) = query.action_type {
        query_builder.push(" AND al.action = ");
        query_builder.push_bind(action);
    }
    if let Some(actor) = query.user_id {
        query_builder.push(" AND al.user_id = ");
        query_builder.push_bind(actor);
    }
    if let Some(entity) = query.entity_id {
        query_builder.push(" AND al.entity_id = ");
        query_builder.push_bind(entity);
    }
    if let Some(etype) = query.entity_type {
        query_builder.push(" AND al.entity_type = ");
        query_builder.push_bind(etype);
    }

    query_builder.push(" ORDER BY al.created_at DESC LIMIT ");
    query_builder.push_bind(limit + 1);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<sqlx::postgres::PgRow> = query_builder
        .build()
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut entries = Vec::new();
    for row in rows {
        let first_name: Option<String> = row
            .try_get("first_name")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let last_name: Option<String> = row
            .try_get("last_name")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let user_name = {
            let full_name = format!(
                "{} {}",
                first_name.unwrap_or_default(),
                last_name.unwrap_or_default()
            )
            .trim()
            .to_string();
            if full_name.is_empty() {
                None
            } else {
                Some(full_name)
            }
        };

        entries.push(AuditLogResponse {
            id: row
                .try_get("id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            user_id: row
                .try_get("user_id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            user_name,
            action: row
                .try_get("action")
                .map_err(|e| AppError::Database(e.to_string()))?,
            entity_type: row
                .try_get("entity_type")
                .map_err(|e| AppError::Database(e.to_string()))?,
            entity_id: row
                .try_get("entity_id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            changes: row
                .try_get("changes")
                .map_err(|e| AppError::Database(e.to_string()))?,
            created_at: row
                .try_get("created_at")
                .map_err(|e| AppError::Database(e.to_string()))?,
            previous_hash: row
                .try_get("previous_hash")
                .map_err(|e| AppError::Database(e.to_string()))?,
            entry_hash: row
                .try_get("entry_hash")
                .map_err(|e| AppError::Database(e.to_string()))?,
        });
    }

    let has_more = entries.len() as i64 > limit;
    if has_more {
        entries.pop();
    }

    Ok(Json(AuditReportResponse {
        entries,
        limit,
        offset,
        has_more,
    }))
}

async fn export_audit_report(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<ExportApprovalState>> {
    let requester_id = require_audit_management_access(&headers).await?;

    let export_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO audit_export_requests (id, requested_by, status, note)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(export_id)
    .bind(requester_id)
    .bind("pending_approval")
    .bind("Export requires secondary approval by another authorized role")
    .execute(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        Some(requester_id),
        "EXPORT_REQUESTED",
        "audit_logs",
        export_id,
        json!({
            "workflow": "four_eyes_approval",
            "status": "pending_approval"
        }),
    )
    .await?;

    Ok(Json(ExportApprovalState {
        export_id,
        status: "pending_approval".to_string(),
        requested_by: requester_id,
        note: "Export requires secondary approval by another authorized role".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct ChainVerificationResult {
    pub is_valid: bool,
    pub broken_at_id: Option<Uuid>,
    pub message: String,
}

// Simple chain verification API that verifies all hashes
async fn verify_audit_chain(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<ChainVerificationResult>> {
    let requester_id = require_audit_management_access(&headers).await?;

    log_audit(
        &pool,
        Some(requester_id),
        "VERIFY_CHAIN",
        "audit_logs",
        requester_id,
        json!({}),
    )
    .await?;

    let logs = sqlx::query(
        "SELECT id, user_id, action, entity_type, entity_id, changes, previous_hash, entry_hash
         FROM audit_logs ORDER BY created_at ASC, id ASC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut expected_previous_hash = "GENESIS".to_string();

    for log in logs {
        let id: Uuid = log
            .try_get("id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let user_id: Option<Uuid> = log
            .try_get("user_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let action: String = log
            .try_get("action")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let entity_type: String = log
            .try_get("entity_type")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let entity_id: Option<Uuid> = log
            .try_get("entity_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let changes: serde_json::Value = log
            .try_get("changes")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let previous_hash: Option<String> = log
            .try_get("previous_hash")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let entry_hash: Option<String> = log
            .try_get("entry_hash")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let actual_previous = previous_hash.unwrap_or_default();
        if actual_previous != expected_previous_hash {
            return Ok(Json(ChainVerificationResult {
                is_valid: false,
                broken_at_id: Some(id),
                message: "Chain broken: Previous hash link mismatch".to_string(),
            }));
        }

        let computed_hash = recompute_entry_hash(
            user_id,
            &action,
            &entity_type,
            entity_id,
            &changes,
            &actual_previous,
        )?;

        if entry_hash.unwrap_or_default() != computed_hash {
            return Ok(Json(ChainVerificationResult {
                is_valid: false,
                broken_at_id: Some(id),
                message: "Tamper detected: Recomputed payload hash does not match saved entry_hash"
                    .to_string(),
            }));
        }

        expected_previous_hash = computed_hash;
    }

    Ok(Json(ChainVerificationResult {
        is_valid: true,
        broken_at_id: None,
        message: "Audit chain is fully continuous and tamper-free".to_string(),
    }))
}

pub fn audit_log_routes() -> Router<PgPool> {
    Router::new()
        .route("/audit-logs", get(get_audit_logs))
        .route("/audit-logs/export", post(export_audit_report))
        .route("/audit-logs/verify", get(verify_audit_chain))
}
