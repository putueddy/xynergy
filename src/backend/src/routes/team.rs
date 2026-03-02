use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use chrono::NaiveDate;
use serde::Deserialize;

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::audit_log::user_claims_from_headers;
use crate::services::begin_rls_transaction;
use crate::services::team_service::{
    get_capacity_report_in_transaction, get_team_members_in_transaction, CapacityReportResponse,
    TeamMemberResponse,
};

async fn get_team(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamMemberResponse>>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let _user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    if claims.role != "department_head" && claims.role != "hr" && claims.role != "admin" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    let mut tx = begin_rls_transaction(&pool, &headers).await?;

    let department_filter = if claims.role == "department_head" {
        // Read from session variable already set by begin_rls_transaction
        // (avoids a redundant SELECT department_id FROM users query)
        let dept_id_str: String =
            sqlx::query_scalar("SELECT current_setting('app.current_department_id', true)")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        if dept_id_str.is_empty() {
            return Err(AppError::Forbidden("Insufficient permissions".to_string()));
        }

        Some(
            Uuid::parse_str(&dept_id_str)
                .map_err(|_| AppError::Internal("Invalid department_id in session".to_string()))?,
        )
    } else {
        None
    };

    let team_members =
        get_team_members_in_transaction(&mut tx, department_filter, &claims.role).await?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(team_members))
}

#[derive(Debug, Deserialize)]
struct CapacityReportQuery {
    start_date: NaiveDate,
    end_date: NaiveDate,
}

async fn get_capacity_report(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<CapacityReportQuery>,
) -> Result<Json<CapacityReportResponse>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let _user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    if claims.role != "department_head" && claims.role != "hr" && claims.role != "admin" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    if query.start_date > query.end_date {
        return Err(AppError::Validation(
            "Start date cannot be after end date.".to_string(),
        ));
    }

    let mut tx = begin_rls_transaction(&pool, &headers).await?;

    let department_filter = if claims.role == "department_head" {
        let dept_id_str: String =
            sqlx::query_scalar("SELECT current_setting('app.current_department_id', true)")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        if dept_id_str.is_empty() {
            return Err(AppError::Forbidden("Insufficient permissions".to_string()));
        }

        Some(
            Uuid::parse_str(&dept_id_str)
                .map_err(|_| AppError::Internal("Invalid department_id in session".to_string()))?,
        )
    } else {
        None
    };

    let report = get_capacity_report_in_transaction(
        &mut tx,
        department_filter,
        query.start_date,
        query.end_date,
    )
    .await?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(report))
}

pub fn team_routes() -> Router<PgPool> {
    Router::new()
        .route("/team", get(get_team))
        .route("/team/capacity-report", get(get_capacity_report))
}
