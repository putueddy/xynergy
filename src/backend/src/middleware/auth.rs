use axum::{
    extract::{State, Request},
    middleware::Next,
    response::Response,
    http::{header::AUTHORIZATION},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use sqlx::PgPool;

use crate::error::{AppError, Result};
use crate::routes::Claims;

/// Extract token from Authorization header
fn extract_token(auth_header: &str) -> Option<&str> {
    auth_header.strip_prefix("Bearer ")
}

/// Authentication middleware
pub async fn auth_middleware(
    State(pool): State<PgPool>,
    req: Request,
    next: Next,
) -> Result<Response> {
    // Get authorization header
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| AppError::Authentication("Missing authorization header".to_string()))?;
    
    // Extract token
    let token = extract_token(auth_header)
        .ok_or_else(|| AppError::Authentication("Invalid authorization header format".to_string()))?;
    
    // Decode and verify token
    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| AppError::Internal("JWT_SECRET not set".to_string()))?;
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| AppError::Authentication(format!("Invalid token: {}", e)))?;
    
    // Check if user still exists in database
    let user_id = uuid::Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;
    
    let user_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)"
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    if !user_exists {
        return Err(AppError::Authentication("User no longer exists".to_string()));
    }
    
    // Continue to the next middleware/handler
    Ok(next.run(req).await)
}
