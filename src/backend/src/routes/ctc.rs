use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::{audit_log::user_claims_from_headers, log_audit};

async fn get_ctc_components(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(resource_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    // Established DB session with RLS policies configured
    let mut tx = crate::services::begin_rls_transaction(&pool, &headers).await?;

    // Find the resource via RLS-restricted transaction
    let resource = sqlx::query!(
        "SELECT id, department_id FROM resources WHERE id = $1",
        resource_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Manual check for defense-in-depth (needed if RLS is bypassed e.g. superuser or test env)
    if let Some(res) = &resource {
        if claims.role == "department_head" {
            let user_dept_query =
                sqlx::query!("SELECT department_id FROM users WHERE id = $1", user_id)
                    .fetch_optional(&pool)
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;

            let mut check_failed = true;
            if let Some(user_row) = user_dept_query {
                if res.department_id == user_row.department_id && res.department_id.is_some() {
                    check_failed = false;
                }
            }

            if check_failed {
                // Appears as RLS breach
                log_audit(
                    &pool,
                    Some(user_id),
                    "ACCESS_DENIED",
                    "ctc_components",
                    resource_id,
                    json!({
                        "reason": "cross_department_access_denied_by_rls",
                        "attempted_role": claims.role,
                    }),
                )
                .await?;
                return Err(AppError::Forbidden(
                    "Access denied by department isolation policy".to_string(),
                ));
            }
        }
    }

    if resource.is_none() {
        if claims.role == "department_head" {
            log_audit(
                &pool,
                Some(user_id),
                "ACCESS_DENIED",
                "ctc_components",
                resource_id,
                json!({
                    "reason": "cross_department_access_denied_by_rls",
                    "attempted_role": claims.role,
                }),
            )
            .await?;
            return Err(AppError::Forbidden(
                "Access denied by department isolation policy".to_string(),
            ));
        }

        return Err(AppError::NotFound("Resource not found".to_string()));
    }

    // HR cross-department view logging
    if claims.role == "hr" {
        // Verify if it's actually cross department
        let user_dept_query =
            sqlx::query!("SELECT department_id FROM users WHERE id = $1", user_id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        let mut is_cross_dept = true;
        if let Some(user_row) = user_dept_query {
            if let Some(res_row) = &resource {
                is_cross_dept = user_row.department_id != res_row.department_id;
            }
        }

        if is_cross_dept {
            log_audit(
                &pool,
                Some(user_id),
                "CTC_VIEW_CROSS_DEPT",
                "ctc_components",
                resource_id,
                json!({
                    "role": "hr",
                    "action": "cross_department_audit_log",
                }),
            )
            .await?;
        }
    }

    // HR cross-department view logging done above if role is hr...
    // Now perform standard CTC view logging for any authorized user
    log_audit(
        &pool,
        Some(user_id),
        "VIEW",
        "ctc_components",
        resource_id,
        json!({
            "action": "view_ctc",
            "role": claims.role,
        }),
    )
    .await?;

    let components = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT components FROM ctc_records WHERE resource_id = $1 ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(resource_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .unwrap_or_else(|| json!({}));

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(json!({
        "resource_id": resource_id,
        "components": components,
        "note": "CTC component details endpoint with RLS and audit logging"
    })))
}

#[derive(serde::Deserialize)]
pub struct UpdateCtcRequest {
    pub components: serde_json::Value,
    pub reason: String,
}

async fn update_ctc_components(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(resource_id): Path<Uuid>,
    Json(payload): Json<UpdateCtcRequest>,
) -> Result<Json<serde_json::Value>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    if payload.reason.trim().is_empty() {
        return Err(AppError::Validation(
            "Update reason is required".to_string(),
        ));
    }

    let mut tx = crate::services::begin_rls_transaction(&pool, &headers).await?;

    // RLS validation (similar to view)
    let resource = sqlx::query!("SELECT id FROM resources WHERE id = $1", resource_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if resource.is_none() {
        return Err(AppError::NotFound("Resource not found".to_string()));
    }

    let before_state = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT components FROM ctc_records WHERE resource_id = $1 ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(resource_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .unwrap_or_else(|| json!({}));
    let after_state = payload.components.clone();

    sqlx::query(
        "INSERT INTO ctc_records (resource_id, components, updated_by, reason)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (resource_id)
         DO UPDATE SET components = EXCLUDED.components, updated_by = EXCLUDED.updated_by, reason = EXCLUDED.reason, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(resource_id)
    .bind(&after_state)
    .bind(user_id)
    .bind(&payload.reason)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Log the mutation with before/after snapshots and reason string
    log_audit(
        &pool,
        Some(user_id),
        "UPDATE",
        "ctc_components",
        resource_id,
        json!({
            "action": "update_ctc",
            "reason": payload.reason,
            "before": before_state,
            "after": after_state
        }),
    )
    .await?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(json!({
        "resource_id": resource_id,
        "components": after_state,
        "note": "CTC mutation recorded with snapshot audit log"
    })))
}

pub fn ctc_routes() -> Router<PgPool> {
    Router::new()
        .route("/ctc/:resource_id/components", get(get_ctc_components))
        .route(
            "/ctc/:resource_id/components",
            axum::routing::put(update_ctc_components),
        )
}
