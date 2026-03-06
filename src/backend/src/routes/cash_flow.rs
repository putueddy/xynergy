use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::{
    audit_log::user_claims_from_headers, audit_payload, cash_flow_service, log_audit,
};

#[derive(Debug, Deserialize)]
pub struct CreateCashFlowEntryRequest {
    pub entry_type: String,
    pub category: String,
    pub amount_idr: i64,
    pub entry_date: chrono::NaiveDate,
    pub description: String,
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CashFlowEntryResponse {
    pub id: Uuid,
    pub entry_type: String,
    pub category: String,
    pub amount_idr: i64,
    pub entry_date: chrono::NaiveDate,
    pub description: String,
    pub project_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListCashFlowEntriesQuery {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub entry_type: Option<String>,
    pub project_id: Option<Uuid>,
}

async fn enforce_finance_access(
    pool: &PgPool,
    headers: &HeaderMap,
    action_name: &str,
    entity_type: &str,
) -> Result<Uuid> {
    let claims = user_claims_from_headers(headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".into()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID".into()))?;

    if !matches!(claims.role.as_str(), "finance" | "admin") {
        log_audit(
            pool,
            Some(user_id),
            "ACCESS_DENIED",
            entity_type,
            Uuid::nil(),
            serde_json::json!({
                "reason": "insufficient_role",
                "attempted_role": claims.role,
                "action": action_name,
            }),
        )
        .await
        .ok();
        return Err(AppError::Forbidden("Insufficient permissions".into()));
    }

    Ok(user_id)
}

async fn create_cash_flow_entry(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(req): Json<CreateCashFlowEntryRequest>,
) -> Result<Json<CashFlowEntryResponse>> {
    let user_id = enforce_finance_access(&pool, &headers, "create_cash_flow_entry", "cash_flow_entry")
        .await?;

    cash_flow_service::validate_cash_flow_entry(
        &req.entry_type,
        &req.category,
        req.amount_idr,
        &req.description,
    )?;

    if let Some(project_id) = req.project_id {
        cash_flow_service::validate_project_exists(&pool, project_id).await?;
    }

    let entry = sqlx::query_as::<_, CashFlowEntryResponse>(
        r#"INSERT INTO cash_flow_entries (entry_type, category, amount_idr, entry_date, description, project_id, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, entry_type, category, amount_idr, entry_date, description, project_id, created_by, created_at, updated_at"#,
    )
    .bind(&req.entry_type)
    .bind(&req.category)
    .bind(req.amount_idr)
    .bind(req.entry_date)
    .bind(&req.description)
    .bind(req.project_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let audit_changes = audit_payload(
        None,
        Some(serde_json::json!({
            "entry_type": entry.entry_type,
            "category": entry.category,
            "amount_idr": entry.amount_idr,
            "entry_date": entry.entry_date,
            "description": entry.description,
            "project_id": entry.project_id,
            "created_by": entry.created_by,
        })),
    );

    log_audit(
        &pool,
        Some(user_id),
        "create",
        "cash_flow_entry",
        entry.id,
        audit_changes,
    )
    .await?;

    Ok(Json(entry))
}

async fn list_cash_flow_entries(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<ListCashFlowEntriesQuery>,
) -> Result<Json<Vec<CashFlowEntryResponse>>> {
    let _user_id = enforce_finance_access(&pool, &headers, "list_cash_flow_entries", "cash_flow_entry")
        .await?;

    if let Some(entry_type) = &query.entry_type {
        if !matches!(entry_type.as_str(), "cash_in" | "cash_out") {
            return Err(AppError::Validation(
                "entry_type filter must be either 'cash_in' or 'cash_out'".into(),
            ));
        }
    }

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"SELECT id, entry_type, category, amount_idr, entry_date, description, project_id, created_by, created_at, updated_at
           FROM cash_flow_entries
           WHERE 1=1"#,
    );

    if let Some(start_date) = query.start_date {
        query_builder.push(" AND entry_date >= ");
        query_builder.push_bind(start_date);
    }

    if let Some(end_date) = query.end_date {
        query_builder.push(" AND entry_date <= ");
        query_builder.push_bind(end_date);
    }

    if let Some(entry_type) = query.entry_type {
        query_builder.push(" AND entry_type = ");
        query_builder.push_bind(entry_type);
    }

    if let Some(project_id) = query.project_id {
        query_builder.push(" AND project_id = ");
        query_builder.push_bind(project_id);
    }

    query_builder.push(" ORDER BY entry_date DESC, created_at DESC");

    let entries: Vec<CashFlowEntryResponse> = query_builder
        .build_query_as()
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(entries))
}

async fn list_project_cash_flow_entries(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<CashFlowEntryResponse>>> {
    let _user_id = enforce_finance_access(
        &pool,
        &headers,
        "list_project_cash_flow_entries",
        "cash_flow_entry",
    )
    .await?;

    cash_flow_service::validate_project_exists(&pool, id).await?;

    let entries = sqlx::query_as::<_, CashFlowEntryResponse>(
        r#"SELECT id, entry_type, category, amount_idr, entry_date, description, project_id, created_by, created_at, updated_at
           FROM cash_flow_entries
           WHERE project_id = $1
           ORDER BY entry_date DESC, created_at DESC"#,
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(entries))
}

pub fn cash_flow_routes() -> Router<PgPool> {
    Router::new()
        .route("/cash-flow/entries", get(list_cash_flow_entries).post(create_cash_flow_entry))
        .route("/projects/:id/cash-flow/entries", get(list_project_cash_flow_entries))
}
