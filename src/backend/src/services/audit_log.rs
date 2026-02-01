use axum::http::HeaderMap;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde_json::{Map, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::routes::Claims;

pub async fn log_audit(
    pool: &PgPool,
    user_id: Option<Uuid>,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
    changes: Value,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO audit_logs (user_id, action, entity_type, entity_id, changes)
         VALUES ($1, $2, $3, $4, $5)",
        user_id,
        action,
        entity_type,
        entity_id,
        changes
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

pub fn audit_payload(before: Option<Value>, after: Option<Value>) -> Value {
    let mut map = Map::new();
    map.insert("before".to_string(), before.unwrap_or(Value::Null));
    map.insert("after".to_string(), after.unwrap_or(Value::Null));
    Value::Object(map)
}

pub fn user_id_from_headers(headers: &HeaderMap) -> Result<Option<Uuid>> {
    let auth_header = match headers.get(axum::http::header::AUTHORIZATION) {
        Some(header) => header.to_str().map_err(|_| {
            AppError::Authentication("Invalid authorization header format".to_string())
        })?,
        None => return Ok(None),
    };

    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        AppError::Authentication("Invalid authorization header format".to_string())
    })?;

    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| AppError::Internal("JWT_SECRET not set".to_string()))?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| AppError::Authentication(format!("Invalid token: {}", e)))?;

    let user_id = Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    Ok(Some(user_id))
}
