use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::{audit_payload, log_audit, user_id_from_headers};

/// Department data structure
#[derive(Debug, Serialize)]
pub struct Department {
    pub id: Uuid,
    pub name: String,
    pub head_id: Option<Uuid>,
    pub head_name: Option<String>,
}

/// Get all departments with head information
async fn get_departments(State(pool): State<PgPool>) -> Result<Json<serde_json::Value>> {
    let departments = sqlx::query!(
        "SELECT d.id, d.name, d.head_id, u.first_name || ' ' || u.last_name as head_name
         FROM departments d
         LEFT JOIN users u ON d.head_id = u.id
         ORDER BY d.name"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let departments_json: Vec<serde_json::Value> = departments
        .into_iter()
        .map(|d| {
            json!({
                "id": d.id,
                "name": d.name,
                "head_id": d.head_id,
                "head_name": d.head_name
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
        "SELECT d.id, d.name, d.head_id, u.first_name || ' ' || u.last_name as head_name
         FROM departments d
         LEFT JOIN users u ON d.head_id = u.id
         WHERE d.id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Department {} not found", id)))?;

    Ok(Json(json!({
        "id": department.id,
        "name": department.name,
        "head_id": department.head_id,
        "head_name": department.head_name
    })))
}

/// Create department request
#[derive(Debug, Deserialize)]
pub struct CreateDepartmentRequest {
    pub name: String,
    pub head_id: Option<Uuid>,
}

/// Create a new department
async fn create_department(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(req): Json<CreateDepartmentRequest>,
) -> Result<Json<serde_json::Value>> {
    let audit_changes = audit_payload(None, Some(json!({
        "name": req.name.clone(),
        "head_id": req.head_id,
    })));
    let user_id = user_id_from_headers(&headers)?;
    // Validate head_id if provided
    if let Some(head_id) = req.head_id {
        let user_exists = sqlx::query!("SELECT id FROM users WHERE id = $1", head_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if user_exists.is_none() {
            return Err(AppError::Validation(format!("User {} not found", head_id)));
        }
    }

    let department = sqlx::query!(
        "INSERT INTO departments (name, head_id)
         VALUES ($1, $2)
         RETURNING id, name, head_id",
        req.name,
        req.head_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to create department: {}", e)))?;

    log_audit(
        &pool,
        user_id,
        "create",
        "department",
        department.id,
        audit_changes,
    )
    .await?;

    // Get head name if head_id is set
    let head_name = if let Some(head_id) = department.head_id {
        sqlx::query!(
            "SELECT first_name || ' ' || last_name as name FROM users WHERE id = $1",
            head_id
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .map(|r| r.name)
    } else {
        None
    };

    Ok(Json(json!({
        "id": department.id,
        "name": department.name,
        "head_id": department.head_id,
        "head_name": head_name
    })))
}

/// Update department request
#[derive(Debug, Deserialize)]
pub struct UpdateDepartmentRequest {
    pub name: Option<String>,
    pub head_id: Option<Uuid>,
}

/// Update an existing department
async fn update_department(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDepartmentRequest>,
) -> Result<Json<serde_json::Value>> {
    // Check if department exists
    let existing = sqlx::query!(
        "SELECT id, name, head_id FROM departments WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if existing.is_none() {
        return Err(AppError::NotFound(format!("Department {} not found", id)));
    }
    let existing = existing.expect("checked department exists");

    // Validate head_id if provided
    if let Some(head_id) = req.head_id {
        let user_exists = sqlx::query!("SELECT id FROM users WHERE id = $1", head_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if user_exists.is_none() {
            return Err(AppError::Validation(format!("User {} not found", head_id)));
        }
    }

    let before_name = existing.name.clone();
    let before_head_id = existing.head_id;
    let after_name_default = existing.name;
    let after_head_default = existing.head_id;
    let audit_changes = audit_payload(
        Some(json!({
            "name": before_name,
            "head_id": before_head_id,
        })),
        Some(json!({
            "name": req.name.clone().unwrap_or_else(|| after_name_default),
            "head_id": req.head_id.or(after_head_default),
        })),
    );
    let user_id = user_id_from_headers(&headers)?;

    // Update department
    let department = sqlx::query!(
        "UPDATE departments 
         SET name = COALESCE($1, name),
             head_id = COALESCE($2, head_id)
         WHERE id = $3
         RETURNING id, name, head_id",
        req.name,
        req.head_id,
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to update department: {}", e)))?;

    log_audit(
        &pool,
        user_id,
        "update",
        "department",
        department.id,
        audit_changes,
    )
    .await?;

    // Get head name if head_id is set
    let head_name = if let Some(head_id) = department.head_id {
        sqlx::query!(
            "SELECT first_name || ' ' || last_name as name FROM users WHERE id = $1",
            head_id
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .map(|r| r.name)
    } else {
        None
    };

    Ok(Json(json!({
        "id": department.id,
        "name": department.name,
        "head_id": department.head_id,
        "head_name": head_name
    })))
}

/// Delete a department
async fn delete_department(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check if department exists
    let existing = sqlx::query!(
        "SELECT id, name, head_id FROM departments WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if existing.is_none() {
        return Err(AppError::NotFound(format!("Department {} not found", id)));
    }
    let existing = existing.expect("checked department exists");

    // Check if department has users
    let user_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM users WHERE department_id = $1",
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if user_count.count.unwrap_or(0) > 0 {
        return Err(AppError::Validation(
            "Cannot delete department with assigned users. Please reassign users first."
                .to_string(),
        ));
    }

    sqlx::query!("DELETE FROM departments WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to delete department: {}", e)))?;

    let user_id = user_id_from_headers(&headers)?;
    let audit_changes = audit_payload(Some(json!({
        "name": existing.name,
        "head_id": existing.head_id,
    })), None);
    log_audit(&pool, user_id, "delete", "department", id, audit_changes).await?;

    Ok(Json(json!({"message": "Department deleted successfully"})))
}

/// Get users for department head selection
async fn get_department_head_candidates(
    State(pool): State<PgPool>,
) -> Result<Json<serde_json::Value>> {
    let users = sqlx::query!(
        "SELECT id, first_name, last_name, email 
         FROM users 
         WHERE role IN ('admin', 'project_manager')
         ORDER BY last_name, first_name"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let users_json: Vec<serde_json::Value> = users
        .into_iter()
        .map(|u| {
            json!({
                "id": u.id,
                "name": format!("{} {}", u.first_name, u.last_name),
                "email": u.email
            })
        })
        .collect();

    Ok(Json(json!(users_json)))
}

/// Create department routes
pub fn department_routes() -> Router<PgPool> {
    Router::new()
        .route("/departments", get(get_departments).post(create_department))
        .route(
            "/departments/:id",
            get(get_department)
                .put(update_department)
                .delete(delete_department),
        )
        .route(
            "/departments/head-candidates",
            get(get_department_head_candidates),
        )
}
