use axum::{
    extract::{State, Path},
    routing::{get, post, put, delete},
    Router,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;
use serde_json::json;
use serde::Deserialize;

use crate::error::{AppError, Result};
use crate::routes::auth::hash_password;

/// Get all users
async fn get_users(
    State(pool): State<PgPool>,
) -> Result<Json<serde_json::Value>> {
    let users = sqlx::query!(
        "SELECT id, email, first_name, last_name, role, department_id 
         FROM users ORDER BY last_name, first_name"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    let users_json: Vec<serde_json::Value> = users
        .into_iter()
        .map(|u| {
            json!({
                "id": u.id,
                "email": u.email,
                "first_name": u.first_name,
                "last_name": u.last_name,
                "role": u.role,
                "department_id": u.department_id
            })
        })
        .collect();
    
    Ok(Json(json!(users_json)))
}

/// Get user by ID
async fn get_user(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let user = sqlx::query!(
        "SELECT id, email, first_name, last_name, role, department_id 
         FROM users WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;
    
    Ok(Json(json!({
        "id": user.id,
        "email": user.email,
        "first_name": user.first_name,
        "last_name": user.last_name,
        "role": user.role,
        "department_id": user.department_id
    })))
}

/// Create user request
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub department_id: Option<Uuid>,
}

/// Create a new user
async fn create_user(
    State(pool): State<PgPool>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>> {
    // Hash the password
    let password_hash = hash_password(&req.password)?;
    
    let user = sqlx::query!(
        "INSERT INTO users (email, password_hash, first_name, last_name, role, department_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, email, first_name, last_name, role, department_id",
        req.email,
        password_hash,
        req.first_name,
        req.last_name,
        req.role,
        req.department_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to create user: {}", e)))?;
    
    Ok(Json(json!({
        "id": user.id,
        "email": user.email,
        "first_name": user.first_name,
        "last_name": user.last_name,
        "role": user.role,
        "department_id": user.department_id
    })))
}

/// Update user request
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: Option<String>,
    pub department_id: Option<Uuid>,
}

/// Update an existing user
async fn update_user(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>> {
    // Check if user exists
    let existing = sqlx::query!(
        "SELECT id FROM users WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if existing.is_none() {
        return Err(AppError::NotFound(format!("User {} not found", id)));
    }

    // Check if any fields are provided
    let has_updates = req.email.is_some()
        || req.first_name.is_some()
        || req.last_name.is_some()
        || req.role.is_some()
        || req.department_id.is_some();

    if !has_updates {
        // No fields to update, return current user
        let user = sqlx::query!(
            "SELECT id, email, first_name, last_name, role, department_id
             FROM users WHERE id = $1",
            id
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        return Ok(Json(json!({
            "id": user.id,
            "email": user.email,
            "first_name": user.first_name,
            "last_name": user.last_name,
            "role": user.role,
            "department_id": user.department_id
        })));
    }

    // Update user with COALESCE to only update provided fields
    let user = sqlx::query!(
        "UPDATE users
         SET email = COALESCE($1, email),
             first_name = COALESCE($2, first_name),
             last_name = COALESCE($3, last_name),
             role = COALESCE($4, role),
             department_id = COALESCE($5, department_id),
             updated_at = CURRENT_TIMESTAMP
         WHERE id = $6
         RETURNING id, email, first_name, last_name, role, department_id",
        req.email,
        req.first_name,
        req.last_name,
        req.role,
        req.department_id,
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to update user: {}", e)))?;

    Ok(Json(json!({
        "id": user.id,
        "email": user.email,
        "first_name": user.first_name,
        "last_name": user.last_name,
        "role": user.role,
        "department_id": user.department_id
    })))
}

/// Delete a user
async fn delete_user(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check if user exists
    let existing = sqlx::query!(
        "SELECT id FROM users WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    if existing.is_none() {
        return Err(AppError::NotFound(format!("User {} not found", id)));
    }
    
    sqlx::query!(
        "DELETE FROM users WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to delete user: {}", e)))?;
    
    Ok(Json(json!({"message": "User deleted successfully"})))
}

/// Create user routes
pub fn user_routes() -> Router<PgPool> {
    Router::new()
        .route("/users", get(get_users).post(create_user))
        .route("/users/:id", get(get_user).put(update_user).delete(delete_user))
}
