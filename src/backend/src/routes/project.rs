use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, put},
    Json, Router,
};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::{
    audit_log::user_claims_from_headers, audit_payload, log_audit, user_id_from_headers,
};

/// Project response structure
#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub id: Uuid,
    pub name: String,
    pub client: Option<String>,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
    pub project_manager_id: Option<Uuid>,
    pub total_budget_idr: i64,
    pub budget_hr_idr: i64,
    pub budget_software_idr: i64,
    pub budget_hardware_idr: i64,
    pub budget_overhead_idr: i64,
}

/// Create project request
#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub client: Option<String>,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
    pub project_manager_id: Option<Uuid>,
    #[serde(default)]
    pub total_budget_idr: i64,
    #[serde(default)]
    pub budget_hr_idr: i64,
    #[serde(default)]
    pub budget_software_idr: i64,
    #[serde(default)]
    pub budget_hardware_idr: i64,
    #[serde(default)]
    pub budget_overhead_idr: i64,
}

/// Update project request
#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub client: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub status: Option<String>,
    pub project_manager_id: Option<Uuid>,
    pub total_budget_idr: Option<i64>,
    pub budget_hr_idr: Option<i64>,
    pub budget_software_idr: Option<i64>,
    pub budget_hardware_idr: Option<i64>,
    pub budget_overhead_idr: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SetProjectBudgetRequest {
    pub total_budget_idr: i64,
    pub budget_hr_idr: i64,
    pub budget_software_idr: i64,
    pub budget_hardware_idr: i64,
    pub budget_overhead_idr: i64,
}

#[derive(Debug, Serialize)]
pub struct ProjectBudgetResponse {
    pub project_id: Uuid,
    pub project_name: String,
    pub client: Option<String>,
    pub total_budget_idr: i64,
    pub budget_hr_idr: i64,
    pub budget_software_idr: i64,
    pub budget_hardware_idr: i64,
    pub budget_overhead_idr: i64,
    pub hr_pct: f64,
    pub software_pct: f64,
    pub hardware_pct: f64,
    pub overhead_pct: f64,
    pub spent_to_date_idr: i64,
    pub remaining_idr: i64,
}

// ── Resource Cost DTOs ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ProjectResourceCostResponse {
    pub project_id: Uuid,
    pub total_resource_cost_idr: i64,
    pub employees: Vec<EmployeeResourceCost>,
    pub monthly_breakdown: Vec<MonthlyResourceCost>,
}

#[derive(Debug, Serialize)]
pub struct EmployeeResourceCost {
    pub resource_id: Uuid,
    pub resource_name: String,
    pub daily_rate_idr: Option<i64>,
    pub days_allocated: i32,
    pub allocation_percentage: f64,
    pub total_cost_idr: i64,
    pub has_rate_change: bool,
    pub rate_change_note: Option<String>,
    pub missing_rate: bool,
}

#[derive(Debug, Serialize)]
pub struct MonthlyResourceCost {
    pub month: String, // "YYYY-MM"
    pub working_days: i32,
    pub cost_idr: i64,
}

// ── Expense DTOs ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProjectExpenseRequest {
    pub category: String,
    pub description: String,
    pub amount_idr: i64,
    pub expense_date: NaiveDate,
    pub vendor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectExpenseRequest {
    pub category: Option<String>,
    pub description: Option<String>,
    pub amount_idr: Option<i64>,
    pub expense_date: Option<NaiveDate>,
    pub vendor: Option<String>,
    pub edit_reason: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectExpenseResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub category: String,
    pub description: String,
    pub amount_idr: i64,
    pub expense_date: NaiveDate,
    pub vendor: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertProjectRevenueRequest {
    pub revenue_month: String,
    pub amount_idr: i64,
    #[serde(default)]
    pub override_erp: bool,
    pub source_reference: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectRevenueRowResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub revenue_month: NaiveDate,
    pub amount_idr: i64,
    pub source_type: String,
    pub source_reference: Option<String>,
    pub entered_by: Option<Uuid>,
    pub entry_date: NaiveDate,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ProjectRevenueGridResponse {
    pub project_id: Uuid,
    pub year: i32,
    pub months: Vec<MonthRevenueEntry>,
    pub ytd_total_idr: i64,
}

#[derive(Debug, Serialize)]
pub struct MonthRevenueEntry {
    pub month: u32,
    pub month_label: String,
    pub revenue_id: Option<Uuid>,
    pub amount_idr: i64,
    pub source_type: Option<String>,
    pub source_reference: Option<String>,
    pub entered_by: Option<Uuid>,
    pub entry_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct IngestErpRevenueRequest {
    pub revenue_month: String,
    pub amount_idr: i64,
    pub source_reference: String,
}

#[derive(Debug, Deserialize)]
struct ProjectRevenueQuery {
    #[serde(default = "default_revenue_year")]
    year: i32,
}

fn default_revenue_year() -> i32 {
    chrono::Utc::now().date_naive().year()
}

/// Assignable project response (minimal fields for assignment dropdown)
#[derive(Debug, Serialize)]
pub struct AssignableProjectResponse {
    pub id: Uuid,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
}

/// Get all projects
async fn get_projects(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProjectResponse>>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    let is_pm = claims.role == "project_manager";
    let user_id = Uuid::parse_str(&claims.sub).unwrap_or(Uuid::nil());

    let projects = if is_pm {
        sqlx::query_as!(
            ProjectResponse,
            "SELECT id, name, client, description, start_date, end_date, status, project_manager_id, total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr
             FROM projects WHERE project_manager_id = $1 ORDER BY start_date DESC",
            user_id
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
    } else {
        sqlx::query_as!(
            ProjectResponse,
            "SELECT id, name, client, description, start_date, end_date, status, project_manager_id, total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr
             FROM projects ORDER BY start_date DESC"
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
    };

    Ok(Json(projects))
}

/// Get assignable projects for assignment form dropdown
/// Role matrix: project_manager sees only active projects they manage;
/// department_head and admin see all active projects.
async fn get_assignable_projects(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<Vec<AssignableProjectResponse>>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    let can_assign = matches!(
        claims.role.as_str(),
        "admin" | "department_head" | "project_manager"
    );
    if !can_assign {
        log_audit(
            &pool,
            Some(user_id),
            "ACCESS_DENIED",
            "project",
            Uuid::nil(),
            serde_json::json!({
                "reason": "insufficient_permissions",
                "attempted_role": claims.role,
                "action": "get_assignable_projects",
            }),
        )
        .await
        .ok();
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    let projects = if claims.role == "project_manager" {
        sqlx::query_as!(
            AssignableProjectResponse,
            "SELECT id, name, start_date, end_date, status
             FROM projects
             WHERE status = 'Active' AND project_manager_id = $1
             ORDER BY name ASC",
            user_id
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
    } else {
        sqlx::query_as!(
            AssignableProjectResponse,
            "SELECT id, name, start_date, end_date, status
             FROM projects
             WHERE status = 'Active'
             ORDER BY name ASC"
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
    };

    Ok(Json(projects))
}

/// Get project by ID
async fn get_project(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ProjectResponse>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    let is_pm = claims.role == "project_manager";
    let user_id = Uuid::parse_str(&claims.sub).unwrap_or(Uuid::nil());

    let project = sqlx::query_as!(
        ProjectResponse,
        "SELECT id, name, client, description, start_date, end_date, status, project_manager_id, total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr
         FROM projects WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", id)))?;

    if is_pm && project.project_manager_id != Some(user_id) {
        // Enforce visibility constraint
        log_audit(
            &pool,
            Some(user_id),
            "ACCESS_DENIED",
            "project",
            id,
            serde_json::json!({
                "reason": "not_project_manager",
                "attempted_role": claims.role
            }),
        )
        .await
        .ok();

        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    Ok(Json(project))
}

/// Create a new project
async fn create_project(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(req): Json<CreateProjectRequest>,
) -> Result<Json<ProjectResponse>> {
    let user_id = user_id_from_headers(&headers)?;
    let claims = user_claims_from_headers(&headers)?;
    let assigned_project_manager_id =
        if claims.as_ref().is_some_and(|c| c.role == "project_manager")
            && req.project_manager_id.is_none()
        {
            user_id
        } else {
            req.project_manager_id
        };

    let has_budget_values = req.total_budget_idr != 0
        || req.budget_hr_idr != 0
        || req.budget_software_idr != 0
        || req.budget_hardware_idr != 0
        || req.budget_overhead_idr != 0;
    if has_budget_values {
        crate::services::project_service::validate_project_budget(
            req.total_budget_idr,
            req.budget_hr_idr,
            req.budget_software_idr,
            req.budget_hardware_idr,
            req.budget_overhead_idr,
        )?;
    }

    let audit_changes = audit_payload(
        None,
        Some(serde_json::json!({
            "name": req.name.clone(),
            "client": req.client.clone(),
            "description": req.description.clone(),
            "start_date": req.start_date,
            "end_date": req.end_date,
            "status": req.status.clone(),
            "project_manager_id": assigned_project_manager_id,
            "total_budget_idr": req.total_budget_idr,
            "budget_hr_idr": req.budget_hr_idr,
            "budget_software_idr": req.budget_software_idr,
            "budget_hardware_idr": req.budget_hardware_idr,
            "budget_overhead_idr": req.budget_overhead_idr,
        })),
    );

    let project = sqlx::query_as!(
        ProjectResponse,
        "INSERT INTO projects (name, client, description, start_date, end_date, status, project_manager_id, total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
         RETURNING id, name, client, description, start_date, end_date, status, project_manager_id, total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr",
        req.name,
        req.client,
        req.description,
        req.start_date,
        req.end_date,
        req.status,
        assigned_project_manager_id,
        req.total_budget_idr,
        req.budget_hr_idr,
        req.budget_software_idr,
        req.budget_hardware_idr,
        req.budget_overhead_idr
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        user_id,
        "create",
        "project",
        project.id,
        audit_changes,
    )
    .await?;

    Ok(Json(project))
}

/// Update a project
async fn update_project(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectResponse>> {
    // Check if project exists
    let existing = sqlx::query!(
        "SELECT id, name, client, description, start_date, end_date, status, project_manager_id, total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr FROM projects WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", id)))?;

    let before_name = existing.name.clone();
    let before_client = existing.client.clone();
    let before_description = existing.description.clone();
    let before_status = existing.status.clone();
    let before_start_date = existing.start_date;
    let before_end_date = existing.end_date;
    let before_manager_id = existing.project_manager_id;
    let before_total_budget_idr = existing.total_budget_idr;
    let before_budget_hr_idr = existing.budget_hr_idr;
    let before_budget_software_idr = existing.budget_software_idr;
    let before_budget_hardware_idr = existing.budget_hardware_idr;
    let before_budget_overhead_idr = existing.budget_overhead_idr;
    let after_name_default = existing.name;
    let after_client_default = existing.client;
    let after_description_default = existing.description;
    let after_status_default = existing.status;
    let after_start_default = existing.start_date;
    let after_end_default = existing.end_date;
    let after_manager_default = existing.project_manager_id;
    let after_total_budget_idr = existing.total_budget_idr;
    let after_budget_hr_idr = existing.budget_hr_idr;
    let after_budget_software_idr = existing.budget_software_idr;
    let after_budget_hardware_idr = existing.budget_hardware_idr;
    let after_budget_overhead_idr = existing.budget_overhead_idr;
    let merged_total_budget_idr = req.total_budget_idr.unwrap_or(after_total_budget_idr);
    let merged_budget_hr_idr = req.budget_hr_idr.unwrap_or(after_budget_hr_idr);
    let merged_budget_software_idr = req.budget_software_idr.unwrap_or(after_budget_software_idr);
    let merged_budget_hardware_idr = req.budget_hardware_idr.unwrap_or(after_budget_hardware_idr);
    let merged_budget_overhead_idr = req.budget_overhead_idr.unwrap_or(after_budget_overhead_idr);

    if req.total_budget_idr.is_some()
        || req.budget_hr_idr.is_some()
        || req.budget_software_idr.is_some()
        || req.budget_hardware_idr.is_some()
        || req.budget_overhead_idr.is_some()
    {
        crate::services::project_service::validate_project_budget(
            merged_total_budget_idr,
            merged_budget_hr_idr,
            merged_budget_software_idr,
            merged_budget_hardware_idr,
            merged_budget_overhead_idr,
        )?;
    }

    let audit_changes = audit_payload(
        Some(serde_json::json!({
            "name": before_name,
            "client": before_client,
            "description": before_description,
            "start_date": before_start_date,
            "end_date": before_end_date,
            "status": before_status,
            "project_manager_id": before_manager_id,
            "total_budget_idr": before_total_budget_idr,
            "budget_hr_idr": before_budget_hr_idr,
            "budget_software_idr": before_budget_software_idr,
            "budget_hardware_idr": before_budget_hardware_idr,
            "budget_overhead_idr": before_budget_overhead_idr,
        })),
        Some(serde_json::json!({
            "name": req.name.clone().unwrap_or_else(|| after_name_default),
            "client": req.client.clone().or(after_client_default),
            "description": req.description.clone().or(after_description_default),
            "start_date": req.start_date.unwrap_or(after_start_default),
            "end_date": req.end_date.unwrap_or(after_end_default),
            "status": req.status.clone().unwrap_or_else(|| after_status_default),
            "project_manager_id": req.project_manager_id.or(after_manager_default),
            "total_budget_idr": merged_total_budget_idr,
            "budget_hr_idr": merged_budget_hr_idr,
            "budget_software_idr": merged_budget_software_idr,
            "budget_hardware_idr": merged_budget_hardware_idr,
            "budget_overhead_idr": merged_budget_overhead_idr,
        })),
    );
    let user_id = user_id_from_headers(&headers)?;

    // Update with new values or keep existing
    let project = sqlx::query_as!(
        ProjectResponse,
        "UPDATE projects 
         SET name = COALESCE($1, name),
             client = COALESCE($2, client),
             description = COALESCE($3, description),
             start_date = COALESCE($4, start_date),
             end_date = COALESCE($5, end_date),
             status = COALESCE($6, status),
             project_manager_id = COALESCE($7, project_manager_id),
             total_budget_idr = COALESCE($8, total_budget_idr),
             budget_hr_idr = COALESCE($9, budget_hr_idr),
             budget_software_idr = COALESCE($10, budget_software_idr),
             budget_hardware_idr = COALESCE($11, budget_hardware_idr),
             budget_overhead_idr = COALESCE($12, budget_overhead_idr)
         WHERE id = $13
         RETURNING id, name, client, description, start_date, end_date, status, project_manager_id, total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr",
        req.name.as_ref(),
        req.client.as_ref(),
        req.description.as_ref(),
        req.start_date,
        req.end_date,
        req.status.as_ref(),
        req.project_manager_id,
        req.total_budget_idr,
        req.budget_hr_idr,
        req.budget_software_idr,
        req.budget_hardware_idr,
        req.budget_overhead_idr,
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        user_id,
        "update",
        "project",
        project.id,
        audit_changes,
    )
    .await?;

    Ok(Json(project))
}

/// Delete a project
async fn delete_project(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check if project exists
    let existing = sqlx::query!(
        "SELECT id, name, client, description, start_date, end_date, status, project_manager_id, total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr FROM projects WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", id)))?;

    // Delete the project
    sqlx::query!("DELETE FROM projects WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let user_id = user_id_from_headers(&headers)?;
    let audit_changes = audit_payload(
        Some(serde_json::json!({
            "name": existing.name,
            "client": existing.client,
            "description": existing.description,
            "start_date": existing.start_date,
            "end_date": existing.end_date,
            "status": existing.status,
            "project_manager_id": existing.project_manager_id,
            "total_budget_idr": existing.total_budget_idr,
            "budget_hr_idr": existing.budget_hr_idr,
            "budget_software_idr": existing.budget_software_idr,
            "budget_hardware_idr": existing.budget_hardware_idr,
            "budget_overhead_idr": existing.budget_overhead_idr,
        })),
        None,
    );
    log_audit(&pool, user_id, "delete", "project", id, audit_changes).await?;

    Ok(Json(
        serde_json::json!({"message": "Project deleted successfully"}),
    ))
}

async fn get_project_budget(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ProjectBudgetResponse>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".into()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID".into()))?;

    if claims.role == "project_manager" {
        let mut conn = pool
            .acquire()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let is_pm =
            crate::services::rbac::is_project_manager(&mut conn, user_id, project_id).await?;
        if !is_pm {
            log_audit(
                &pool,
                Some(user_id),
                "ACCESS_DENIED",
                "project_budget",
                project_id,
                serde_json::json!({"reason": "not_project_manager", "action": "get_project_budget"}),
            )
            .await
            .ok();
            return Err(AppError::Forbidden("Insufficient permissions".into()));
        }
    } else if claims.role != "admin" {
        return Err(AppError::Forbidden("Insufficient permissions".into()));
    }

    let project = sqlx::query!(
        "SELECT id, name, client, total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr FROM projects WHERE id = $1",
        project_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let total = project.total_budget_idr;
    let (hr_pct, sw_pct, hw_pct, oh_pct) = if total > 0 {
        (
            project.budget_hr_idr as f64 / total as f64 * 100.0,
            project.budget_software_idr as f64 / total as f64 * 100.0,
            project.budget_hardware_idr as f64 / total as f64 * 100.0,
            project.budget_overhead_idr as f64 / total as f64 * 100.0,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    let expense_total_idr: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(amount_idr), 0)::BIGINT AS \"total!: i64\" FROM project_expenses WHERE project_id = $1",
        project_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let resource_costs =
        crate::services::project_cost_service::compute_project_resource_costs(&pool, project_id)
            .await?;
    let spent_to_date_idr = expense_total_idr + resource_costs.total_resource_cost_idr;

    Ok(Json(ProjectBudgetResponse {
        project_id: project.id,
        project_name: project.name,
        client: project.client,
        total_budget_idr: total,
        budget_hr_idr: project.budget_hr_idr,
        budget_software_idr: project.budget_software_idr,
        budget_hardware_idr: project.budget_hardware_idr,
        budget_overhead_idr: project.budget_overhead_idr,
        hr_pct,
        software_pct: sw_pct,
        hardware_pct: hw_pct,
        overhead_pct: oh_pct,
        spent_to_date_idr,
        remaining_idr: total - spent_to_date_idr,
    }))
}

async fn set_project_budget(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
    Json(req): Json<SetProjectBudgetRequest>,
) -> Result<Json<ProjectBudgetResponse>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".into()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID".into()))?;

    if claims.role == "project_manager" {
        let mut conn = pool
            .acquire()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let is_pm =
            crate::services::rbac::is_project_manager(&mut conn, user_id, project_id).await?;
        if !is_pm {
            log_audit(
                &pool,
                Some(user_id),
                "ACCESS_DENIED",
                "project_budget",
                project_id,
                serde_json::json!({"reason": "not_project_manager", "action": "set_project_budget"}),
            )
            .await
            .ok();
            return Err(AppError::Forbidden("Insufficient permissions".into()));
        }
    } else if claims.role != "admin" {
        log_audit(
            &pool,
            Some(user_id),
            "ACCESS_DENIED",
            "project_budget",
            project_id,
            serde_json::json!({"reason": "insufficient_role", "attempted_role": claims.role, "action": "set_project_budget"}),
        )
        .await
        .ok();
        return Err(AppError::Forbidden("Insufficient permissions".into()));
    }

    crate::services::project_service::validate_project_budget(
        req.total_budget_idr,
        req.budget_hr_idr,
        req.budget_software_idr,
        req.budget_hardware_idr,
        req.budget_overhead_idr,
    )?;

    let existing = sqlx::query!(
        "SELECT total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr FROM projects WHERE id = $1",
        project_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let audit_changes = audit_payload(
        Some(serde_json::json!({
            "total_budget_idr": existing.total_budget_idr,
            "budget_hr_idr": existing.budget_hr_idr,
            "budget_software_idr": existing.budget_software_idr,
            "budget_hardware_idr": existing.budget_hardware_idr,
            "budget_overhead_idr": existing.budget_overhead_idr,
        })),
        Some(serde_json::json!({
            "total_budget_idr": req.total_budget_idr,
            "budget_hr_idr": req.budget_hr_idr,
            "budget_software_idr": req.budget_software_idr,
            "budget_hardware_idr": req.budget_hardware_idr,
            "budget_overhead_idr": req.budget_overhead_idr,
        })),
    );

    let project = sqlx::query!(
        "UPDATE projects SET total_budget_idr = $1, budget_hr_idr = $2, budget_software_idr = $3, budget_hardware_idr = $4, budget_overhead_idr = $5 WHERE id = $6 RETURNING id, name, client, total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr",
        req.total_budget_idr,
        req.budget_hr_idr,
        req.budget_software_idr,
        req.budget_hardware_idr,
        req.budget_overhead_idr,
        project_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        Some(user_id),
        "update",
        "project_budget",
        project_id,
        audit_changes,
    )
    .await?;

    let total = project.total_budget_idr;
    let (hr_pct, sw_pct, hw_pct, oh_pct) = if total > 0 {
        (
            project.budget_hr_idr as f64 / total as f64 * 100.0,
            project.budget_software_idr as f64 / total as f64 * 100.0,
            project.budget_hardware_idr as f64 / total as f64 * 100.0,
            project.budget_overhead_idr as f64 / total as f64 * 100.0,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    let expense_total_idr: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(amount_idr), 0)::BIGINT AS \"total!: i64\" FROM project_expenses WHERE project_id = $1",
        project_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let resource_costs =
        crate::services::project_cost_service::compute_project_resource_costs(&pool, project_id)
            .await?;
    let spent_to_date_idr = expense_total_idr + resource_costs.total_resource_cost_idr;

    Ok(Json(ProjectBudgetResponse {
        project_id: project.id,
        project_name: project.name,
        client: project.client,
        total_budget_idr: total,
        budget_hr_idr: project.budget_hr_idr,
        budget_software_idr: project.budget_software_idr,
        budget_hardware_idr: project.budget_hardware_idr,
        budget_overhead_idr: project.budget_overhead_idr,
        hr_pct,
        software_pct: sw_pct,
        hardware_pct: hw_pct,
        overhead_pct: oh_pct,
        spent_to_date_idr,
        remaining_idr: total - spent_to_date_idr,
    }))
}

// ── Expense CRUD Handlers ───────────────────────────────────────────────────

/// Helper: enforce PM-owns-project or admin access for expense endpoints.
async fn enforce_expense_access(
    pool: &PgPool,
    headers: &HeaderMap,
    project_id: Uuid,
    action_name: &str,
    denied_entity_type: &str,
) -> Result<Uuid> {
    let claims = user_claims_from_headers(headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".into()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID".into()))?;

    if claims.role == "project_manager" {
        let mut conn = pool
            .acquire()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let is_pm =
            crate::services::rbac::is_project_manager(&mut conn, user_id, project_id).await?;
        if !is_pm {
            log_audit(
                pool,
                Some(user_id),
                "ACCESS_DENIED",
                denied_entity_type,
                project_id,
                serde_json::json!({"reason": "not_project_manager", "action": action_name}),
            )
            .await
            .ok();
            return Err(AppError::Forbidden("Insufficient permissions".into()));
        }
    } else if claims.role != "admin" {
        log_audit(
            pool,
            Some(user_id),
            "ACCESS_DENIED",
            denied_entity_type,
            project_id,
            serde_json::json!({"reason": "insufficient_role", "attempted_role": claims.role, "action": action_name}),
        )
        .await
        .ok();
        return Err(AppError::Forbidden("Insufficient permissions".into()));
    }

    Ok(user_id)
}

/// Create a project expense.
async fn create_project_expense(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateProjectExpenseRequest>,
) -> Result<Json<ProjectExpenseResponse>> {
    let user_id = enforce_expense_access(
        &pool,
        &headers,
        project_id,
        "create_expense",
        "project_expense",
    )
    .await?;

    crate::services::project_service::validate_create_expense(
        &req.category,
        req.amount_idr,
        &req.description,
    )?;

    // Verify project exists
    sqlx::query_scalar!("SELECT id FROM projects WHERE id = $1", project_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let expense = sqlx::query_as!(
        ProjectExpenseResponse,
        r#"INSERT INTO project_expenses (project_id, category, description, amount_idr, expense_date, vendor, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, project_id, category, description, amount_idr, expense_date, vendor, created_by, created_at, updated_at"#,
        project_id,
        req.category,
        req.description,
        req.amount_idr,
        req.expense_date,
        req.vendor,
        user_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        Some(user_id),
        "create",
        "project_expense",
        expense.id,
        audit_payload(
            None,
            Some(serde_json::json!({
                "project_id": project_id,
                "category": req.category,
                "description": req.description,
                "amount_idr": req.amount_idr,
                "expense_date": req.expense_date.to_string(),
                "vendor": req.vendor,
            })),
        ),
    )
    .await?;

    Ok(Json(expense))
}

/// List project expenses (newest first).
async fn list_project_expenses(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<ProjectExpenseResponse>>> {
    let _user_id = enforce_expense_access(
        &pool,
        &headers,
        project_id,
        "list_expenses",
        "project_expense",
    )
    .await?;

    sqlx::query_scalar!("SELECT id FROM projects WHERE id = $1", project_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let expenses = sqlx::query_as!(
        ProjectExpenseResponse,
        r#"SELECT id, project_id, category, description, amount_idr, expense_date, vendor, created_by, created_at, updated_at
         FROM project_expenses
         WHERE project_id = $1
         ORDER BY expense_date DESC, created_at DESC"#,
        project_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(expenses))
}

/// Update a project expense (requires edit_reason).
async fn update_project_expense(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path((project_id, expense_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateProjectExpenseRequest>,
) -> Result<Json<ProjectExpenseResponse>> {
    let user_id = enforce_expense_access(
        &pool,
        &headers,
        project_id,
        "update_expense",
        "project_expense",
    )
    .await?;

    crate::services::project_service::validate_update_expense(
        req.category.as_deref(),
        req.description.as_deref(),
        req.amount_idr,
        &req.edit_reason,
    )?;

    // Fetch existing expense and verify it belongs to this project
    let existing = sqlx::query!(
        "SELECT id, project_id, category, description, amount_idr, expense_date, vendor FROM project_expenses WHERE id = $1 AND project_id = $2",
        expense_id,
        project_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Expense {} not found in project {}", expense_id, project_id)))?;

    let updated = sqlx::query_as!(
        ProjectExpenseResponse,
        r#"UPDATE project_expenses
         SET category = COALESCE($1, category),
             description = COALESCE($2, description),
             amount_idr = COALESCE($3, amount_idr),
             expense_date = COALESCE($4, expense_date),
             vendor = CASE
                 WHEN $5::text IS NULL THEN vendor
                 WHEN $5::text = '' THEN NULL
                 ELSE $5::text
             END,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = $6 AND project_id = $7
         RETURNING id, project_id, category, description, amount_idr, expense_date, vendor, created_by, created_at, updated_at"#,
        req.category,
        req.description,
        req.amount_idr,
        req.expense_date,
        req.vendor,
        expense_id,
        project_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        Some(user_id),
        "update",
        "project_expense",
        expense_id,
        audit_payload(
            Some(serde_json::json!({
                "category": existing.category,
                "description": existing.description,
                "amount_idr": existing.amount_idr,
                "expense_date": existing.expense_date.to_string(),
                "vendor": existing.vendor,
            })),
            Some(serde_json::json!({
                "category": updated.category,
                "description": updated.description,
                "amount_idr": updated.amount_idr,
                "expense_date": updated.expense_date.to_string(),
                "vendor": updated.vendor,
                "edit_reason": req.edit_reason,
            })),
        ),
    )
    .await?;

    Ok(Json(updated))
}

/// Delete a project expense (hard delete).
async fn delete_project_expense(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path((project_id, expense_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    let user_id = enforce_expense_access(
        &pool,
        &headers,
        project_id,
        "delete_expense",
        "project_expense",
    )
    .await?;

    // Fetch existing for audit snapshot, verify it belongs to this project
    let existing = sqlx::query!(
        "SELECT id, project_id, category, description, amount_idr, expense_date, vendor FROM project_expenses WHERE id = $1 AND project_id = $2",
        expense_id,
        project_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Expense {} not found in project {}", expense_id, project_id)))?;

    sqlx::query!(
        "DELETE FROM project_expenses WHERE id = $1 AND project_id = $2",
        expense_id,
        project_id
    )
    .execute(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        Some(user_id),
        "expense_deleted",
        "project_expense",
        expense_id,
        audit_payload(
            Some(serde_json::json!({
                "project_id": project_id,
                "category": existing.category,
                "description": existing.description,
                "amount_idr": existing.amount_idr,
                "expense_date": existing.expense_date.to_string(),
                "vendor": existing.vendor,
            })),
            None,
        ),
    )
    .await?;

    Ok(Json(
        serde_json::json!({"message": "Expense deleted successfully"}),
    ))
}

fn parse_revenue_month_for_lookup(input: &str) -> Result<NaiveDate> {
    let trimmed = input.trim();
    if trimmed.len() != 7 || trimmed.chars().nth(4) != Some('-') {
        return Err(AppError::Validation(
            "revenue_month must use YYYY-MM format".into(),
        ));
    }

    NaiveDate::parse_from_str(&format!("{}-01", trimmed), "%Y-%m-%d")
        .map_err(|_| AppError::Validation("revenue_month must use YYYY-MM format".into()))
}

fn extract_idempotency_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn map_revenue_row_response(
    row: crate::services::project_revenue_service::ProjectRevenueRow,
) -> ProjectRevenueRowResponse {
    ProjectRevenueRowResponse {
        id: row.id,
        project_id: row.project_id,
        revenue_month: row.revenue_month,
        amount_idr: row.amount_idr,
        source_type: row.source_type,
        source_reference: row.source_reference,
        entered_by: row.entered_by,
        entry_date: row.entry_date,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

#[derive(Debug, Clone)]
struct RevenueBeforeAudit {
    id: Uuid,
    amount_idr: i64,
    source_type: String,
    source_reference: Option<String>,
}

async fn fetch_revenue_before_audit(
    pool: &PgPool,
    project_id: Uuid,
    revenue_month: NaiveDate,
) -> Result<Option<RevenueBeforeAudit>> {
    let row_opt = sqlx::query(
        r#"SELECT id, amount_idr, source_type, source_reference
           FROM project_revenues
           WHERE project_id = $1 AND revenue_month = $2"#,
    )
    .bind(project_id)
    .bind(revenue_month)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some(row) = row_opt {
        Ok(Some(RevenueBeforeAudit {
            id: row
                .try_get("id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            amount_idr: row
                .try_get("amount_idr")
                .map_err(|e| AppError::Database(e.to_string()))?,
            source_type: row
                .try_get("source_type")
                .map_err(|e| AppError::Database(e.to_string()))?,
            source_reference: row
                .try_get("source_reference")
                .map_err(|e| AppError::Database(e.to_string()))?,
        }))
    } else {
        Ok(None)
    }
}

async fn upsert_project_revenue(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
    Json(req): Json<UpsertProjectRevenueRequest>,
) -> Result<Json<ProjectRevenueRowResponse>> {
    let user_id = enforce_expense_access(
        &pool,
        &headers,
        project_id,
        "upsert_revenue",
        "project_revenue",
    )
    .await?;

    sqlx::query_scalar!("SELECT id FROM projects WHERE id = $1", project_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let lookup_month = parse_revenue_month_for_lookup(&req.revenue_month)?;
    let before = fetch_revenue_before_audit(&pool, project_id, lookup_month).await?;

    let row = crate::services::project_revenue_service::upsert_project_revenue(
        &pool, project_id, user_id, &req,
    )
    .await?;

    let action = if before.is_some() { "update" } else { "create" };
    let audit_changes = if let Some(before_row) = before {
        audit_payload(
            Some(serde_json::json!({
                "amount_idr": before_row.amount_idr,
                "source_type": before_row.source_type,
                "source_reference": before_row.source_reference,
            })),
            Some(serde_json::json!({
                "amount_idr": row.amount_idr,
                "source_type": row.source_type,
                "source_reference": row.source_reference,
                "override_erp": req.override_erp,
            })),
        )
    } else {
        audit_payload(
            None,
            Some(serde_json::json!({
                "project_id": project_id,
                "revenue_month": row.revenue_month.to_string(),
                "amount_idr": row.amount_idr,
                "source_type": row.source_type,
                "source_reference": row.source_reference,
            })),
        )
    };

    log_audit(
        &pool,
        Some(user_id),
        action,
        "project_revenue",
        row.id,
        audit_changes,
    )
    .await?;

    Ok(Json(map_revenue_row_response(row)))
}

async fn get_project_revenue(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
    Query(query): Query<ProjectRevenueQuery>,
) -> Result<Json<ProjectRevenueGridResponse>> {
    let _user_id = enforce_expense_access(
        &pool,
        &headers,
        project_id,
        "get_revenue",
        "project_revenue",
    )
    .await?;

    sqlx::query_scalar!("SELECT id FROM projects WHERE id = $1", project_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let grid =
        crate::services::project_revenue_service::get_revenue_grid(&pool, project_id, query.year)
            .await?;

    let months = grid
        .months
        .into_iter()
        .map(|m| MonthRevenueEntry {
            month: m.month,
            month_label: m.month_label,
            revenue_id: m.revenue_id,
            amount_idr: m.amount_idr,
            source_type: m.source_type,
            source_reference: m.source_reference,
            entered_by: m.entered_by,
            entry_date: m.entry_date,
        })
        .collect();

    Ok(Json(ProjectRevenueGridResponse {
        project_id: grid.project_id,
        year: grid.year,
        months,
        ytd_total_idr: grid.ytd_total_idr,
    }))
}

async fn ingest_erp_revenue(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
    Json(req): Json<IngestErpRevenueRequest>,
) -> Result<Json<ProjectRevenueRowResponse>> {
    let idempotency_key = extract_idempotency_key(&headers);

    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".into()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID".into()))?;

    if !matches!(claims.role.as_str(), "admin" | "finance") {
        log_audit(
            &pool,
            Some(user_id),
            "ACCESS_DENIED",
            "project_revenue_erp",
            project_id,
            serde_json::json!({
                "reason": "insufficient_role",
                "attempted_role": claims.role,
                "action": "ingest_erp_revenue"
            }),
        )
        .await
        .ok();
        return Err(AppError::Forbidden("Insufficient permissions".into()));
    }

    sqlx::query_scalar!("SELECT id FROM projects WHERE id = $1", project_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let lookup_month = parse_revenue_month_for_lookup(&req.revenue_month)?;
    let before = fetch_revenue_before_audit(&pool, project_id, lookup_month).await?;

    let row = crate::services::project_revenue_service::ingest_erp_revenue(
        &pool,
        project_id,
        user_id,
        &req,
        idempotency_key.as_deref(),
    )
    .await?;

    let preserved_manual = before
        .as_ref()
        .map(|existing| {
            (existing.source_type == "manual" || existing.source_type == "manual_override")
                && existing.id == row.id
        })
        .unwrap_or(false);

    if !preserved_manual {
        let action = if before.is_some() { "update" } else { "create" };
        let audit_changes = if let Some(before_row) = before {
            audit_payload(
                Some(serde_json::json!({
                    "amount_idr": before_row.amount_idr,
                    "source_type": before_row.source_type,
                    "source_reference": before_row.source_reference,
                })),
                Some(serde_json::json!({
                    "amount_idr": row.amount_idr,
                    "source_type": row.source_type,
                    "source_reference": row.source_reference,
                })),
            )
        } else {
            audit_payload(
                None,
                Some(serde_json::json!({
                    "project_id": project_id,
                    "revenue_month": row.revenue_month.to_string(),
                    "amount_idr": row.amount_idr,
                    "source_type": row.source_type,
                    "source_reference": row.source_reference,
                })),
            )
        };

        log_audit(
            &pool,
            Some(user_id),
            action,
            "project_revenue",
            row.id,
            audit_changes,
        )
        .await?;
    }

    Ok(Json(map_revenue_row_response(row)))
}

// ── Resource Cost Endpoint ────────────────────────────────────────────────

/// Get computed resource costs for a project.
async fn get_project_resource_costs(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ProjectResourceCostResponse>> {
    let _user_id = enforce_expense_access(
        &pool,
        &headers,
        project_id,
        "get_resource_costs",
        "project_resource_costs",
    )
    .await?;

    let result =
        crate::services::project_cost_service::compute_project_resource_costs(&pool, project_id)
            .await?;

    let employees = result
        .employees
        .into_iter()
        .map(|e| EmployeeResourceCost {
            resource_id: e.resource_id,
            resource_name: e.resource_name,
            daily_rate_idr: e.daily_rate_idr,
            days_allocated: e.days_allocated,
            allocation_percentage: e.allocation_percentage,
            total_cost_idr: e.total_cost_idr,
            has_rate_change: e.has_rate_change,
            rate_change_note: e.rate_change_note,
            missing_rate: e.missing_rate,
        })
        .collect();

    let monthly_breakdown = result
        .monthly_breakdown
        .into_iter()
        .map(|m| MonthlyResourceCost {
            month: m.month,
            working_days: m.working_days,
            cost_idr: m.cost_idr,
        })
        .collect();

    Ok(Json(ProjectResourceCostResponse {
        project_id: result.project_id,
        total_resource_cost_idr: result.total_resource_cost_idr,
        employees,
        monthly_breakdown,
    }))
}

/// Create project routes
pub fn project_routes() -> Router<PgPool> {
    Router::new()
        .route("/projects", get(get_projects).post(create_project))
        .route("/projects/assignable", get(get_assignable_projects))
        .route(
            "/projects/:id/budget",
            get(get_project_budget).post(set_project_budget),
        )
        .route(
            "/projects/:id/resource-costs",
            get(get_project_resource_costs),
        )
        .route(
            "/projects/:id/expenses",
            get(list_project_expenses).post(create_project_expense),
        )
        .route(
            "/projects/:id/expenses/:expense_id",
            put(update_project_expense).delete(delete_project_expense),
        )
        .route(
            "/projects/:id/revenue",
            get(get_project_revenue).post(upsert_project_revenue),
        )
        .route(
            "/projects/:id/revenue/erp-sync",
            axum::routing::post(ingest_erp_revenue),
        )
        .route(
            "/projects/:id",
            get(get_project).put(update_project).delete(delete_project),
        )
}
