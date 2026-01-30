use axum::{
    extract::{State, Path},
    routing::{get, post, put, delete},
    Router,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;

use crate::error::{AppError, Result};

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

/// Get all projects
async fn get_projects(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<ProjectResponse>>> {
    let projects = sqlx::query_as!(
        ProjectResponse,
        "SELECT id, name, description, start_date, end_date, status, project_manager_id 
         FROM projects ORDER BY start_date DESC"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    Ok(Json(projects))
}

/// Get project by ID
async fn get_project(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProjectResponse>> {
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
    
    Ok(Json(project))
}

/// Create a new project
async fn create_project(
    State(pool): State<PgPool>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<Json<ProjectResponse>> {
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
    
    Ok(Json(project))
}

/// Update a project
async fn update_project(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectResponse>> {
    // Check if project exists
    let _ = sqlx::query!(
        "SELECT id FROM projects WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", id)))?;
    
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
    
    Ok(Json(project))
}

/// Delete a project
async fn delete_project(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check if project exists
    let _ = sqlx::query!(
        "SELECT id FROM projects WHERE id = $1",
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
    
    Ok(Json(serde_json::json!({"message": "Project deleted successfully"})))
}

/// Create project routes
pub fn project_routes() -> Router<PgPool> {
    Router::new()
        .route("/projects", get(get_projects).post(create_project))
        .route("/projects/:id", get(get_project).put(update_project).delete(delete_project))
}
