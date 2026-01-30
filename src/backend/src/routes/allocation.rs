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

/// Allocation response structure
#[derive(Debug, Serialize)]
pub struct AllocationResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub resource_id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub allocation_percentage: f64,
    pub project_name: String,
    pub resource_name: String,
}

/// Create allocation request
#[derive(Debug, Deserialize)]
pub struct CreateAllocationRequest {
    pub project_id: Uuid,
    pub resource_id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub allocation_percentage: f64,
}

/// Update allocation request
#[derive(Debug, Deserialize)]
pub struct UpdateAllocationRequest {
    pub project_id: Option<Uuid>,
    pub resource_id: Option<Uuid>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub allocation_percentage: Option<f64>,
}

/// Convert BigDecimal to f64
fn bigdecimal_to_f64(bd: sqlx::types::BigDecimal) -> f64 {
    bd.to_string().parse().unwrap_or(0.0)
}

/// Convert f64 to BigDecimal
fn f64_to_bigdecimal(f: f64) -> sqlx::types::BigDecimal {
    sqlx::types::BigDecimal::try_from(f).unwrap_or_default()
}

/// Get all allocations with project and resource names
async fn get_allocations(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<AllocationResponse>>> {
    let allocations = sqlx::query!(
        "SELECT a.id, a.project_id, a.resource_id, a.start_date, a.end_date, a.allocation_percentage,
                p.name as project_name, r.name as resource_name
         FROM allocations a
         JOIN projects p ON a.project_id = p.id
         JOIN resources r ON a.resource_id = r.id
         ORDER BY a.start_date DESC"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    let response: Vec<AllocationResponse> = allocations
        .into_iter()
        .map(|a| AllocationResponse {
            id: a.id,
            project_id: a.project_id.expect("project_id is not null"),
            resource_id: a.resource_id.expect("resource_id is not null"),
            start_date: a.start_date,
            end_date: a.end_date,
            allocation_percentage: bigdecimal_to_f64(a.allocation_percentage),
            project_name: a.project_name,
            resource_name: a.resource_name,
        })
        .collect();
    
    Ok(Json(response))
}

/// Get allocations by project ID
async fn get_allocations_by_project(
    State(pool): State<PgPool>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<AllocationResponse>>> {
    let allocations = sqlx::query!(
        "SELECT a.id, a.project_id, a.resource_id, a.start_date, a.end_date, a.allocation_percentage,
                p.name as project_name, r.name as resource_name
         FROM allocations a
         JOIN projects p ON a.project_id = p.id
         JOIN resources r ON a.resource_id = r.id
         WHERE a.project_id = $1
         ORDER BY a.start_date DESC",
        project_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    let response: Vec<AllocationResponse> = allocations
        .into_iter()
        .map(|a| AllocationResponse {
            id: a.id,
            project_id: a.project_id.expect("project_id is not null"),
            resource_id: a.resource_id.expect("resource_id is not null"),
            start_date: a.start_date,
            end_date: a.end_date,
            allocation_percentage: bigdecimal_to_f64(a.allocation_percentage),
            project_name: a.project_name,
            resource_name: a.resource_name,
        })
        .collect();
    
    Ok(Json(response))
}

/// Get allocations by resource ID
async fn get_allocations_by_resource(
    State(pool): State<PgPool>,
    Path(resource_id): Path<Uuid>,
) -> Result<Json<Vec<AllocationResponse>>> {
    let allocations = sqlx::query!(
        "SELECT a.id, a.project_id, a.resource_id, a.start_date, a.end_date, a.allocation_percentage,
                p.name as project_name, r.name as resource_name
         FROM allocations a
         JOIN projects p ON a.project_id = p.id
         JOIN resources r ON a.resource_id = r.id
         WHERE a.resource_id = $1
         ORDER BY a.start_date DESC",
        resource_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    let response: Vec<AllocationResponse> = allocations
        .into_iter()
        .map(|a| AllocationResponse {
            id: a.id,
            project_id: a.project_id.expect("project_id is not null"),
            resource_id: a.resource_id.expect("resource_id is not null"),
            start_date: a.start_date,
            end_date: a.end_date,
            allocation_percentage: bigdecimal_to_f64(a.allocation_percentage),
            project_name: a.project_name,
            resource_name: a.resource_name,
        })
        .collect();
    
    Ok(Json(response))
}

/// Create a new allocation
async fn create_allocation(
    State(pool): State<PgPool>,
    Json(req): Json<CreateAllocationRequest>,
) -> Result<Json<AllocationResponse>> {
    let allocation_percentage_bd = f64_to_bigdecimal(req.allocation_percentage);
    
    let allocation = sqlx::query!(
        "INSERT INTO allocations (project_id, resource_id, start_date, end_date, allocation_percentage)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, project_id, resource_id, start_date, end_date, allocation_percentage",
        req.project_id,
        req.resource_id,
        req.start_date,
        req.end_date,
        allocation_percentage_bd
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    // Get project and resource names
    let project_name = sqlx::query_scalar!(
        "SELECT name FROM projects WHERE id = $1",
        req.project_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    let resource_name = sqlx::query_scalar!(
        "SELECT name FROM resources WHERE id = $1",
        req.resource_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    Ok(Json(AllocationResponse {
        id: allocation.id,
        project_id: allocation.project_id.expect("project_id is not null"),
        resource_id: allocation.resource_id.expect("resource_id is not null"),
        start_date: allocation.start_date,
        end_date: allocation.end_date,
        allocation_percentage: bigdecimal_to_f64(allocation.allocation_percentage),
        project_name,
        resource_name,
    }))
}

/// Update an allocation
async fn update_allocation(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAllocationRequest>,
) -> Result<Json<AllocationResponse>> {
    // Check if allocation exists
    let existing = sqlx::query!(
        "SELECT id, project_id, resource_id FROM allocations WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Allocation {} not found", id)))?;
    
    // Convert percentage if provided
    let allocation_percentage_bd = req.allocation_percentage.map(f64_to_bigdecimal);
    
    // Update with new values or keep existing
    let allocation = sqlx::query!(
        "UPDATE allocations 
         SET project_id = COALESCE($1, project_id),
             resource_id = COALESCE($2, resource_id),
             start_date = COALESCE($3, start_date),
             end_date = COALESCE($4, end_date),
             allocation_percentage = COALESCE($5, allocation_percentage)
         WHERE id = $6
         RETURNING id, project_id, resource_id, start_date, end_date, allocation_percentage",
        req.project_id,
        req.resource_id,
        req.start_date,
        req.end_date,
        allocation_percentage_bd,
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    // Get project and resource names
    let project_id = allocation.project_id.expect("project_id is not null");
    let resource_id = allocation.resource_id.expect("resource_id is not null");
    
    let project_name = sqlx::query_scalar!(
        "SELECT name FROM projects WHERE id = $1",
        project_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    let resource_name = sqlx::query_scalar!(
        "SELECT name FROM resources WHERE id = $1",
        resource_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    Ok(Json(AllocationResponse {
        id: allocation.id,
        project_id,
        resource_id,
        start_date: allocation.start_date,
        end_date: allocation.end_date,
        allocation_percentage: bigdecimal_to_f64(allocation.allocation_percentage),
        project_name,
        resource_name,
    }))
}

/// Delete an allocation
async fn delete_allocation(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check if allocation exists
    let _ = sqlx::query!(
        "SELECT id FROM allocations WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Allocation {} not found", id)))?;
    
    // Delete the allocation
    sqlx::query!("DELETE FROM allocations WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    
    Ok(Json(serde_json::json!({"message": "Allocation deleted successfully"})))
}

/// Create allocation routes
pub fn allocation_routes() -> Router<PgPool> {
    Router::new()
        .route("/allocations", get(get_allocations).post(create_allocation))
        .route("/allocations/:id", put(update_allocation).delete(delete_allocation))
        .route("/allocations/project/:project_id", get(get_allocations_by_project))
        .route("/allocations/resource/:resource_id", get(get_allocations_by_resource))
}
