use axum::{
    extract::{State, Path},
    routing::{get, post, put, delete},
    Router,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, Datelike, Weekday};
use std::collections::HashMap;

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

/// Daily allocation info for validation
#[derive(Debug, Clone)]
struct DailyAllocation {
    date: NaiveDate,
    allocated_hours: f64,
    assignments: Vec<AssignmentInfo>,
}

#[derive(Debug, Clone)]
struct AssignmentInfo {
    allocation_id: Uuid,
    project_id: Uuid,
    hours: f64,
}

/// Convert BigDecimal to f64
fn bigdecimal_to_f64(bd: sqlx::types::BigDecimal) -> f64 {
    bd.to_string().parse().unwrap_or(0.0)
}

/// Convert f64 to BigDecimal
fn f64_to_bigdecimal(f: f64) -> sqlx::types::BigDecimal {
    sqlx::types::BigDecimal::try_from(f).unwrap_or_default()
}

/// Check if date is a weekend (Saturday or Sunday)
fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

/// Get holidays within date range
async fn get_holidays_in_range(
    pool: &PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<NaiveDate>> {
    let holidays: Vec<NaiveDate> = sqlx::query_scalar!(
        "SELECT date FROM holidays WHERE date >= $1 AND date <= $2",
        start_date,
        end_date
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    Ok(holidays)
}

/// Get resource working hours configuration
async fn get_resource_working_hours(
    pool: &PgPool,
    resource_id: Uuid,
) -> Result<f64> {
    let working_hours = sqlx::query_scalar!(
        "SELECT COALESCE(working_hours, 8.0) FROM resources WHERE id = $1",
        resource_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    Ok(working_hours.unwrap_or(8.0))
}

/// Get existing allocations for resource in date range
async fn get_existing_allocations(
    pool: &PgPool,
    resource_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
    exclude_allocation_id: Option<Uuid>,
) -> Result<Vec<(Uuid, NaiveDate, NaiveDate, f64)>> {
    let query = if let Some(exclude_id) = exclude_allocation_id {
        sqlx::query!(
            "SELECT id, start_date, end_date, allocation_percentage
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
        sqlx::query!(
            "SELECT id, start_date, end_date, allocation_percentage
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
    
    let allocations: Vec<(Uuid, NaiveDate, NaiveDate, f64)> = query
        .into_iter()
        .map(|row| (
            row.id,
            row.start_date,
            row.end_date,
            bigdecimal_to_f64(row.allocation_percentage)
        ))
        .collect();
    
    Ok(allocations)
}

/// Calculate daily allocations for a resource
async fn calculate_daily_allocations(
    pool: &PgPool,
    resource_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
    new_allocation_percentage: f64,
    new_start_date: NaiveDate,
    new_end_date: NaiveDate,
    exclude_allocation_id: Option<Uuid>,
) -> Result<(HashMap<NaiveDate, DailyAllocation>, f64)> {
    // Get working hours capacity
    let daily_capacity = get_resource_working_hours(pool, resource_id).await?;
    
    // Get holidays in range
    let holidays = get_holidays_in_range(pool, start_date, end_date).await?;
    let holiday_set: std::collections::HashSet<_> = holidays.into_iter().collect();
    
    // Get existing allocations
    let existing_allocations = get_existing_allocations(
        pool, resource_id, start_date, end_date, exclude_allocation_id
    )..await?;
    
    // Initialize daily allocations map
    let mut daily_allocations: HashMap<NaiveDate, DailyAllocation> = HashMap::new();
    
    // Process existing allocations
    for (alloc_id, alloc_start, alloc_end, percentage) in existing_allocations {
        let mut current_date = alloc_start;
        while current_date <= alloc_end {
            // Skip weekends and holidays
            if !is_weekend(current_date) && !holiday_set.contains(&current_date) {
                let hours = daily_capacity * (percentage / 100.0);
                
                daily_allocations
                    .entry(current_date)
                    .or_insert_with(|| DailyAllocation {
                        date: current_date,
                        allocated_hours: 0.0,
                        assignments: Vec::new(),
                    })
                    .allocated_hours += hours;
                    
                daily_allocations
                    .get_mut(&current_date)
                    .unwrap()
                    .assignments
                    .push(AssignmentInfo {
                        allocation_id: alloc_id,
                        project_id: Uuid::nil(), // Will be filled if needed
                        hours,
                    });
            }
            current_date = current_date.succ_opt().unwrap_or(current_date);
        }
    }
    
    // Add new allocation
    let mut current_date = new_start_date;
    while current_date <= new_end_date {
        if !is_weekend(current_date) && !holiday_set.contains(&current_date) {
            let new_hours = daily_capacity * (new_allocation_percentage / 100.0);
            
            daily_allocations
                .entry(current_date)
                .or_insert_with(|| DailyAllocation {
                    date: current_date,
                    allocated_hours: 0.0,
                    assignments: Vec::new(),
                })
                .allocated_hours += new_hours;
        }
        current_date = current_date.succ_opt().unwrap_or(current_date);
    }
    
    Ok((daily_allocations, daily_capacity))
}

/// Check if resource has capacity for new allocation
async fn check_resource_capacity(
    pool: &PgPool,
    resource_id: Uuid,
    new_start_date: NaiveDate,
    new_end_date: NaiveDate,
    new_allocation_percentage: f64,
    exclude_allocation_id: Option<Uuid>,
) -> Result<(bool, String, f64)> {
    // Calculate date range to check (union of all allocations)
    let (existing_start, existing_end) = if let Some(exclude_id) = exclude_allocation_id {
        let row = sqlx::query!(
            "SELECT MIN(start_date) as min_start, MAX(end_date) as max_end
             FROM allocations 
             WHERE resource_id = $1 AND id != $2",
            resource_id,
            exclude_id
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
        
        (
            row.as_ref().and_then(|r| r.min_start).unwrap_or(new_start_date),
            row.as_ref().and_then(|r| r.max_end).unwrap_or(new_end_date),
        )
    } else {
        let row = sqlx::query!(
            "SELECT MIN(start_date) as min_start, MAX(end_date) as max_end
             FROM allocations 
             WHERE resource_id = $1",
            resource_id
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
        
        (
            row.as_ref().and_then(|r| r.min_start).unwrap_or(new_start_date),
            row.as_ref().and_then(|r| r.max_end).unwrap_or(new_end_date),
        )
    };
    
    let check_start = std::cmp::min(existing_start, new_start_date);
    let check_end = std::cmp::max(existing_end, new_end_date);
    
    // Calculate daily allocations
    let (daily_allocations, daily_capacity) = calculate_daily_allocations(
        pool,
        resource_id,
        check_start,
        check_end,
        new_allocation_percentage,
        new_start_date,
        new_end_date,
        exclude_allocation_id,
    ).await?;
    
    // Check for over-allocation
    let mut over_allocated_days: Vec<(NaiveDate, f64)> = Vec::new();
    
    for (date, allocation) in &daily_allocations {
        if allocation.allocated_hours > daily_capacity {
            over_allocated_days.push((*date, allocation.allocated_hours));
        }
    }
    
    // Sort by date
    over_allocated_days.sort_by(|a, b| a.0.cmp(&b.0));
    
    let has_capacity = over_allocated_days.is_empty();
    
    let message = if has_capacity {
        format!(
            "Resource has sufficient capacity. Daily capacity: {:.1} hours",
            daily_capacity
        )
    } else {
        let days_str = over_allocated_days
            .iter()
            .map(|(date, hours)| {
                format!("{} ({:.1}h allocated)", date, hours)
            })
            .collect::<Vec<_>>()
            .join(", ");
        
        format!(
            "Resource over-allocated on: {}. Daily capacity: {:.1} hours",
            days_str, daily_capacity
        )
    };
    
    Ok((has_capacity, message, daily_capacity))
}

/// Validate allocation dates are within project dates
async fn validate_allocation_dates(
    pool: &PgPool,
    project_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<()> {
    let project = sqlx::query!(
        "SELECT start_date, end_date FROM projects WHERE id = $1",
        project_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;
    
    if start_date < project.start_date {
        return Err(AppError::Validation(format!(
            "Allocation start date ({}) cannot be before project start date ({})",
            start_date, project.start_date
        )));
    }
    
    if end_date > project.end_date {
        return Err(AppError::Validation(format!(
            "Allocation end date ({}) cannot be after project end date ({})",
            end_date, project.end_date
        )));
    }
    
    Ok(())
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
    // Validate dates are within project dates
    validate_allocation_dates(
        &pool,
        req.project_id,
        req.start_date,
        req.end_date,
    ).await?;
    
    // Check if resource has capacity
    let (has_capacity, message, _daily_capacity) = check_resource_capacity(
        &pool,
        req.resource_id,
        req.start_date,
        req.end_date,
        req.allocation_percentage,
        None,
    )
    .await?;

    if !has_capacity {
        return Err(AppError::Validation(message));
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

    // Determine values for validation
    let resource_id = req.resource_id.or(existing.resource_id).expect("resource_id is not null");
    let project_id = req.project_id.or(existing.project_id).expect("project_id is not null");
    let start_date = req.start_date.or(Some(existing.start_date)).expect("start_date is not null");
    let end_date = req.end_date.or(Some(existing.end_date)).expect("end_date is not null");
    let new_percentage = req.allocation_percentage.unwrap_or_else(|| bigdecimal_to_f64(existing.allocation_percentage));

    // Validate dates are within project dates
    validate_allocation_dates(
        &pool,
        project_id,
        start_date,
        end_date,
    ).await?;

    // Check if resource has capacity (excluding this allocation)
    let (has_capacity, message, _daily_capacity) = check_resource_capacity(
        &pool,
        resource_id,
        start_date,
        end_date,
        new_percentage,
        Some(id),
    )
    .await?;

    if !has_capacity {
        return Err(AppError::Validation(message));
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
