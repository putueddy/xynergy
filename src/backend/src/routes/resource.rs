use axum::{
    extract::{State, Path},
    routing::{get, post, put, delete},
    Router,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{AppError, Result};

/// Resource response structure using f64 for capacity
#[derive(Debug, Serialize)]
pub struct ResourceResponse {
    pub id: Uuid,
    pub name: String,
    pub resource_type: String,
    pub capacity: Option<f64>,
    pub department_id: Option<Uuid>,
    pub skills: Option<serde_json::Value>,
}

/// Create resource request
#[derive(Debug, Deserialize)]
pub struct CreateResourceRequest {
    pub name: String,
    pub resource_type: String,
    pub capacity: Option<f64>,
    pub department_id: Option<Uuid>,
    pub skills: Option<serde_json::Value>,
}

/// Update resource request
#[derive(Debug, Deserialize)]
pub struct UpdateResourceRequest {
    pub name: Option<String>,
    pub resource_type: Option<String>,
    pub capacity: Option<f64>,
    pub department_id: Option<Uuid>,
    pub skills: Option<serde_json::Value>,
}

/// Convert BigDecimal to f64
fn bigdecimal_to_f64(bd: Option<sqlx::types::BigDecimal>) -> Option<f64> {
    bd.and_then(|d| d.to_string().parse::<f64>().ok())
}

/// Convert f64 to BigDecimal
fn f64_to_bigdecimal(f: Option<f64>) -> Option<sqlx::types::BigDecimal> {
    f.and_then(|v| sqlx::types::BigDecimal::try_from(v).ok())
}

/// Get all resources
async fn get_resources(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<ResourceResponse>>> {
    let resources = sqlx::query!(
        "SELECT id, name, resource_type, capacity, department_id, skills 
         FROM resources ORDER BY name"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    let response: Vec<ResourceResponse> = resources
        .into_iter()
        .map(|r| ResourceResponse {
            id: r.id,
            name: r.name,
            resource_type: r.resource_type,
            capacity: bigdecimal_to_f64(r.capacity),
            department_id: r.department_id,
            skills: r.skills,
        })
        .collect();
    
    Ok(Json(response))
}

/// Get resource by ID
async fn get_resource(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<ResourceResponse>> {
    let resource = sqlx::query!(
        "SELECT id, name, resource_type, capacity, department_id, skills 
         FROM resources WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Resource {} not found", id)))?;
    
    Ok(Json(ResourceResponse {
        id: resource.id,
        name: resource.name,
        resource_type: resource.resource_type,
        capacity: bigdecimal_to_f64(resource.capacity),
        department_id: resource.department_id,
        skills: resource.skills,
    }))
}

/// Create a new resource
async fn create_resource(
    State(pool): State<PgPool>,
    Json(req): Json<CreateResourceRequest>,
) -> Result<Json<ResourceResponse>> {
    let capacity_decimal = f64_to_bigdecimal(req.capacity);
    
    let resource = sqlx::query!(
        "INSERT INTO resources (name, resource_type, capacity, department_id, skills)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, name, resource_type, capacity, department_id, skills",
        req.name,
        req.resource_type,
        capacity_decimal,
        req.department_id,
        req.skills
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    Ok(Json(ResourceResponse {
        id: resource.id,
        name: resource.name,
        resource_type: resource.resource_type,
        capacity: bigdecimal_to_f64(resource.capacity),
        department_id: resource.department_id,
        skills: resource.skills,
    }))
}

/// Update a resource
async fn update_resource(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateResourceRequest>,
) -> Result<Json<ResourceResponse>> {
    // Check if resource exists
    let _ = sqlx::query!(
        "SELECT id FROM resources WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Resource {} not found", id)))?;
    
    // Convert capacity
    let capacity_decimal = f64_to_bigdecimal(req.capacity);
    
    // Update with new values or keep existing
    let resource = sqlx::query!(
        "UPDATE resources 
         SET name = COALESCE($1, name),
             resource_type = COALESCE($2, resource_type),
             capacity = COALESCE($3, capacity),
             department_id = COALESCE($4, department_id),
             skills = COALESCE($5, skills)
         WHERE id = $6
         RETURNING id, name, resource_type, capacity, department_id, skills",
        req.name.as_ref(),
        req.resource_type.as_ref(),
        capacity_decimal,
        req.department_id,
        req.skills,
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    Ok(Json(ResourceResponse {
        id: resource.id,
        name: resource.name,
        resource_type: resource.resource_type,
        capacity: bigdecimal_to_f64(resource.capacity),
        department_id: resource.department_id,
        skills: resource.skills,
    }))
}

/// Delete a resource
async fn delete_resource(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check if resource exists
    let _ = sqlx::query!(
        "SELECT id FROM resources WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Resource {} not found", id)))?;
    
    // Delete the resource
    sqlx::query!("DELETE FROM resources WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    
    Ok(Json(json!({"message": "Resource deleted successfully"})))
}

/// Create resource routes
pub fn resource_routes() -> Router<PgPool> {
    Router::new()
        .route("/resources", get(get_resources).post(create_resource))
        .route("/resources/:id", get(get_resource).put(update_resource).delete(delete_resource))
}
