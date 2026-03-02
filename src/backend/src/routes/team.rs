use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::audit_log::user_claims_from_headers;
use crate::services::budget_service::{
    compute_budget_breakdown, compute_department_budget_utilization, BudgetBreakdownResponse,
    DepartmentBudgetSummaryResponse,
};
use crate::services::begin_rls_transaction;
use crate::services::team_service::{
    get_capacity_report_in_transaction, get_team_members_in_transaction, CapacityReportResponse,
    TeamMemberResponse,
};
use crate::services::{audit_payload, log_audit, user_id_from_headers};

#[derive(Debug, Deserialize)]
struct SetDepartmentBudgetRequest {
    budget_period: String,
    total_budget_idr: i64,
    alert_threshold_pct: Option<i32>,
    department_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct DepartmentBudgetQuery {
    period: String,
    department_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct BudgetBreakdownQuery {
    period: Option<String>,
    start_period: Option<String>,
    end_period: Option<String>,
    department_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct CapacityReportQuery {
    start_date: NaiveDate,
    end_date: NaiveDate,
}

#[derive(Debug, Serialize)]
struct DepartmentBudgetUpsertResponse {
    id: Uuid,
    department_id: Uuid,
    budget_period: String,
    total_budget_idr: i64,
    alert_threshold_pct: i32,
}

fn assert_budget_access_role(role: &str) -> Result<()> {
    if role != "department_head" && role != "hr" && role != "admin" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }
    Ok(())
}

async fn session_department_id(tx: &mut Transaction<'_, Postgres>) -> Result<Uuid> {
    let dept_id_str: String = sqlx::query_scalar("SELECT current_setting('app.current_department_id', true)")
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if dept_id_str.is_empty() {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    Uuid::parse_str(&dept_id_str)
        .map_err(|_| AppError::Internal("Invalid department_id in session".to_string()))
}

async fn resolve_department_id(
    tx: &mut Transaction<'_, Postgres>,
    role: &str,
    requested_department_id: Option<Uuid>,
) -> Result<Uuid> {
    if role == "department_head" {
        return session_department_id(tx).await;
    }

    if role == "hr" || role == "admin" {
        if let Some(department_id) = requested_department_id {
            return Ok(department_id);
        }
        return session_department_id(tx).await;
    }

    Err(AppError::Forbidden("Insufficient permissions".to_string()))
}

fn parse_yyyy_mm(period: &str, field_name: &str) -> Result<(i32, u32)> {
    let parts: Vec<&str> = period.split('-').collect();
    if parts.len() != 2 || parts[0].len() != 4 || parts[1].len() != 2 {
        return Err(AppError::Validation(format!(
            "{} must be in YYYY-MM format",
            field_name
        )));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| AppError::Validation(format!("Invalid year in {}", field_name)))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| AppError::Validation(format!("Invalid month in {}", field_name)))?;

    if month == 0 || month > 12 {
        return Err(AppError::Validation(format!(
            "Month in {} must be between 01 and 12",
            field_name
        )));
    }

    let normalized = format!("{:04}-{:02}", year, month);
    if normalized != period {
        return Err(AppError::Validation(format!(
            "{} must be in YYYY-MM format",
            field_name
        )));
    }

    Ok((year, month))
}

fn next_period(period: &str) -> Result<String> {
    let (year, month) = parse_yyyy_mm(period, "period")?;
    if month == 12 {
        Ok(format!("{:04}-{:02}", year + 1, 1))
    } else {
        Ok(format!("{:04}-{:02}", year, month + 1))
    }
}

fn build_period_range(start_period: &str, end_period: &str) -> Result<Vec<String>> {
    let (start_year, start_month) = parse_yyyy_mm(start_period, "start_period")?;
    let (end_year, end_month) = parse_yyyy_mm(end_period, "end_period")?;
    if (start_year, start_month) > (end_year, end_month) {
        return Err(AppError::Validation(
            "start_period cannot be after end_period".to_string(),
        ));
    }

    let mut periods = Vec::new();
    let mut current = format!("{:04}-{:02}", start_year, start_month);
    let end = format!("{:04}-{:02}", end_year, end_month);
    while current <= end {
        periods.push(current.clone());
        current = next_period(&current)?;
    }

    Ok(periods)
}

async fn get_team(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamMemberResponse>>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let _user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    assert_budget_access_role(&claims.role)?;

    let mut tx = begin_rls_transaction(&pool, &headers).await?;

    let department_filter = if claims.role == "department_head" {
        Some(resolve_department_id(&mut tx, &claims.role, None).await?)
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

async fn get_capacity_report(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<CapacityReportQuery>,
) -> Result<Json<CapacityReportResponse>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let _user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    assert_budget_access_role(&claims.role)?;

    if query.start_date > query.end_date {
        return Err(AppError::Validation(
            "Start date cannot be after end date.".to_string(),
        ));
    }

    let mut tx = begin_rls_transaction(&pool, &headers).await?;

    let department_filter = if claims.role == "department_head" {
        Some(resolve_department_id(&mut tx, &claims.role, None).await?)
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

async fn set_department_budget(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(req): Json<SetDepartmentBudgetRequest>,
) -> Result<Json<DepartmentBudgetUpsertResponse>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    assert_budget_access_role(&claims.role)?;

    parse_yyyy_mm(&req.budget_period, "budget_period")?;

    if req.total_budget_idr <= 0 {
        return Err(AppError::Validation(
            "total_budget_idr must be greater than 0".to_string(),
        ));
    }

    let alert_threshold_pct = req.alert_threshold_pct.unwrap_or(80);
    if !(50..=100).contains(&alert_threshold_pct) {
        return Err(AppError::Validation(
            "alert_threshold_pct must be between 50 and 100".to_string(),
        ));
    }

    let mut tx = begin_rls_transaction(&pool, &headers).await?;
    let department_id =
        resolve_department_id(&mut tx, &claims.role, req.department_id).await?;

    let existing_row = sqlx::query(
        "SELECT id, department_id, budget_period, total_budget_idr, alert_threshold_pct
         FROM department_budgets
         WHERE department_id = $1 AND budget_period = $2",
    )
    .bind(department_id)
    .bind(&req.budget_period)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let row = sqlx::query(
        "INSERT INTO department_budgets (department_id, budget_period, total_budget_idr, alert_threshold_pct)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (department_id, budget_period)
         DO UPDATE SET
             total_budget_idr = EXCLUDED.total_budget_idr,
             alert_threshold_pct = EXCLUDED.alert_threshold_pct,
             updated_at = NOW()
         RETURNING id, department_id, budget_period, total_budget_idr, alert_threshold_pct",
    )
    .bind(department_id)
    .bind(&req.budget_period)
    .bind(req.total_budget_idr)
    .bind(i16::try_from(alert_threshold_pct).map_err(|_| {
        AppError::Validation("alert_threshold_pct is out of range".to_string())
    })?)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let id: Uuid = row
        .try_get("id")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let department_id: Uuid = row
        .try_get("department_id")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let budget_period: String = row
        .try_get("budget_period")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let total_budget_idr: i64 = row
        .try_get("total_budget_idr")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let alert_threshold_raw: i16 = row
        .try_get("alert_threshold_pct")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let alert_threshold_pct = i32::from(alert_threshold_raw);

    let before_json = if let Some(before) = existing_row {
        let before_id: Uuid = before
            .try_get("id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let before_department_id: Uuid = before
            .try_get("department_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let before_budget_period: String = before
            .try_get("budget_period")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let before_total_budget_idr: i64 = before
            .try_get("total_budget_idr")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let before_alert_threshold_raw: i16 = before
            .try_get("alert_threshold_pct")
            .map_err(|e| AppError::Database(e.to_string()))?;

        Some(json!({
            "id": before_id,
            "department_id": before_department_id,
            "budget_period": before_budget_period,
            "total_budget_idr": before_total_budget_idr,
            "alert_threshold_pct": i32::from(before_alert_threshold_raw),
        }))
    } else {
        None
    };

    let after_json = json!({
        "id": id,
        "department_id": department_id,
        "budget_period": budget_period,
        "total_budget_idr": total_budget_idr,
        "alert_threshold_pct": alert_threshold_pct,
    });

    let user_id = user_id_from_headers(&headers)?;
    log_audit(
        &pool,
        user_id,
        "upsert",
        "department_budget",
        id,
        audit_payload(before_json, Some(after_json)),
    )
    .await?;

    Ok(Json(DepartmentBudgetUpsertResponse {
        id,
        department_id,
        budget_period,
        total_budget_idr,
        alert_threshold_pct,
    }))
}

async fn get_department_budget_summary(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<DepartmentBudgetQuery>,
) -> Result<Json<DepartmentBudgetSummaryResponse>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    assert_budget_access_role(&claims.role)?;

    parse_yyyy_mm(&query.period, "period")?;

    let mut tx = begin_rls_transaction(&pool, &headers).await?;
    let department_id =
        resolve_department_id(&mut tx, &claims.role, query.department_id).await?;

    let summary =
        compute_department_budget_utilization(&mut tx, department_id, &query.period).await?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(summary))
}

async fn get_budget_breakdown(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<BudgetBreakdownQuery>,
) -> Result<Json<BudgetBreakdownResponse>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    assert_budget_access_role(&claims.role)?;

    let periods = match (
        query.period.as_ref(),
        query.start_period.as_ref(),
        query.end_period.as_ref(),
    ) {
        (Some(period), None, None) => {
            parse_yyyy_mm(period, "period")?;
            vec![period.clone()]
        }
        (None, Some(start_period), Some(end_period)) => {
            build_period_range(start_period, end_period)?
        }
        _ => {
            return Err(AppError::Validation(
                "Invalid query: provide either 'period' for single-period mode, or both 'start_period' and 'end_period' for range mode.".to_string(),
            ))
        }
    };

    let mut tx = begin_rls_transaction(&pool, &headers).await?;
    let department_id =
        resolve_department_id(&mut tx, &claims.role, query.department_id).await?;
    let breakdown = compute_budget_breakdown(&mut tx, department_id, periods).await?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(breakdown))
}

pub fn team_routes() -> Router<PgPool> {
    Router::new()
        .route("/team", get(get_team))
        .route("/team/capacity-report", get(get_capacity_report))
        .route("/team/budget", post(set_department_budget).get(get_department_budget_summary))
        .route("/team/budget/breakdown", get(get_budget_breakdown))
}
