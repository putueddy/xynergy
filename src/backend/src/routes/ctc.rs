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

    if claims.role == "department_head" {
        log_audit(
            &pool,
            Some(user_id),
            "ACCESS_DENIED",
            "ctc_components",
            resource_id,
            json!({
                "reason": "insufficient_permissions",
                "attempted_role": claims.role,
            }),
        )
        .await
        .ok();
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    Ok(Json(json!({
        "resource_id": resource_id,
        "components": {},
        "note": "CTC component details endpoint placeholder for RBAC enforcement"
    })))
}

pub fn ctc_routes() -> Router<PgPool> {
    Router::new().route("/ctc/:resource_id/components", get(get_ctc_components))
}
