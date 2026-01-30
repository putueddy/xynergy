use axum::{
    extract::{State, Path},
    routing::get,
    Router,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;
use serde_json::json;

use crate::error::{AppError, Result};

/// Get all departments
async fn get_departments(
    State(pool): State<PgPool>,
) -> Result<Json<serde_json::Value>> {
    let departments = sqlx::query!(
        "SELECT id, name FROM departments ORDER BY name"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    let departments_json: Vec<serde_json::Value> = departments
        .into_iter()
        .map(|d| {
            json!({
                "id": d.id,
                "name": d.name
            })
        })
        .collect();
    
    Ok(Json(json!(departments_json)))
}

/// Get department by ID
async fn get_department(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let department = sqlx::query!(
        "SELECT id, name FROM departments WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Department {} not found", id)))?;
    
    Ok(Json(json!({
        "id": department.id,
        "name": department.name
    })))
}

/// Create department routes
pub fn department_routes() -> Router<PgPool> {
    Router::new()
        .route("/departments", get(get_departments))
        .route("/departments/:id", get(get_department))
}
