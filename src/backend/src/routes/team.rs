use axum::{extract::State, http::HeaderMap, routing::get, Json, Router};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::audit_log::user_claims_from_headers;
use crate::services::team_service::{get_team_members_in_transaction, TeamMemberResponse};
use crate::services::{begin_rls_transaction, log_audit};

async fn get_team(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamMemberResponse>>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    if claims.role != "department_head" && claims.role != "hr" && claims.role != "admin" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    let mut tx = begin_rls_transaction(&pool, &headers).await?;

    let department_filter = if claims.role == "department_head" {
        // Read from session variable already set by begin_rls_transaction
        // (avoids a redundant SELECT department_id FROM users query)
        let dept_id_str: String = sqlx::query_scalar(
            "SELECT current_setting('app.current_department_id', true)"
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if dept_id_str.is_empty() {
            return Err(AppError::Forbidden("Insufficient permissions".to_string()));
        }

        Some(Uuid::parse_str(&dept_id_str)
            .map_err(|_| AppError::Internal("Invalid department_id in session".to_string()))?)
    } else {
        None
    };

    let team_members =
        get_team_members_in_transaction(&mut tx, department_filter, &claims.role).await?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let audit_entity_id = department_filter.unwrap_or(user_id);
    log_audit(
        &pool,
        Some(user_id),
        "team_view",
        "department",
        audit_entity_id,
        json!({"action": "view_team", "role": claims.role, "department_filter": department_filter.map(|d| d.to_string())}),
    )
    .await?;

    Ok(Json(team_members))
}

pub fn team_routes() -> Router<PgPool> {
    Router::new().route("/team", get(get_team))
}
