use axum::{
    extract::{State, Path, Query},
    routing::get,
    Router,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;
use serde::Deserialize;
use serde_json::json;

use crate::error::{AppError, Result};

/// Query parameters for project listing
#[derive(Debug, Deserialize)]
pub struct ProjectQuery {
    pub status: Option<String>,
}

/// Get all projects
async fn get_projects(
    State(pool): State<PgPool>,
) -> Result<Json<serde_json::Value>> {
    let projects = sqlx::query!(
        "SELECT id, name, description, start_date, end_date, status, project_manager_id 
         FROM projects ORDER BY start_date DESC"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    let projects_json: Vec<serde_json::Value> = projects
        .into_iter()
        .map(|p| {
            json!({
                "id": p.id,
                "name": p.name,
                "description": p.description,
                "start_date": p.start_date,
                "end_date": p.end_date,
                "status": p.status,
                "project_manager_id": p.project_manager_id
            })
        })
        .collect();
    
    Ok(Json(json!(projects_json)))
}

/// Get project by ID
async fn get_project(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let project = sqlx::query!(
        "SELECT id, name, description, start_date, end_date, status, project_manager_id 
         FROM projects WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", id)))?;
    
    Ok(Json(json!({
        "id": project.id,
        "name": project.name,
        "description": project.description,
        "start_date": project.start_date,
        "end_date": project.end_date,
        "status": project.status,
        "project_manager_id": project.project_manager_id
    })))
}

/// Create project routes
pub fn project_routes() -> Router<PgPool> {
    Router::new()
        .route("/projects", get(get_projects))
        .route("/projects/:id", get(get_project))
}
