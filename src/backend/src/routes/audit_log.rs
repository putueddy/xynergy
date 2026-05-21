use axum::{
    body::Bytes,
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::compliance_audit_report::{
    build_watermark, clamp_limit, clamp_offset, generate_report, validate_date_order,
    AuditReportType, ExportWatermark, ReportFilters,
};
use crate::services::ComplianceAuditReport;
use crate::services::{
    audit_log::user_claims_from_headers, log_audit, log_audit_in_transaction, recompute_entry_hash,
};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark: Option<ExportWatermark>,
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

async fn require_audit_management_access_with_audit(
    pool: &PgPool,
    headers: &HeaderMap,
    action: &str,
    entity_type: &str,
) -> Result<Uuid> {
    let claims = match user_claims_from_headers(headers) {
        Ok(Some(claims)) => claims,
        Ok(None) => {
            log_audit(
                pool,
                None,
                "ACCESS_DENIED",
                entity_type,
                Uuid::nil(),
                json!({
                    "reason": "missing_token",
                    "action": action,
                }),
            )
            .await?;
            return Err(AppError::Authentication("Missing token".to_string()));
        }
        Err(err) => {
            log_audit(
                pool,
                None,
                "ACCESS_DENIED",
                entity_type,
                Uuid::nil(),
                json!({
                    "reason": "invalid_token",
                    "action": action,
                }),
            )
            .await?;
            return Err(err);
        }
    };
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(user_id) => user_id,
        Err(_) => {
            log_audit(
                pool,
                None,
                "ACCESS_DENIED",
                entity_type,
                Uuid::nil(),
                json!({
                    "reason": "invalid_subject",
                    "action": action,
                }),
            )
            .await?;
            return Err(AppError::Authentication("Invalid user ID".to_string()));
        }
    };

    if claims.role != "admin" && claims.role != "finance" {
        log_audit(
            pool,
            Some(user_id),
            "ACCESS_DENIED",
            entity_type,
            Uuid::nil(),
            json!({
                "reason": "insufficient_role",
                "attempted_role": claims.role,
                "action": action,
            }),
        )
        .await?;
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    Ok(user_id)
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

// ---------- Story 5.4 compliance audit report endpoint ----------

#[derive(Debug, Deserialize)]
pub struct ComplianceReportQuery {
    pub report_type: String,
    pub start_date: String,
    pub end_date: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub snapshot_at: Option<DateTime<Utc>>,
    pub user_id: Option<Uuid>,
    pub action_type: Option<String>,
}

fn normalize_action_type(action_type: Option<String>) -> Option<String> {
    action_type
        .map(|action| action.trim().to_string())
        .filter(|action| !action.is_empty())
}

fn parse_required_date(field: &str, value: &str) -> Result<NaiveDate> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!("{} is required", field)));
    }
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map_err(|_| AppError::Validation(format!("{} must be YYYY-MM-DD", field)))
}

async fn database_snapshot_at(pool: &PgPool) -> Result<DateTime<Utc>> {
    sqlx::query_scalar::<_, DateTime<Utc>>("SELECT CURRENT_TIMESTAMP")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

async fn get_compliance_audit_report(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<ComplianceReportQuery>,
) -> Result<Json<ComplianceAuditReport>> {
    let requester_id = require_audit_management_access_with_audit(
        &pool,
        &headers,
        "get_compliance_audit_report",
        "audit_report",
    )
    .await?;

    let report_type = AuditReportType::parse(query.report_type.trim())?;
    let start_date = parse_required_date("start_date", &query.start_date)?;
    let end_date = parse_required_date("end_date", &query.end_date)?;
    let action_type = normalize_action_type(query.action_type.clone());

    let limit = clamp_limit(query.limit);
    let offset = clamp_offset(query.offset);
    if offset > 0 && query.snapshot_at.is_none() {
        return Err(AppError::Validation(
            "snapshot_at is required when requesting report pages after the first page".to_string(),
        ));
    }
    let snapshot_at = match query.snapshot_at {
        Some(snapshot_at) => snapshot_at,
        None => database_snapshot_at(&pool).await?,
    };

    // Access logs are the only report that consumes user_id / action_type
    // filters today; reject them for other reports to keep semantics tight.
    if !matches!(report_type, AuditReportType::AccessLogs)
        && (query.user_id.is_some() || action_type.is_some())
    {
        return Err(AppError::Validation(
            "user_id and action_type filters are only supported for report_type=access_logs"
                .to_string(),
        ));
    }

    let filters = ReportFilters {
        report_type,
        start_date,
        end_date,
        limit,
        offset,
        snapshot_at,
        user_id: query.user_id,
        action_type: action_type.clone(),
    };

    let report = generate_report(&pool, filters).await?;
    let row_count = report.rows.len() as i64;

    log_audit(
        &pool,
        Some(requester_id),
        "COMPLIANCE_AUDIT_REPORT_GENERATED",
        "audit_report",
        requester_id,
        json!({
            "report_type": report.report_type,
            "start_date": report.start_date,
            "end_date": report.end_date,
            "snapshot_at": report.snapshot_at,
            "limit": report.limit,
            "offset": report.offset,
            "row_count": row_count,
            "filters": {
                "user_id": query.user_id,
                "action_type": action_type,
            }
        }),
    )
    .await?;

    Ok(Json(report))
}

// ---------- Export request (extended for report metadata) ----------

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ExportRequestPayload {
    pub report_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub snapshot_at: Option<DateTime<Utc>>,
    pub user_id: Option<Uuid>,
    pub action_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn parse_export_payload(body: Bytes) -> Result<ExportRequestPayload> {
    if body.iter().all(|b| b.is_ascii_whitespace()) {
        return Ok(ExportRequestPayload::default());
    }

    serde_json::from_slice::<ExportRequestPayload>(&body)
        .map_err(|_| AppError::Validation("Invalid export request payload".to_string()))
}

async fn lookup_email(pool: &PgPool, user_id: Uuid) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

async fn export_audit_report(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ExportApprovalState>> {
    let requester_id = require_audit_management_access_with_audit(
        &pool,
        &headers,
        "export_audit_report",
        "audit_export_request",
    )
    .await?;

    let payload = parse_export_payload(body)?;
    let action_type = normalize_action_type(payload.action_type.clone());
    if payload.report_type.is_none()
        && (payload.start_date.is_some()
            || payload.end_date.is_some()
            || payload.snapshot_at.is_some()
            || payload.user_id.is_some()
            || action_type.is_some()
            || payload.limit.is_some()
            || payload.offset.is_some())
    {
        return Err(AppError::Validation(
            "report_type is required when export report filters are provided".to_string(),
        ));
    }

    let export_id = Uuid::new_v4();
    let requested_at = database_snapshot_at(&pool).await?;
    let requester_email = lookup_email(&pool, requester_id).await;

    let (report_type, filters_json, watermark) = if let Some(report_type_raw) = payload.report_type
    {
        let report_type = AuditReportType::parse(report_type_raw.trim())?;
        if payload.limit.is_some() || payload.offset.is_some() {
            return Err(AppError::Validation(
                "limit and offset are not supported for export requests; exports include all matching rows"
                    .to_string(),
            ));
        }
        let start_date = payload
            .start_date
            .as_deref()
            .map(|s| parse_required_date("start_date", s))
            .transpose()?
            .ok_or_else(|| {
                AppError::Validation("start_date is required when report_type is provided".into())
            })?;
        let end_date = payload
            .end_date
            .as_deref()
            .map(|s| parse_required_date("end_date", s))
            .transpose()?
            .ok_or_else(|| {
                AppError::Validation("end_date is required when report_type is provided".into())
            })?;
        let snapshot_at = payload.snapshot_at.unwrap_or(requested_at);
        if snapshot_at > requested_at {
            return Err(AppError::Validation(
                "snapshot_at cannot be in the future".to_string(),
            ));
        }

        let filters = ReportFilters {
            report_type,
            start_date,
            end_date,
            limit: clamp_limit(None),
            offset: clamp_offset(None),
            snapshot_at,
            user_id: payload.user_id,
            action_type,
        };
        validate_date_order(start_date, end_date)?;
        if !matches!(report_type, AuditReportType::AccessLogs)
            && (filters.user_id.is_some() || filters.action_type.is_some())
        {
            return Err(AppError::Validation(
                "user_id and action_type filters are only supported for report_type=access_logs"
                    .to_string(),
            ));
        }

        let watermark = build_watermark(
            export_id,
            requester_id,
            requester_email.clone(),
            requested_at,
            &filters,
        );

        let filters_json = json!({
            "report_type": filters.report_type.as_str(),
            "start_date": filters.start_date,
            "end_date": filters.end_date,
            "snapshot_at": filters.snapshot_at,
            "scope": "all_matching_rows",
            "user_id": filters.user_id,
            "action_type": filters.action_type,
        });

        (
            Some(report_type.as_str().to_string()),
            Some(filters_json),
            Some(watermark),
        )
    } else {
        // Legacy generic export request: keep the pre-5.4 empty-body contract
        // for existing callers. Report-aware exports use typed metadata and
        // watermark fields above.
        (None, None, None)
    };

    let watermark_json = watermark
        .as_ref()
        .map(|w| serde_json::to_value(w).unwrap_or(serde_json::Value::Null));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    sqlx::query(
        "INSERT INTO audit_export_requests (id, requested_by, status, note, report_type, filters, watermark)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(export_id)
    .bind(requester_id)
    .bind("pending_approval")
    .bind("Export requires secondary approval by another authorized role")
    .bind(report_type.as_deref())
    .bind(filters_json.clone())
    .bind(watermark_json.clone())
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut audit_payload = json!({
        "workflow": "four_eyes_approval",
        "status": "pending_approval",
    });
    if let Some(rt) = report_type.as_deref() {
        audit_payload["report_type"] = serde_json::Value::String(rt.to_string());
    }
    if let Some(filters) = filters_json.clone() {
        audit_payload["filters"] = filters;
    }
    if watermark.is_some() {
        audit_payload["watermark_attached"] = serde_json::Value::Bool(true);
    }

    log_audit_in_transaction(
        &mut tx,
        Some(requester_id),
        "EXPORT_REQUESTED",
        "audit_export_request",
        export_id,
        audit_payload,
    )
    .await?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(ExportApprovalState {
        export_id,
        status: "pending_approval".to_string(),
        requested_by: requester_id,
        note: "Export requires secondary approval by another authorized role".to_string(),
        report_type,
        filters: filters_json,
        watermark,
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
        .route("/audit-logs/reports", get(get_compliance_audit_report))
        .route("/audit-logs/export", post(export_audit_report))
        .route("/audit-logs/verify", get(verify_audit_chain))
}
