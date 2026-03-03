use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::routes::auth::hash_password;
use crate::services::{
    audit_log::user_claims_from_headers, audit_payload, log_audit, user_id_from_headers,
};

/// Helper to ensure standard user management routes are only for admins.
async fn require_admin(pool: &PgPool, headers: &HeaderMap) -> Result<()> {
    let claims = user_claims_from_headers(headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    if claims.role != "admin" {
        // Log denied attempt
        log_audit(
            pool,
            Uuid::parse_str(&claims.sub).ok(),
            "ACCESS_DENIED",
            "user_management",
            Uuid::nil(),
            json!({
                "reason": "insufficient_permissions",
                "attempted_role": claims.role
            }),
        )
        .await
        .ok();

        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    Ok(())
}

/// Get all users
async fn get_users(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>> {
    require_admin(&pool, &headers).await?;

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
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    require_admin(&pool, &headers).await?;

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
    headers: HeaderMap,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>> {
    require_admin(&pool, &headers).await?;

    let audit_changes = audit_payload(
        None,
        Some(json!({
            "email": req.email.clone(),
            "first_name": req.first_name.clone(),
            "last_name": req.last_name.clone(),
            "role": req.role.clone(),
            "department_id": req.department_id,
        })),
    );
    let user_id = user_id_from_headers(&headers)?;

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

    log_audit(&pool, user_id, "create", "user", user.id, audit_changes).await?;

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
    pub password: Option<String>,
}

/// Update an existing user
async fn update_user(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>> {
    require_admin(&pool, &headers).await?;

    // Check if user exists
    let existing = sqlx::query!(
        "SELECT id, email, first_name, last_name, role, department_id FROM users WHERE id = $1",
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
        || req.department_id.is_some()
        || req.password.is_some();

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

    let existing = existing.ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;
    let before_email = existing.email.clone();
    let before_first = existing.first_name.clone();
    let before_last = existing.last_name.clone();
    let before_role = existing.role.clone();
    let before_department = existing.department_id;
    let after_email_default = existing.email;
    let after_first_default = existing.first_name;
    let after_last_default = existing.last_name;
    let after_role_default = existing.role;
    let after_department_default = existing.department_id;
    let audit_changes = audit_payload(
        Some(json!({
            "email": before_email,
            "first_name": before_first,
            "last_name": before_last,
            "role": before_role,
            "department_id": before_department,
        })),
        Some(json!({
            "email": req.email.clone().unwrap_or_else(|| after_email_default),
            "first_name": req.first_name.clone().unwrap_or_else(|| after_first_default),
            "last_name": req.last_name.clone().unwrap_or_else(|| after_last_default),
            "role": req.role.clone().unwrap_or_else(|| after_role_default),
            "department_id": req.department_id.or(after_department_default),
            "password_changed": req.password.is_some(),
        })),
    );
    let user_id = user_id_from_headers(&headers)?;

    // Hash password if provided
    let password_hash = match &req.password {
        Some(pw) if !pw.is_empty() => Some(hash_password(pw)?),
        _ => None,
    };

    // Update user with COALESCE to only update provided fields
    let user = sqlx::query!(
        "UPDATE users
         SET email = COALESCE($1, email),
             first_name = COALESCE($2, first_name),
             last_name = COALESCE($3, last_name),
             role = COALESCE($4, role),
             department_id = COALESCE($5, department_id),
             password_hash = COALESCE($6, password_hash),
             updated_at = CURRENT_TIMESTAMP
         WHERE id = $7
         RETURNING id, email, first_name, last_name, role, department_id",
        req.email,
        req.first_name,
        req.last_name,
        req.role,
        req.department_id,
        password_hash,
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to update user: {}", e)))?;

    log_audit(&pool, user_id, "update", "user", user.id, audit_changes).await?;

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
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    require_admin(&pool, &headers).await?;

    // Check if user exists
    let existing = sqlx::query!(
        "SELECT id, email, first_name, last_name, role, department_id FROM users WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if existing.is_none() {
        return Err(AppError::NotFound(format!("User {} not found", id)));
    }

    let existing = existing.ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;

    sqlx::query!("DELETE FROM users WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to delete user: {}", e)))?;

    let user_id = user_id_from_headers(&headers)?;
    let audit_changes = audit_payload(
        Some(json!({
            "email": existing.email,
            "first_name": existing.first_name,
            "last_name": existing.last_name,
            "role": existing.role,
            "department_id": existing.department_id,
        })),
        None,
    );
    log_audit(&pool, user_id, "delete", "user", id, audit_changes).await?;

    Ok(Json(json!({"message": "User deleted successfully"})))
}

/// Create user routes
pub fn user_routes() -> Router<PgPool> {
    Router::new()
        .route("/users", get(get_users).post(create_user))
        .route(
            "/users/:id",
            get(get_user).put(update_user).delete(delete_user),
        )
}
