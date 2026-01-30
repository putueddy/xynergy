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

/// Create user routes
pub fn user_routes() -> Router<PgPool> {
    Router::new()
        .route("/users", get(get_users))
        .route("/users/:id", get(get_user))
}
