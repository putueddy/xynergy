//! Role-based dashboard route.
//!
//! Implements `GET /api/v1/dashboard` (Story 6.1). The handler does only
//! authentication and role gating; per-role data assembly lives in
//! `services::dashboard_service::build_dashboard`.

use axum::{extract::State, http::HeaderMap, routing::get, Json, Router};
use sqlx::PgPool;

use crate::error::{AppError, Result};
use crate::services::audit_log::user_claims_from_headers;
use crate::services::dashboard_service::{build_dashboard, RoleDashboardResponse};

pub fn dashboard_routes() -> Router<PgPool> {
    Router::new().route("/dashboard", get(get_role_dashboard))
}

async fn get_role_dashboard(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<RoleDashboardResponse>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    let response = build_dashboard(&pool, &headers, &claims).await?;
    Ok(Json(response))
}
