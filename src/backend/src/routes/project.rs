use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, put},
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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
    let assigned_project_manager_id = if claims
        .as_ref()
        .is_some_and(|c| c.role == "project_manager")
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
    let merged_budget_software_idr = req
        .budget_software_idr
        .unwrap_or(after_budget_software_idr);
    let merged_budget_hardware_idr = req
        .budget_hardware_idr
        .unwrap_or(after_budget_hardware_idr);
    let merged_budget_overhead_idr = req
        .budget_overhead_idr
        .unwrap_or(after_budget_overhead_idr);

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
        let is_pm = crate::services::rbac::is_project_manager(&mut conn, user_id, project_id).await?;
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

    let spent_to_date_idr: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(amount_idr), 0)::BIGINT AS \"total!: i64\" FROM project_expenses WHERE project_id = $1",
        project_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

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
        let is_pm = crate::services::rbac::is_project_manager(&mut conn, user_id, project_id).await?;
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

    let spent_to_date_idr: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(amount_idr), 0)::BIGINT AS \"total!: i64\" FROM project_expenses WHERE project_id = $1",
        project_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

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
        let is_pm = crate::services::rbac::is_project_manager(&mut conn, user_id, project_id).await?;
        if !is_pm {
            log_audit(
                pool,
                Some(user_id),
                "ACCESS_DENIED",
                "project_expense",
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
            "project_expense",
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
    let user_id = enforce_expense_access(&pool, &headers, project_id, "create_expense").await?;

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
    let _user_id = enforce_expense_access(&pool, &headers, project_id, "list_expenses").await?;

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
    let user_id = enforce_expense_access(&pool, &headers, project_id, "update_expense").await?;

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
    let user_id = enforce_expense_access(&pool, &headers, project_id, "delete_expense").await?;

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

    sqlx::query!("DELETE FROM project_expenses WHERE id = $1 AND project_id = $2", expense_id, project_id)
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

    Ok(Json(serde_json::json!({"message": "Expense deleted successfully"})))
}


/// Create project routes
pub fn project_routes() -> Router<PgPool> {
    Router::new()
        .route("/projects", get(get_projects).post(create_project))
        .route("/projects/assignable", get(get_assignable_projects))
        .route("/projects/:id/budget", get(get_project_budget).post(set_project_budget))
        .route(
            "/projects/:id/expenses",
            get(list_project_expenses).post(create_project_expense),
        )
        .route(
            "/projects/:id/expenses/:expense_id",
            put(update_project_expense).delete(delete_project_expense),
        )
        .route(
            "/projects/:id",
            get(get_project).put(update_project).delete(delete_project),
        )
}
