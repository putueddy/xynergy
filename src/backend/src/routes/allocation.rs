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

/// Check if resource has capacity for new allocation
async fn check_resource_capacity(
    pool: &PgPool,
    resource_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
    new_allocation_percentage: f64,
    exclude_allocation_id: Option<Uuid>,
) -> Result<(bool, f64, f64)> {
    // Get resource capacity
    let resource_capacity = sqlx::query_scalar!(
        "SELECT capacity FROM resources WHERE id = $1",
        resource_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let capacity = resource_capacity
        .map(bigdecimal_to_f64)
        .unwrap_or(8.0); // Default 8 hours/day if not set

    // Calculate total allocated hours for the resource in the date range
    // Use a single query with optional exclusion
    let existing_allocations: Vec<sqlx::types::BigDecimal> = if let Some(exclude_id) = exclude_allocation_id {
        sqlx::query_scalar!(
            "SELECT allocation_percentage
             FROM allocations 
             WHERE resource_id = $1 
             AND id != $2
             AND (
                 (start_date <= $3 AND end_date >= $3) OR
                 (start_date <= $4 AND end_date >= $4) OR
                 (start_date >= $3 AND end_date <= $4)
             )",
            resource_id,
            exclude_id,
            start_date,
            end_date
        )
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
    } else {
        sqlx::query_scalar!(
            "SELECT allocation_percentage
             FROM allocations 
             WHERE resource_id = $1 
             AND (
                 (start_date <= $2 AND end_date >= $2) OR
                 (start_date <= $3 AND end_date >= $3) OR
                 (start_date >= $2 AND end_date <= $3)
             )",
            resource_id,
            start_date,
            end_date
        )
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
    };

    // Calculate total allocation percentage (assuming percentage is % of capacity)
    let total_existing_percentage: f64 = existing_allocations
        .iter()
        .map(|bd| bigdecimal_to_f64(bd.clone()))
        .sum();

    let total_percentage = total_existing_percentage + new_allocation_percentage;
    let has_capacity = total_percentage <= 100.0;

    Ok((has_capacity, total_percentage, capacity))
}

/// Create a new allocation
async fn create_allocation(
    State(pool): State<PgPool>,
    Json(req): Json<CreateAllocationRequest>,
) -> Result<Json<AllocationResponse>> {
    // Check if resource has capacity
    let (has_capacity, total_percentage, capacity) = check_resource_capacity(
        &pool,
        req.resource_id,
        req.start_date,
        req.end_date,
        req.allocation_percentage,
        None,
    )
    .await?;

    if !has_capacity {
        return Err(AppError::Validation(format!(
            "Resource over-allocated: total allocation would be {:.0}% (capacity: {:.0} hours/day). \
             Please reduce allocation or choose different dates.",
            total_percentage, capacity
        )));
    }

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
        "SELECT id, project_id, resource_id, start_date, end_date, allocation_percentage FROM allocations WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Allocation {} not found", id)))?;

    // Determine values for capacity check
    let resource_id = req.resource_id.or(existing.resource_id).expect("resource_id is not null");
    let start_date = req.start_date.or(Some(existing.start_date)).expect("start_date is not null");
    let end_date = req.end_date.or(Some(existing.end_date)).expect("end_date is not null");
    let new_percentage = req.allocation_percentage.unwrap_or_else(|| bigdecimal_to_f64(existing.allocation_percentage));

    // Check if resource has capacity (excluding this allocation)
    let (has_capacity, total_percentage, capacity) = check_resource_capacity(
        &pool,
        resource_id,
        start_date,
        end_date,
        new_percentage,
        Some(id),
    )
    .await?;

    if !has_capacity {
        return Err(AppError::Validation(format!(
            "Resource over-allocated: total allocation would be {:.0}% (capacity: {:.0} hours/day). \
             Please reduce allocation or choose different dates.",
            total_percentage, capacity
        )));
    }

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
