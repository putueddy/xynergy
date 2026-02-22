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
                .await
                .ok();
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
            .await
            .ok();
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
            .await
            .ok();
        }
    }

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(json!({
        "resource_id": resource_id,
        "components": {},
        "note": "CTC component details endpoint with RLS and Audit logging"
    })))
}

pub fn ctc_routes() -> Router<PgPool> {
    Router::new().route("/ctc/:resource_id/components", get(get_ctc_components))
}
