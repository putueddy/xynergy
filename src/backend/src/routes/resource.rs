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

/// Get all resources
async fn get_resources(
    State(pool): State<PgPool>,
) -> Result<Json<serde_json::Value>> {
    let resources = sqlx::query!(
        "SELECT id, name, resource_type, capacity, department_id, skills 
         FROM resources ORDER BY name"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    let resources_json: Vec<serde_json::Value> = resources
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "name": r.name,
                "resource_type": r.resource_type,
                "capacity": r.capacity.map(|c| c.to_string()),
                "department_id": r.department_id,
                "skills": r.skills
            })
        })
        .collect();
    
    Ok(Json(json!(resources_json)))
}

/// Get resource by ID
async fn get_resource(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let resource = sqlx::query!(
        "SELECT id, name, resource_type, capacity, department_id, skills 
         FROM resources WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Resource {} not found", id)))?;
    
    Ok(Json(json!({
        "id": resource.id,
        "name": resource.name,
        "resource_type": resource.resource_type,
        "capacity": resource.capacity.map(|c| c.to_string()),
        "department_id": resource.department_id,
        "skills": resource.skills
    })))
}

/// Create resource routes
pub fn resource_routes() -> Router<PgPool> {
    Router::new()
        .route("/resources", get(get_resources))
        .route("/resources/:id", get(get_resource))
}
