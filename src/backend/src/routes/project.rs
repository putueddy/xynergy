use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
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
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
    pub project_manager_id: Option<Uuid>,
}

/// Create project request
#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
    pub project_manager_id: Option<Uuid>,
}

/// Update project request
#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub status: Option<String>,
    pub project_manager_id: Option<Uuid>,
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
            "SELECT id, name, description, start_date, end_date, status, project_manager_id 
             FROM projects WHERE project_manager_id = $1 ORDER BY start_date DESC",
            user_id
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
    } else {
        sqlx::query_as!(
            ProjectResponse,
            "SELECT id, name, description, start_date, end_date, status, project_manager_id 
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
        "SELECT id, name, description, start_date, end_date, status, project_manager_id 
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
    let audit_changes = audit_payload(
        None,
        Some(serde_json::json!({
            "name": req.name.clone(),
            "description": req.description.clone(),
            "start_date": req.start_date,
            "end_date": req.end_date,
            "status": req.status.clone(),
            "project_manager_id": req.project_manager_id,
        })),
    );
    let user_id = user_id_from_headers(&headers)?;

    let project = sqlx::query_as!(
        ProjectResponse,
        "INSERT INTO projects (name, description, start_date, end_date, status, project_manager_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, name, description, start_date, end_date, status, project_manager_id",
        req.name,
        req.description,
        req.start_date,
        req.end_date,
        req.status,
        req.project_manager_id
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
        "SELECT id, name, description, start_date, end_date, status, project_manager_id FROM projects WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", id)))?;

    let before_name = existing.name.clone();
    let before_description = existing.description.clone();
    let before_status = existing.status.clone();
    let before_start_date = existing.start_date;
    let before_end_date = existing.end_date;
    let before_manager_id = existing.project_manager_id;
    let after_name_default = existing.name;
    let after_description_default = existing.description;
    let after_status_default = existing.status;
    let after_start_default = existing.start_date;
    let after_end_default = existing.end_date;
    let after_manager_default = existing.project_manager_id;
    let audit_changes = audit_payload(
        Some(serde_json::json!({
            "name": before_name,
            "description": before_description,
            "start_date": before_start_date,
            "end_date": before_end_date,
            "status": before_status,
            "project_manager_id": before_manager_id,
        })),
        Some(serde_json::json!({
            "name": req.name.clone().unwrap_or_else(|| after_name_default),
            "description": req.description.clone().or(after_description_default),
            "start_date": req.start_date.unwrap_or(after_start_default),
            "end_date": req.end_date.unwrap_or(after_end_default),
            "status": req.status.clone().unwrap_or_else(|| after_status_default),
            "project_manager_id": req.project_manager_id.or(after_manager_default),
        })),
    );
    let user_id = user_id_from_headers(&headers)?;

    // Update with new values or keep existing
    let project = sqlx::query_as!(
        ProjectResponse,
        "UPDATE projects 
         SET name = COALESCE($1, name),
             description = COALESCE($2, description),
             start_date = COALESCE($3, start_date),
             end_date = COALESCE($4, end_date),
             status = COALESCE($5, status),
             project_manager_id = COALESCE($6, project_manager_id)
         WHERE id = $7
         RETURNING id, name, description, start_date, end_date, status, project_manager_id",
        req.name.as_ref(),
        req.description.as_ref(),
        req.start_date,
        req.end_date,
        req.status.as_ref(),
        req.project_manager_id,
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
        "SELECT id, name, description, start_date, end_date, status, project_manager_id FROM projects WHERE id = $1",
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
            "description": existing.description,
            "start_date": existing.start_date,
            "end_date": existing.end_date,
            "status": existing.status,
            "project_manager_id": existing.project_manager_id,
        })),
        None,
    );
    log_audit(&pool, user_id, "delete", "project", id, audit_changes).await?;

    Ok(Json(
        serde_json::json!({"message": "Project deleted successfully"}),
    ))
}

/// Create project routes
pub fn project_routes() -> Router<PgPool> {
    Router::new()
        .route("/projects", get(get_projects).post(create_project))
        .route("/projects/assignable", get(get_assignable_projects))
        .route(
            "/projects/:id",
            get(get_project).put(update_project).delete(delete_project),
        )
}
