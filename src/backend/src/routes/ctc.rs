use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use bigdecimal::BigDecimal;
use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::error::{AppError, Result};
use crate::services::{
    audit_log::user_claims_from_headers, calculate_ctc, log_audit, BpjsConfig, CtcComponents,
};
use crate::services::ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService, EncryptedPayload};
use crate::services::key_provider::EnvKeyProvider;

async fn get_ctc_components(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(resource_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    // Established DB session with RLS policies configured
    let mut tx = crate::services::begin_rls_transaction(&pool, &headers).await?;

    // Find the resource via RLS-restricted transaction
    let resource = sqlx::query!(
        "SELECT id, department_id FROM resources WHERE id = $1",
        resource_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Manual check for defense-in-depth (needed if RLS is bypassed e.g. superuser or test env)
    if let Some(res) = &resource {
        if claims.role == "department_head" {
            let user_dept_query =
                sqlx::query!("SELECT department_id FROM users WHERE id = $1", user_id)
                    .fetch_optional(&pool)
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;

            let mut check_failed = true;
            if let Some(user_row) = user_dept_query {
                if res.department_id == user_row.department_id && res.department_id.is_some() {
                    check_failed = false;
                }
            }

            if check_failed {
                // Appears as RLS breach
                log_audit(
                    &pool,
                    Some(user_id),
                    "ACCESS_DENIED",
                    "ctc_components",
                    resource_id,
                    json!({
                        "reason": "cross_department_access_denied_by_rls",
                        "attempted_role": claims.role,
                    }),
                )
                .await?;
                return Err(AppError::Forbidden(
                    "Access denied by department isolation policy".to_string(),
                ));
            }
        }
    }

    if resource.is_none() {
        if claims.role == "department_head" {
            log_audit(
                &pool,
                Some(user_id),
                "ACCESS_DENIED",
                "ctc_components",
                resource_id,
                json!({
                    "reason": "cross_department_access_denied_by_rls",
                    "attempted_role": claims.role,
                }),
            )
            .await?;
            return Err(AppError::Forbidden(
                "Access denied by department isolation policy".to_string(),
            ));
        }

        return Err(AppError::NotFound("Resource not found".to_string()));
    }

    // HR cross-department view logging
    if claims.role == "hr" {
        // Verify if it's actually cross department
        let user_dept_query =
            sqlx::query!("SELECT department_id FROM users WHERE id = $1", user_id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        let mut is_cross_dept = true;
        if let Some(user_row) = user_dept_query {
            if let Some(res_row) = &resource {
                is_cross_dept = user_row.department_id != res_row.department_id;
            }
        }

        if is_cross_dept {
            log_audit(
                &pool,
                Some(user_id),
                "CTC_VIEW_CROSS_DEPT",
                "ctc_components",
                resource_id,
                json!({
                    "role": "hr",
                    "action": "cross_department_audit_log",
                }),
            )
            .await?;
        }
    }

    // HR cross-department view logging done above if role is hr...
    // Now perform standard CTC view logging for any authorized user
    log_audit(
        &pool,
        Some(user_id),
        "VIEW",
        "ctc_components",
        resource_id,
        json!({
            "action": "view_ctc",
            "role": claims.role,
        }),
    )
    .await?;

    let row_result = sqlx::query(
        "SELECT components, encrypted_components, encrypted_daily_rate, key_version, encryption_version, encryption_algorithm 
         FROM ctc_records WHERE resource_id = $1 ORDER BY updated_at DESC LIMIT 1"
    )
    .bind(resource_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut components = json!({});

    if let Some(row) = row_result {
        use sqlx::Row;
        let encrypted_components: Option<String> = row.try_get("encrypted_components").unwrap_or(None);
        let encrypted_daily_rate: Option<String> = row.try_get("encrypted_daily_rate").unwrap_or(None);
        
        if let Some(enc_str) = encrypted_components {
            if claims.role == "hr" {
                let key_version: String = row.try_get("key_version").unwrap_or_default();
                let encryption_version: String = row.try_get("encryption_version").unwrap_or_default();
                let algorithm: String = row.try_get("encryption_algorithm").unwrap_or_default();
                
                let payload = EncryptedPayload {
                    ciphertext: enc_str,
                    key_version: key_version.clone(),
                    encryption_version: encryption_version.clone(),
                    algorithm: algorithm.clone(),
                    encrypted_at: chrono::Utc::now(), // Not strictly needed for decrypt
                };
                
                let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
                let mut decrypted_components = crypto_svc.decrypt_components(&payload).await?;

                if let Some(enc_daily_rate) = encrypted_daily_rate {
                    let daily_payload = EncryptedPayload {
                        ciphertext: enc_daily_rate,
                        key_version: key_version.clone(),
                        encryption_version: encryption_version.clone(),
                        algorithm: algorithm.clone(),
                        encrypted_at: chrono::Utc::now(),
                    };

                    let decrypted_daily_rate = crypto_svc.decrypt_components(&daily_payload).await?;
                    if let Some(daily_rate_value) = decrypted_daily_rate.get("daily_rate") {
                        if let Some(obj) = decrypted_components.as_object_mut() {
                            obj.insert("daily_rate".to_string(), daily_rate_value.clone());
                        }
                    }
                }

                components = decrypted_components;
            } else {
                components = json!({"status": "encrypted", "note": "Detailed components are restricted to HR"});
            }
        } else {
            return Err(AppError::Forbidden(
                "CTC record is pending encryption migration".to_string(),
            ));
        }
    }

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(json!({
        "resource_id": resource_id,
        "components": components,
        "note": "CTC component details endpoint with RLS and audit logging"
    })))
}

#[derive(serde::Deserialize)]
pub struct UpdateCtcRequest {
    pub components: serde_json::Value,
    pub reason: String,
    pub effective_date_policy: Option<String>,
}

fn normalize_effective_date_policy(value: &str) -> Option<&'static str> {
    match value {
        "pro_rata" => Some("pro_rata"),
        "effective_first_of_month" => Some("effective_first_of_month"),
        _ => None,
    }
}

fn apply_effective_date_policy(policy: &str, today: NaiveDate) -> NaiveDate {
    if policy == "effective_first_of_month" && today.day() != 1 {
        let (year, month) = if today.month() == 12 {
            (today.year() + 1, 1)
        } else {
            (today.year(), today.month() + 1)
        };
        return NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(today);
    }

    today
}

async fn resolve_effective_date_policy(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    requested_policy: Option<&str>,
) -> Result<String> {
    let db_policy: Option<String> = sqlx::query_scalar(
        "SELECT value FROM global_settings WHERE key = 'ctc_effective_date_policy' LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some(policy) = db_policy {
        let normalized = normalize_effective_date_policy(policy.trim()).ok_or_else(|| {
            AppError::Validation(
                "Invalid ctc_effective_date_policy in global_settings".to_string(),
            )
        })?;
        return Ok(normalized.to_string());
    }

    let requested = requested_policy
        .and_then(normalize_effective_date_policy)
        .unwrap_or("pro_rata");

    sqlx::query(
        "INSERT INTO global_settings (key, value, description)
         VALUES ('ctc_effective_date_policy', $1, 'CTC effective date policy: pro_rata or effective_first_of_month')
         ON CONFLICT (key)
         DO UPDATE SET value = EXCLUDED.value, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(requested)
    .execute(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(requested.to_string())
}

async fn update_ctc_components(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(resource_id): Path<Uuid>,
    Json(payload): Json<UpdateCtcRequest>,
) -> Result<Json<serde_json::Value>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    if payload.reason.trim().is_empty() {
        return Err(AppError::Validation(
            "Update reason is required".to_string(),
        ));
    }

    if claims.role != "hr" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    let mut tx = crate::services::begin_rls_transaction(&pool, &headers).await?;

    // RLS validation (similar to view)
    let resource = sqlx::query!("SELECT id FROM resources WHERE id = $1", resource_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if resource.is_none() {
        return Err(AppError::NotFound("Resource not found".to_string()));
    }

    let after_state = payload.components.clone();

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let encrypted_payload = crypto_svc.encrypt_components(&after_state).await?;

    let existing_encrypted_daily_rate = sqlx::query_scalar::<_, Option<String>>(
        "SELECT encrypted_daily_rate FROM ctc_records WHERE resource_id = $1",
    )
    .bind(resource_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .flatten();

    let encrypted_daily_rate = if let Some(daily_rate) = extract_daily_rate_string(&after_state) {
        let encrypted_daily_rate_payload = crypto_svc
            .encrypt_components(&json!({ "daily_rate": daily_rate }))
            .await?;
        encrypted_daily_rate_payload.ciphertext
    } else if let Some(existing_ciphertext) = existing_encrypted_daily_rate {
        existing_ciphertext
    } else {
        let encrypted_daily_rate_payload = crypto_svc
            .encrypt_components(&json!({ "daily_rate": "0" }))
            .await?;
        encrypted_daily_rate_payload.ciphertext
    };

    let existing_ctc_state = sqlx::query(
        "SELECT encrypted_components, encrypted_daily_rate, key_version, encryption_version, encryption_algorithm, encrypted_at,
                effective_date, working_days_per_month, status
         FROM ctc_records
         WHERE resource_id = $1",
    )
    .bind(resource_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Get next revision number
    let next_revision: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM ctc_revisions WHERE resource_id = $1",
    )
    .bind(resource_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let requested_policy = payload
        .effective_date_policy
        .as_deref()
        .or_else(|| {
            payload
                .components
                .get("effective_date_policy")
                .and_then(|v| v.as_str())
        });

    let effective_date_policy = resolve_effective_date_policy(&mut tx, requested_policy).await?;
    let applied_effective_date = apply_effective_date_policy(&effective_date_policy, Utc::now().date_naive());

    // If this is the first revision for a legacy CTC row, persist a baseline snapshot first
    // so the first user-visible change can diff against actual prior values instead of null.
    let mut new_revision_number = next_revision;
    if next_revision == 1 {
        if let Some(row) = &existing_ctc_state {
            use sqlx::Row;
            let base_encrypted_components: String = row
                .try_get("encrypted_components")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let base_encrypted_daily_rate: Option<String> = row
                .try_get("encrypted_daily_rate")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let base_key_version: String = row
                .try_get("key_version")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let base_encryption_version: String = row
                .try_get("encryption_version")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let base_encryption_algorithm: String = row
                .try_get("encryption_algorithm")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let base_encrypted_at: chrono::DateTime<chrono::Utc> = row
                .try_get("encrypted_at")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let base_effective_date: chrono::NaiveDate = row
                .try_get("effective_date")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let base_working_days: i32 = row
                .try_get("working_days_per_month")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let base_status: String = row
                .try_get("status")
                .map_err(|e| AppError::Database(e.to_string()))?;

            sqlx::query(
                "INSERT INTO ctc_revisions (
                    resource_id, revision_number,
                    key_version, encryption_version, encryption_algorithm, encrypted_at,
                    encrypted_components, encrypted_daily_rate,
                    effective_date_policy, effective_date, working_days_per_month, status,
                    changed_by, reason, created_at
                 ) VALUES (
                    $1, 1,
                    $2, $3, $4, $5,
                    $6, $7,
                    'pro_rata', $8, $9, $10,
                    $11, 'Baseline snapshot before first revision update', CURRENT_TIMESTAMP
                 )",
            )
            .bind(resource_id)
            .bind(&base_key_version)
            .bind(&base_encryption_version)
            .bind(&base_encryption_algorithm)
            .bind(base_encrypted_at)
            .bind(&base_encrypted_components)
            .bind(&base_encrypted_daily_rate)
            .bind(base_effective_date)
            .bind(base_working_days)
            .bind(&base_status)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            new_revision_number = 2;
        }
    }

    sqlx::query(
        "INSERT INTO ctc_records (resource_id, components, encrypted_components, key_version, encryption_version, encryption_algorithm, encrypted_at, encrypted_daily_rate, updated_by, reason)
         VALUES ($1, '{}'::jsonb, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (resource_id)
         DO UPDATE SET components = '{}'::jsonb, 
                       encrypted_components = EXCLUDED.encrypted_components, 
                       key_version = EXCLUDED.key_version, 
                       encryption_version = EXCLUDED.encryption_version, 
                       encryption_algorithm = EXCLUDED.encryption_algorithm, 
                       encrypted_at = EXCLUDED.encrypted_at, 
                       encrypted_daily_rate = EXCLUDED.encrypted_daily_rate,
                       updated_by = EXCLUDED.updated_by, 
                       reason = EXCLUDED.reason, 
                       updated_at = CURRENT_TIMESTAMP",
    )
    .bind(resource_id)
    .bind(&encrypted_payload.ciphertext)
    .bind(&encrypted_payload.key_version)
    .bind(&encrypted_payload.encryption_version)
    .bind(&encrypted_payload.algorithm)
    .bind(encrypted_payload.encrypted_at)
    .bind(&encrypted_daily_rate)
    .bind(user_id)
    .bind(&payload.reason)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    sqlx::query(
        "INSERT INTO ctc_revisions (
            resource_id, revision_number,
            key_version, encryption_version, encryption_algorithm, encrypted_at,
            encrypted_components, encrypted_daily_rate,
            effective_date_policy, effective_date, working_days_per_month, status,
            changed_by, reason, created_at
        ) VALUES (
            $1, $2,
            $3, $4, $5, $6,
            $7, $8,
            $9, $10, 22, 'Active',
            $11, $12, CURRENT_TIMESTAMP
        )"
    )
    .bind(resource_id)
    .bind(new_revision_number as i32)
    .bind(&encrypted_payload.key_version)
    .bind(&encrypted_payload.encryption_version)
    .bind(&encrypted_payload.algorithm)
    .bind(encrypted_payload.encrypted_at)
    .bind(&encrypted_payload.ciphertext)
    .bind(&encrypted_daily_rate)
    .bind(&effective_date_policy)
    .bind(applied_effective_date)
    .bind(user_id)
    .bind(&payload.reason)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Log the mutation with before/after snapshots and reason string
    // Redact sensitive payload in audit logs per AC #5
    log_audit(
        &pool,
        Some(user_id),
        "UPDATE",
        "ctc_components",
        resource_id,
        json!({
            "action": "update_ctc",
            "reason": payload.reason,
            "status": "encrypted"
        }),
    )
    .await?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(json!({
        "resource_id": resource_id,
        "components": after_state,
        "policy": effective_date_policy,
        "effective_date": applied_effective_date,
        "note": "CTC mutation recorded with snapshot audit log"
    })))
}

/// Request DTO for creating a new CTC record
#[derive(Debug, Deserialize, Validate)]
pub struct CreateCtcRequest {
    pub resource_id: String,

    #[validate(range(min = 1, message = "Base salary must be a positive whole number"))]
    pub base_salary: i64,

    #[validate(range(min = 0, message = "HRA allowance must be a non-negative whole number"))]
    pub hra_allowance: i64,

    #[validate(range(
        min = 0,
        message = "Medical allowance must be a non-negative whole number"
    ))]
    pub medical_allowance: i64,

    #[validate(range(
        min = 0,
        message = "Transport allowance must be a non-negative whole number"
    ))]
    pub transport_allowance: i64,

    #[validate(range(
        min = 0,
        message = "Meal allowance must be a non-negative whole number"
    ))]
    pub meal_allowance: i64,

    #[validate(range(min = 1, max = 31, message = "Working days must be between 1 and 31"))]
    pub working_days_per_month: Option<i32>,

    /// Risk tier for JKK calculation: 1=low, 2=medium, 3=high, 4=very high
    #[validate(range(min = 1, max = 4, message = "Risk tier must be 1-4"))]
    pub risk_tier: Option<i32>,
}

/// Response DTO for CTC calculation preview
#[derive(Debug, Serialize)]
pub struct CtcCalculationResponse {
    pub resource_id: Uuid,
    pub base_salary: i64,
    pub allowances: AllowancesResponse,
    pub bpjs: BpjsResponse,
    pub thr_monthly_accrual: i64,
    pub total_monthly_ctc: i64,
    pub daily_rate: f64,
    pub working_days_per_month: i32,
}

#[derive(Debug, Serialize)]
pub struct AllowancesResponse {
    pub hra: i64,
    pub medical: i64,
    pub transport: i64,
    pub meal: i64,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct BpjsResponse {
    pub kesehatan: BpjsKesehatanResponse,
    pub ketenagakerjaan: BpjsKetenagakerjaanResponse,
}

#[derive(Debug, Serialize)]
pub struct BpjsKesehatanResponse {
    pub employer: i64,
    pub employee: i64,
}

#[derive(Debug, Serialize)]
pub struct BpjsKetenagakerjaanResponse {
    pub employer: i64,
    pub employee: i64,
}

// Helper function to convert BigDecimal to i64 (truncates decimal part)
fn bd_to_i64(bd: &BigDecimal) -> i64 {
    let s = bd.to_string();
    let int_part = s.split('.').next().unwrap_or("0");
    int_part.parse().unwrap_or(0)
}

fn bd_to_f64(bd: &BigDecimal) -> Result<f64> {
    bd.to_string()
        .parse::<f64>()
        .map_err(|_| AppError::Internal("Invalid decimal conversion for daily_rate".to_string()))
}

fn extract_daily_rate_string(components: &serde_json::Value) -> Option<String> {
    match components.get("daily_rate") {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

fn jkk_rate_for_tier(tier: i32) -> Result<BigDecimal> {
    let numerator = match tier {
        1 => 24i64,
        2 => 54i64,
        3 => 89i64,
        4 => 174i64,
        _ => {
            return Err(AppError::Validation(
                "Risk tier must be between 1 and 4".to_string(),
            ))
        }
    };

    Ok(BigDecimal::from(numerator) / BigDecimal::from(10_000i64))
}

/// Calculate BPJS preview (for confirmation before saving)
async fn calculate_bpjs_preview(
    State(_pool): State<PgPool>,
    headers: HeaderMap,
    Json(req): Json<CreateCtcRequest>,
) -> Result<Json<CtcCalculationResponse>> {
    // Authorization: HR only
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    if claims.role != "hr" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    // Validate request
    req.validate()
        .map_err(|e| AppError::Validation(format!("Validation error: {}", e)))?;

    // Manual UUID validation for resource_id
    let resource_id = Uuid::parse_str(&req.resource_id)
        .map_err(|_| AppError::Validation("Invalid resource ID format".to_string()))?;

    // Build components
    let components = CtcComponents {
        base_salary: BigDecimal::from(req.base_salary),
        hra_allowance: BigDecimal::from(req.hra_allowance),
        medical_allowance: BigDecimal::from(req.medical_allowance),
        transport_allowance: BigDecimal::from(req.transport_allowance),
        meal_allowance: BigDecimal::from(req.meal_allowance),
    };

    // Configure BPJS based on risk tier
    let mut config = BpjsConfig::default();
    if let Some(tier) = req.risk_tier {
        // Adjust JKK rate based on risk tier
        config.ketenagakerjaan_jkk_rate = jkk_rate_for_tier(tier)?;
    }

    let working_days = req.working_days_per_month.unwrap_or(22);
    let calculation = calculate_ctc(components, working_days, &config);

    // Convert BigDecimal to i64/f64 for JSON response
    let response = CtcCalculationResponse {
        resource_id,
        base_salary: req.base_salary,
        allowances: AllowancesResponse {
            hra: req.hra_allowance,
            medical: req.medical_allowance,
            transport: req.transport_allowance,
            meal: req.meal_allowance,
            total: req.hra_allowance
                + req.medical_allowance
                + req.transport_allowance
                + req.meal_allowance,
        },
        bpjs: BpjsResponse {
            kesehatan: BpjsKesehatanResponse {
                employer: bd_to_i64(&calculation.bpjs.kesehatan_employer),
                employee: bd_to_i64(&calculation.bpjs.kesehatan_employee),
            },
            ketenagakerjaan: BpjsKetenagakerjaanResponse {
                employer: bd_to_i64(&calculation.bpjs.ketenagakerjaan_employer),
                employee: bd_to_i64(&calculation.bpjs.ketenagakerjaan_employee),
            },
        },
        thr_monthly_accrual: bd_to_i64(&calculation.thr_monthly_accrual),
        total_monthly_ctc: bd_to_i64(&calculation.total_monthly_ctc),
        daily_rate: bd_to_f64(&calculation.daily_rate)?,
        working_days_per_month: calculation.working_days_per_month,
    };

    Ok(Json(response))
}

/// Create a new CTC record
async fn create_ctc_record(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(req): Json<CreateCtcRequest>,
) -> Result<Json<CtcCalculationResponse>> {
    // Authorization: HR only
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    if claims.role != "hr" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    let _user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    // Validate request
    req.validate()
        .map_err(|e| AppError::Validation(format!("Validation error: {}", e)))?;

    // Manual UUID validation for resource_id
    let resource_id = Uuid::parse_str(&req.resource_id)
        .map_err(|_| AppError::Validation("Invalid resource ID format".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    // Verify resource exists (using runtime query for compatibility)
    let resource_exists = sqlx::query("SELECT id FROM resources WHERE id = $1")
        .bind(resource_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if resource_exists.is_none() {
        return Err(AppError::NotFound(format!(
            "Resource {} not found",
            resource_id
        )));
    }

    // Check if CTC record already exists for this resource
    let existing = sqlx::query("SELECT resource_id FROM ctc_records WHERE resource_id = $1")
        .bind(resource_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if existing.is_some() {
        return Err(AppError::Validation(format!(
            "CTC record already exists for resource {}. Use update endpoint instead.",
            resource_id
        )));
    }

    // Build components
    let components = CtcComponents {
        base_salary: BigDecimal::from(req.base_salary),
        hra_allowance: BigDecimal::from(req.hra_allowance),
        medical_allowance: BigDecimal::from(req.medical_allowance),
        transport_allowance: BigDecimal::from(req.transport_allowance),
        meal_allowance: BigDecimal::from(req.meal_allowance),
    };

    // Configure BPJS based on risk tier
    let mut config = BpjsConfig::default();
    if let Some(tier) = req.risk_tier {
        config.ketenagakerjaan_jkk_rate = jkk_rate_for_tier(tier)?;
    }

    let working_days = req.working_days_per_month.unwrap_or(22);
    let calculation = calculate_ctc(components, working_days, &config);

    // Convert BigDecimal to i64 for storage
    let base_salary_i64: i64 = bd_to_i64(&calculation.components.base_salary);
    let hra_i64: i64 = bd_to_i64(&calculation.components.hra_allowance);
    let medical_i64: i64 = bd_to_i64(&calculation.components.medical_allowance);
    let transport_i64: i64 = bd_to_i64(&calculation.components.transport_allowance);
    let meal_i64: i64 = bd_to_i64(&calculation.components.meal_allowance);
    let bpjs_kes_employer: i64 = bd_to_i64(&calculation.bpjs.kesehatan_employer);
    let bpjs_kes_employee: i64 = bd_to_i64(&calculation.bpjs.kesehatan_employee);
    let bpjs_ket_employer: i64 = bd_to_i64(&calculation.bpjs.ketenagakerjaan_employer);
    let bpjs_ket_employee: i64 = bd_to_i64(&calculation.bpjs.ketenagakerjaan_employee);
    let thr_monthly: i64 = bd_to_i64(&calculation.thr_monthly_accrual);
    let total_ctc: i64 = bd_to_i64(&calculation.total_monthly_ctc);
    let daily_rate = calculation.daily_rate.with_scale(2);
    let daily_rate_str = daily_rate.to_string();

    // Insert CTC record
    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let encrypted_payload = crypto_svc.encrypt_components(&json!({
        "base_salary": base_salary_i64,
        "hra_allowance": hra_i64,
        "medical_allowance": medical_i64,
        "transport_allowance": transport_i64,
        "meal_allowance": meal_i64,
        "bpjs_kesehatan_employer": bpjs_kes_employer,
        "bpjs_kesehatan_employee": bpjs_kes_employee,
        "bpjs_ketenagakerjaan_employer": bpjs_ket_employer,
        "bpjs_ketenagakerjaan_employee": bpjs_ket_employee,
        "thr_monthly_accrual": thr_monthly,
        "total_monthly_ctc": total_ctc,
    })).await?;

    let encrypted_daily_rate_payload = crypto_svc
        .encrypt_components(&json!({ "daily_rate": daily_rate_str }))
        .await?;

    let mut tx = pool.begin().await.map_err(|e| AppError::Database(e.to_string()))?;

    sqlx::query(
        "INSERT INTO ctc_records (
            resource_id, components, encrypted_components, key_version, encryption_version, encryption_algorithm, encrypted_at, encrypted_daily_rate,
            daily_rate, working_days_per_month,
            effective_date, status, created_by, created_at, updated_by, reason
        ) VALUES (
            $1, '{}'::jsonb, $2, $3, $4, $5, $6, $7, $8::numeric, $9, CURRENT_DATE, 'Active', $10, CURRENT_TIMESTAMP, $10, 'Initial CTC record creation'
        )"
    )
    .bind(resource_id)
    .bind(&encrypted_payload.ciphertext)
    .bind(&encrypted_payload.key_version)
    .bind(&encrypted_payload.encryption_version)
    .bind(&encrypted_payload.algorithm)
    .bind(encrypted_payload.encrypted_at)
    .bind(&encrypted_daily_rate_payload.ciphertext)
    .bind("0")
    .bind(working_days)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    sqlx::query(
        "INSERT INTO ctc_revisions (
            resource_id, revision_number,
            key_version, encryption_version, encryption_algorithm, encrypted_at,
            encrypted_components, encrypted_daily_rate,
            effective_date_policy, effective_date, working_days_per_month, status,
            changed_by, reason, created_at
        ) VALUES (
            $1, 1,
            $2, $3, $4, $5,
            $6, $7,
            'pro_rata', CURRENT_DATE, $8, 'Active',
            $9, 'Initial CTC record creation', CURRENT_TIMESTAMP
        )"
    )
    .bind(resource_id)
    .bind(&encrypted_payload.key_version)
    .bind(&encrypted_payload.encryption_version)
    .bind(&encrypted_payload.algorithm)
    .bind(encrypted_payload.encrypted_at)
    .bind(&encrypted_payload.ciphertext)
    .bind(&encrypted_daily_rate_payload.ciphertext)
    .bind(working_days)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    tx.commit().await.map_err(|e| AppError::Database(e.to_string()))?;

    // Log audit event
    // Plaintext values are not included in audit logs per AC #5
    log_audit(
        &pool,
        Some(user_id),
        "CREATE",
        "ctc_record",
        resource_id,
        json!({
            "action": "create_ctc",
            "status": "encrypted",
            "working_days_per_month": working_days,
        }),
    )
    .await?;

    // Build response
    let response = CtcCalculationResponse {
        resource_id,
        base_salary: req.base_salary,
        allowances: AllowancesResponse {
            hra: req.hra_allowance,
            medical: req.medical_allowance,
            transport: req.transport_allowance,
            meal: req.meal_allowance,
            total: req.hra_allowance
                + req.medical_allowance
                + req.transport_allowance
                + req.meal_allowance,
        },
        bpjs: BpjsResponse {
            kesehatan: BpjsKesehatanResponse {
                employer: bpjs_kes_employer,
                employee: bpjs_kes_employee,
            },
            ketenagakerjaan: BpjsKetenagakerjaanResponse {
                employer: bpjs_ket_employer,
                employee: bpjs_ket_employee,
            },
        },
        thr_monthly_accrual: thr_monthly,
        total_monthly_ctc: total_ctc,
        daily_rate: bd_to_f64(&calculation.daily_rate)?,
        working_days_per_month: working_days,
    };

    Ok(Json(response))
}

#[derive(Debug, Serialize)]
pub struct CtcDiffField {
    pub field: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct CtcHistoryEntry {
    pub revision_number: i32,
    pub date: chrono::DateTime<chrono::Utc>,
    pub actor_id: Uuid,
    pub reason: String,
    pub policy: String,
    pub diffs: Vec<CtcDiffField>,
}

async fn get_ctc_history(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(resource_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    if claims.role != "hr" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    // Log the access
    log_audit(
        &pool,
        Some(user_id),
        "VIEW",
        "ctc_history",
        resource_id,
        json!({
            "action": "view_ctc_history",
            "role": claims.role,
        }),
    )
    .await?;

    let revisions = sqlx::query(
        r#"
        SELECT 
            revision_number, created_at, changed_by, reason, effective_date_policy,
            encrypted_components, encrypted_daily_rate,
            key_version, encryption_version, encryption_algorithm, encrypted_at
        FROM ctc_revisions
        WHERE resource_id = $1
        ORDER BY revision_number ASC
        "#
    )
    .bind(resource_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let mut history_entries = Vec::new();
    let mut previous_state: Option<serde_json::Value> = None;

    use sqlx::Row;
    for row in revisions {
        let rev_revision_number: i32 = row.get("revision_number");
        let rev_created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let rev_changed_by: Uuid = row.get("changed_by");
        let rev_reason: String = row.get("reason");
        let rev_effective_date_policy: String = row.get("effective_date_policy");
        
        let encrypted_components: String = row.get("encrypted_components");
        let encrypted_daily_rate: Option<String> = row.get("encrypted_daily_rate");
        let key_version: String = row.get("key_version");
        let encryption_version: String = row.get("encryption_version");
        let encryption_algorithm: String = row.get("encryption_algorithm");
        let encrypted_at: chrono::DateTime<chrono::Utc> = row.get("encrypted_at");

        let payload = EncryptedPayload {
            ciphertext: encrypted_components,
            key_version: key_version.clone(),
            encryption_version: encryption_version.clone(),
            algorithm: encryption_algorithm.clone(),
            encrypted_at,
        };
        
        let mut current_state = crypto_svc.decrypt_components(&payload).await?;

        if let Some(enc_daily) = encrypted_daily_rate {
             let daily_payload = EncryptedPayload {
                ciphertext: enc_daily,
                key_version,
                encryption_version,
                algorithm: encryption_algorithm,
                encrypted_at,
            };
            let daily_decrypted = crypto_svc.decrypt_components(&daily_payload).await?;
            if let Some(dr) = daily_decrypted.get("daily_rate") {
                if let Some(obj) = current_state.as_object_mut() {
                    obj.insert("daily_rate".to_string(), dr.clone());
                }
            }
        }

        let mut diffs = Vec::new();

        let current_obj = current_state
            .as_object()
            .cloned()
            .unwrap_or_default();

        if let Some(prev) = &previous_state {
            let prev_obj = prev.as_object().cloned().unwrap_or_default();
            
            // Check for changed or new fields
            for (k, v) in &current_obj {
                let prev_v = prev_obj.get(k).cloned().unwrap_or(serde_json::Value::Null);
                if *v != prev_v {
                    diffs.push(CtcDiffField {
                        field: k.clone(),
                        old_value: prev_v,
                        new_value: v.clone(),
                    });
                }
            }
        } else {
            // First revision, all are new
            for (k, v) in &current_obj {
                diffs.push(CtcDiffField {
                    field: k.clone(),
                    old_value: serde_json::Value::Null,
                    new_value: v.clone(),
                });
            }
        }

        history_entries.push(CtcHistoryEntry {
            revision_number: rev_revision_number,
            date: rev_created_at,
            actor_id: rev_changed_by,
            reason: rev_reason,
            policy: rev_effective_date_policy,
            diffs,
        });

        previous_state = Some(current_state);
    }

    Ok(Json(json!({
        "resource_id": resource_id,
        "history": history_entries
    })))
}

pub fn ctc_routes() -> Router<PgPool> {
    Router::new()
        .route("/ctc", post(create_ctc_record))
        .route("/ctc/calculate", post(calculate_bpjs_preview))
        .route("/ctc/:resource_id/components", get(get_ctc_components))
        .route(
            "/ctc/:resource_id/components",
            axum::routing::put(update_ctc_components),
        )
        .route("/ctc/:resource_id/history", get(get_ctc_history))
}
