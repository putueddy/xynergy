use axum::http::HeaderMap;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde_json::{Map, Value};
use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::routes::Claims;

use sha2::{Digest, Sha256};

fn canonical_json(value: &Value) -> Result<String> {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_string(value)
                .map_err(|e| AppError::Internal(format!("Failed to serialize scalar JSON: {}", e)))
        }
        Value::Array(items) => {
            let mut out = String::from("[");
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json(item)?);
            }
            out.push(']');
            Ok(out)
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();

            let mut out = String::from("{");
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).map_err(|e| {
                    AppError::Internal(format!("Failed to serialize JSON key: {}", e))
                })?);
                out.push(':');
                if let Some(v) = map.get(*key) {
                    out.push_str(&canonical_json(v)?);
                }
            }
            out.push('}');
            Ok(out)
        }
    }
}

fn compute_entry_hash(
    user_id: Option<Uuid>,
    action: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
    changes: &Value,
    previous_hash: &str,
) -> Result<String> {
    let payload = serde_json::json!({
        "user_id": user_id,
        "action": action,
        "entity_type": entity_type,
        "entity_id": entity_id,
        "changes": changes,
        "previous_hash": previous_hash
    });

    let canonical = canonical_json(&payload)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn recompute_entry_hash(
    user_id: Option<Uuid>,
    action: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
    changes: &Value,
    previous_hash: &str,
) -> Result<String> {
    compute_entry_hash(
        user_id,
        action,
        entity_type,
        entity_id,
        changes,
        previous_hash,
    )
}

pub async fn log_audit(
    pool: &PgPool,
    user_id: Option<Uuid>,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
    changes: Value,
) -> Result<()> {
    // Acquire connection and begin transaction to serialize audit log writes globally
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Use advisory lock to serialize global audit chain insertion
    sqlx::query("SELECT pg_advisory_xact_lock(88889999)")
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Get previous hash (default to 'GENESIS' if first entry)
    let previous_hash =
        sqlx::query("SELECT entry_hash FROM audit_logs ORDER BY created_at DESC, id DESC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .and_then(|row| {
                row.try_get::<Option<String>, _>("entry_hash")
                    .ok()
                    .flatten()
            })
            .unwrap_or_else(|| "GENESIS".to_string());

    let entry_hash = compute_entry_hash(
        user_id,
        action,
        entity_type,
        Some(entity_id),
        &changes,
        &previous_hash,
    )?;

    sqlx::query(
        "INSERT INTO audit_logs (user_id, action, entity_type, entity_id, changes, previous_hash, entry_hash)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(user_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(changes)
    .bind(previous_hash)
    .bind(entry_hash)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    tx.commit()
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

pub fn user_claims_from_headers(headers: &HeaderMap) -> Result<Option<Claims>> {
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

    Ok(Some(token_data.claims))
}

pub fn user_id_from_headers(headers: &HeaderMap) -> Result<Option<Uuid>> {
    let claims_opt = user_claims_from_headers(headers)?;
    if let Some(claims) = claims_opt {
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;
        Ok(Some(user_id))
    } else {
        Ok(None)
    }
}
