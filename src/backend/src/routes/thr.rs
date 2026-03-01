use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use bigdecimal::BigDecimal;
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService, EncryptedPayload};
use crate::services::key_provider::EnvKeyProvider;
use crate::services::{
    audit_log::user_claims_from_headers,
    log_audit,
    thr_calculator::{
        calculate_thr, calculate_thr_basis, compute_service_months, format_accrual_period,
        ThrCalculationBasis, ThrConfig,
    },
};

#[derive(Debug, Deserialize)]
struct ConfigureThrRequest {
    thr_eligible: bool,
    thr_calculation_basis: String,
    thr_employment_start_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
struct RunThrAccrualRequest {
    accrual_period: String,
}

#[derive(Debug, Deserialize)]
struct ThrReportQuery {
    month: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

fn bd_to_i64(bd: &BigDecimal) -> Result<i64> {
    let s = bd.to_string();
    let int_part = s.split('.').next().unwrap_or("0");
    int_part
        .parse()
        .map_err(|_| AppError::Internal(format!("BigDecimal '{}' cannot be converted to i64", s)))
}

fn parse_period_start(period: &str) -> Result<NaiveDate> {
    let parts: Vec<&str> = period.split('-').collect();
    if parts.len() != 2 {
        return Err(AppError::Validation(
            "accrual_period must be in YYYY-MM format".to_string(),
        ));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| AppError::Validation("Invalid year in accrual_period".to_string()))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| AppError::Validation("Invalid month in accrual_period".to_string()))?;

    if month == 0 || month > 12 {
        return Err(AppError::Validation(
            "Month in accrual_period must be between 01 and 12".to_string(),
        ));
    }

    let normalized = format_accrual_period(year, month);
    if normalized != period {
        return Err(AppError::Validation(
            "accrual_period must be in YYYY-MM format".to_string(),
        ));
    }

    NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| AppError::Validation("Invalid accrual period date".to_string()))
}

fn parse_basis(basis: &str) -> Result<ThrCalculationBasis> {
    basis.parse::<ThrCalculationBasis>().map_err(|_| {
        AppError::Validation("thr_calculation_basis must be 'full' or 'prorated'".to_string())
    })
}

fn json_value_to_big_decimal(value: Option<&serde_json::Value>, field_name: &str) -> Result<BigDecimal> {
    let v = value.ok_or_else(|| {
        AppError::Validation(format!("Missing '{}' in encrypted CTC components", field_name))
    })?;

    match v {
        serde_json::Value::String(s) => s.parse::<BigDecimal>().map_err(|_| {
            AppError::Validation(format!(
                "Invalid '{}' value in encrypted CTC components",
                field_name
            ))
        }),
        serde_json::Value::Number(n) => n.to_string().parse::<BigDecimal>().map_err(|_| {
            AppError::Validation(format!(
                "Invalid '{}' value in encrypted CTC components",
                field_name
            ))
        }),
        _ => Err(AppError::Validation(format!(
            "Invalid '{}' type in encrypted CTC components",
            field_name
        ))),
    }
}

fn amount_from_encrypted_json(payload: &serde_json::Value) -> Result<BigDecimal> {
    let value = payload
        .get("amount")
        .ok_or_else(|| AppError::Validation("Missing 'amount' in encrypted THR payload".to_string()))?;
    match value {
        serde_json::Value::String(s) => s.parse::<BigDecimal>().map_err(|_| {
            AppError::Validation("Invalid 'amount' value in encrypted THR payload".to_string())
        }),
        serde_json::Value::Number(n) => n.to_string().parse::<BigDecimal>().map_err(|_| {
            AppError::Validation("Invalid 'amount' value in encrypted THR payload".to_string())
        }),
        _ => Err(AppError::Validation(
            "Invalid 'amount' type in encrypted THR payload".to_string(),
        )),
    }
}

async fn configure_thr(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(resource_id): Path<Uuid>,
    Json(req): Json<ConfigureThrRequest>,
) -> Result<Json<serde_json::Value>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    if claims.role != "hr" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    let _ = parse_basis(&req.thr_calculation_basis)?;

    let resource_exists = sqlx::query("SELECT id FROM resources WHERE id = $1")
        .bind(resource_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if resource_exists.is_none() {
        return Err(AppError::NotFound("Resource not found".to_string()));
    }

    let ctc_exists = sqlx::query("SELECT resource_id FROM ctc_records WHERE resource_id = $1")
        .bind(resource_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if ctc_exists.is_none() {
        return Err(AppError::NotFound("CTC record not found for resource".to_string()));
    }

    sqlx::query(
        "UPDATE ctc_records
         SET thr_eligible = $1,
             thr_calculation_basis = $2,
             updated_at = CURRENT_TIMESTAMP
         WHERE resource_id = $3",
    )
    .bind(req.thr_eligible)
    .bind(&req.thr_calculation_basis)
    .bind(resource_id)
    .execute(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let has_employment_start_date = req.thr_employment_start_date.is_some();
    if let Some(start_date) = req.thr_employment_start_date {
        sqlx::query(
            "UPDATE resources SET employment_start_date = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(start_date)
        .bind(resource_id)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    let employment_start_date: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT employment_start_date FROM resources WHERE id = $1",
    )
    .bind(resource_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        Some(user_id),
        "THR_CONFIGURE",
        "thr_config",
        resource_id,
        json!({
            "status": "encrypted",
            "thr_eligible": req.thr_eligible,
            "thr_calculation_basis": req.thr_calculation_basis,
            "has_employment_start_date": has_employment_start_date,
        }),
    )
    .await?;

    Ok(Json(json!({
        "resource_id": resource_id,
        "thr_eligible": req.thr_eligible,
        "thr_calculation_basis": req.thr_calculation_basis,
        "employment_start_date": employment_start_date,
    })))
}

async fn get_thr_config(
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

    let row = sqlx::query(
        "SELECT c.resource_id, c.thr_eligible, c.thr_calculation_basis, r.employment_start_date
         FROM ctc_records c
         JOIN resources r ON r.id = c.resource_id
         WHERE c.resource_id = $1",
    )
    .bind(resource_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let row = row.ok_or_else(|| AppError::NotFound("CTC record not found for resource".to_string()))?;

    use sqlx::Row;
    let thr_eligible: bool = row
        .try_get("thr_eligible")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let thr_calculation_basis: String = row
        .try_get("thr_calculation_basis")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let employment_start_date: Option<NaiveDate> = row
        .try_get("employment_start_date")
        .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        Some(user_id),
        "VIEW",
        "thr_config",
        resource_id,
        json!({
            "status": "encrypted",
            "action": "view_thr_config",
        }),
    )
    .await?;

    Ok(Json(json!({
        "resource_id": resource_id,
        "thr_eligible": thr_eligible,
        "thr_calculation_basis": thr_calculation_basis,
        "employment_start_date": employment_start_date,
    })))
}

async fn run_thr_accrual(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(req): Json<RunThrAccrualRequest>,
) -> Result<Json<serde_json::Value>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    if claims.role != "hr" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    let accrual_date = parse_period_start(&req.accrual_period)?;
    let normalized_period = format_accrual_period(accrual_date.year(), accrual_date.month());

    let eligible_rows = sqlx::query(
        "SELECT
            c.resource_id,
            c.thr_calculation_basis,
            r.employment_start_date,
            c.encrypted_components,
            c.key_version,
            c.encryption_version,
            c.encryption_algorithm,
            c.encrypted_at
         FROM ctc_records c
         JOIN resources r ON r.id = c.resource_id
         WHERE c.thr_eligible = true",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let mut processed: i64 = 0;
    let mut skipped: i64 = 0;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let existing_accruals: Vec<Uuid> = sqlx::query_scalar(
        "SELECT resource_id FROM thr_accruals WHERE accrual_period = $1",
    )
    .bind(&normalized_period)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    let existing_set: HashSet<Uuid> = existing_accruals.into_iter().collect();

    use sqlx::Row;
    for row in eligible_rows {
        let resource_id: Uuid = row
            .try_get("resource_id")
            .map_err(|e| AppError::Database(e.to_string()))?;

        if existing_set.contains(&resource_id) {
            skipped += 1;
            continue;
        }

        let basis_str: String = row
            .try_get("thr_calculation_basis")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let employment_start_date: Option<NaiveDate> = row
            .try_get("employment_start_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encrypted_components: String = row
            .try_get("encrypted_components")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let key_version: String = row
            .try_get("key_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encryption_version: String = row
            .try_get("encryption_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encryption_algorithm: String = row
            .try_get("encryption_algorithm")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encrypted_at: chrono::DateTime<chrono::Utc> = row
            .try_get("encrypted_at")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let basis = parse_basis(&basis_str)?;

        let ctc_payload = EncryptedPayload {
            ciphertext: encrypted_components,
            key_version,
            encryption_version,
            algorithm: encryption_algorithm,
            encrypted_at,
        };

        let decrypted_ctc = crypto_svc.decrypt_components(&ctc_payload).await?;
        let base_salary = json_value_to_big_decimal(decrypted_ctc.get("base_salary"), "base_salary")?;
        let hra_allowance =
            json_value_to_big_decimal(decrypted_ctc.get("hra_allowance"), "hra_allowance")?;
        let medical_allowance =
            json_value_to_big_decimal(decrypted_ctc.get("medical_allowance"), "medical_allowance")?;
        let transport_allowance = json_value_to_big_decimal(
            decrypted_ctc.get("transport_allowance"),
            "transport_allowance",
        )?;
        let meal_allowance =
            json_value_to_big_decimal(decrypted_ctc.get("meal_allowance"), "meal_allowance")?;

        let thr_config = ThrConfig {
            eligible: true,
            calculation_basis: basis.clone(),
            employment_start_date,
        };
        let thr_calc = calculate_thr(
            &thr_config,
            &base_salary,
            &hra_allowance,
            &medical_allowance,
            &transport_allowance,
            &meal_allowance,
            accrual_date,
        );

        let accrual_amount_enc = crypto_svc
            .encrypt_components(&json!({
                "amount": bd_to_i64(&thr_calc.monthly_accrual)?,
            }))
            .await?;
        let annual_entitlement_enc = crypto_svc
            .encrypt_components(&json!({
                "amount": bd_to_i64(&thr_calc.annual_entitlement)?,
            }))
            .await?;

        let insert_result = sqlx::query(
            "INSERT INTO thr_accruals (
                id,
                resource_id,
                accrual_period,
                service_months_at_accrual,
                calculation_basis,
                encrypted_accrual_amount,
                encrypted_annual_entitlement,
                key_version,
                encryption_version,
                encryption_algorithm,
                encrypted_at,
                accrued_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
            )",
        )
        .bind(Uuid::new_v4())
        .bind(resource_id)
        .bind(&normalized_period)
        .bind(thr_calc.service_months)
        .bind(basis.as_str())
        .bind(&accrual_amount_enc.ciphertext)
        .bind(&annual_entitlement_enc.ciphertext)
        .bind(&accrual_amount_enc.key_version)
        .bind(&accrual_amount_enc.encryption_version)
        .bind(&accrual_amount_enc.algorithm)
        .bind(accrual_amount_enc.encrypted_at)
        .bind(user_id)
        .execute(&mut *tx)
        .await;

        match insert_result {
            Ok(_) => {
                processed += 1;
                log_audit(
                    &pool,
                    Some(user_id),
                    "THR_ACCRUAL",
                    "thr_accrual",
                    resource_id,
                    json!({
                        "status": "encrypted",
                        "period": normalized_period,
                        "service_months": thr_calc.service_months,
                        "basis": basis.as_str(),
                    }),
                )
                .await?;
            }
            Err(e) => {
                if e.to_string().contains("uq_thr_accruals_resource_period")
                    || e.to_string().contains("duplicate key")
                {
                    skipped += 1;
                    continue;
                }
                return Err(AppError::Database(e.to_string()));
            }
        }

    }

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(json!({
        "period": normalized_period,
        "processed": processed,
        "skipped": skipped,
    })))
}

async fn get_thr_accrual_history(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(resource_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    if claims.role != "hr" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }
    let _user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    let rows = sqlx::query(
        "SELECT
            accrual_period,
            service_months_at_accrual,
            calculation_basis,
            encrypted_accrual_amount,
            encrypted_annual_entitlement,
            key_version,
            encryption_version,
            encryption_algorithm,
            encrypted_at
         FROM thr_accruals
         WHERE resource_id = $1
         ORDER BY accrual_period DESC",
    )
    .bind(resource_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let mut accruals = Vec::new();

    use sqlx::Row;
    for row in rows {
        let period: String = row
            .try_get("accrual_period")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let service_months: i32 = row
            .try_get("service_months_at_accrual")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let basis: String = row
            .try_get("calculation_basis")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encrypted_accrual_amount: String = row
            .try_get("encrypted_accrual_amount")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encrypted_annual_entitlement: String = row
            .try_get("encrypted_annual_entitlement")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let key_version: String = row
            .try_get("key_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encryption_version: String = row
            .try_get("encryption_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let algorithm: String = row
            .try_get("encryption_algorithm")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encrypted_at: chrono::DateTime<chrono::Utc> = row
            .try_get("encrypted_at")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let accrual_payload = EncryptedPayload {
            ciphertext: encrypted_accrual_amount,
            key_version: key_version.clone(),
            encryption_version: encryption_version.clone(),
            algorithm: algorithm.clone(),
            encrypted_at,
        };
        let entitlement_payload = EncryptedPayload {
            ciphertext: encrypted_annual_entitlement,
            key_version,
            encryption_version,
            algorithm,
            encrypted_at,
        };

        let accrual_amount = amount_from_encrypted_json(&crypto_svc.decrypt_components(&accrual_payload).await?)?;
        let annual_entitlement =
            amount_from_encrypted_json(&crypto_svc.decrypt_components(&entitlement_payload).await?)?;

        accruals.push(json!({
            "period": period,
            "service_months": service_months,
            "basis": basis,
            "accrual_amount": bd_to_i64(&accrual_amount)?,
            "annual_entitlement": bd_to_i64(&annual_entitlement)?,
        }));
    }

    Ok(Json(json!({
        "resource_id": resource_id,
        "accruals": accruals,
    })))
}

async fn get_thr_payout_report(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<ThrReportQuery>,
) -> Result<Json<serde_json::Value>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    if claims.role != "hr" {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    let month = query
        .month
        .ok_or_else(|| AppError::Validation("month query parameter is required".to_string()))?;
    let payout_date = parse_period_start(&month)?;
    let payout_period = format_accrual_period(payout_date.year(), payout_date.month());

    let limit = query.limit.unwrap_or(50).max(1).min(200);
    let offset = query.offset.unwrap_or(0).max(0);

    let rows = sqlx::query(
        "SELECT
            c.resource_id,
            c.thr_calculation_basis,
            r.employment_start_date,
            c.encrypted_components,
            c.key_version,
            c.encryption_version,
            c.encryption_algorithm,
            c.encrypted_at
         FROM ctc_records c
         JOIN resources r ON r.id = c.resource_id
         WHERE c.thr_eligible = true
         ORDER BY c.resource_id
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let mut entries = Vec::new();

    let all_accrual_rows = sqlx::query(
        "SELECT resource_id, encrypted_accrual_amount, key_version, encryption_version, encryption_algorithm, encrypted_at
         FROM thr_accruals WHERE accrual_period <= $1",
    )
    .bind(&payout_period)
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut accruals_by_resource: HashMap<Uuid, Vec<(String, String, String, String, chrono::DateTime<chrono::Utc>)>> =
        HashMap::new();
    for accrual_row in all_accrual_rows {
        let resource_id: Uuid = accrual_row
            .try_get("resource_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encrypted_accrual_amount: String = accrual_row
            .try_get("encrypted_accrual_amount")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let key_version: String = accrual_row
            .try_get("key_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encryption_version: String = accrual_row
            .try_get("encryption_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encryption_algorithm: String = accrual_row
            .try_get("encryption_algorithm")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encrypted_at: chrono::DateTime<chrono::Utc> = accrual_row
            .try_get("encrypted_at")
            .map_err(|e| AppError::Database(e.to_string()))?;

        accruals_by_resource
            .entry(resource_id)
            .or_default()
            .push((
                encrypted_accrual_amount,
                key_version,
                encryption_version,
                encryption_algorithm,
                encrypted_at,
            ));
    }

    use sqlx::Row;
    for row in rows {
        let resource_id: Uuid = row
            .try_get("resource_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let basis_str: String = row
            .try_get("thr_calculation_basis")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let employment_start_date: Option<NaiveDate> = row
            .try_get("employment_start_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encrypted_components: String = row
            .try_get("encrypted_components")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let key_version: String = row
            .try_get("key_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encryption_version: String = row
            .try_get("encryption_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let algorithm: String = row
            .try_get("encryption_algorithm")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encrypted_at: chrono::DateTime<chrono::Utc> = row
            .try_get("encrypted_at")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let ctc_payload = EncryptedPayload {
            ciphertext: encrypted_components,
            key_version,
            encryption_version,
            algorithm,
            encrypted_at,
        };
        let decrypted_ctc = crypto_svc.decrypt_components(&ctc_payload).await?;

        let base_salary = json_value_to_big_decimal(decrypted_ctc.get("base_salary"), "base_salary")?;
        let hra_allowance =
            json_value_to_big_decimal(decrypted_ctc.get("hra_allowance"), "hra_allowance")?;
        let medical_allowance =
            json_value_to_big_decimal(decrypted_ctc.get("medical_allowance"), "medical_allowance")?;
        let transport_allowance = json_value_to_big_decimal(
            decrypted_ctc.get("transport_allowance"),
            "transport_allowance",
        )?;
        let meal_allowance =
            json_value_to_big_decimal(decrypted_ctc.get("meal_allowance"), "meal_allowance")?;

        let thr_basis_amount = calculate_thr_basis(
            &base_salary,
            &hra_allowance,
            &medical_allowance,
            &transport_allowance,
            &meal_allowance,
        );

        let service_months = compute_service_months(employment_start_date, payout_date);
        let basis = parse_basis(&basis_str)?;
        let thr_config = ThrConfig {
            eligible: true,
            calculation_basis: basis.clone(),
            employment_start_date,
        };
        let calculation = calculate_thr(
            &thr_config,
            &base_salary,
            &hra_allowance,
            &medical_allowance,
            &transport_allowance,
            &meal_allowance,
            payout_date,
        );

        let mut accrued_to_date = BigDecimal::from(0i64);
        if let Some(accrual_rows) = accruals_by_resource.get(&resource_id) {
            for (encrypted_accrual_amount, key_version, encryption_version, encryption_algorithm, encrypted_at) in
                accrual_rows
            {
            let payload = EncryptedPayload {
                    ciphertext: encrypted_accrual_amount.clone(),
                    key_version: key_version.clone(),
                    encryption_version: encryption_version.clone(),
                    algorithm: encryption_algorithm.clone(),
                    encrypted_at: *encrypted_at,
            };
            let amount_json = crypto_svc.decrypt_components(&payload).await?;
            accrued_to_date += amount_from_encrypted_json(&amount_json)?;
            }
        }

        let remaining_top_up = calculation.annual_entitlement.clone() - accrued_to_date.clone();
        let basis_explanation = if basis == ThrCalculationBasis::Full {
            if service_months >= 12 {
                "full entitlement (>=12 months service)"
            } else {
                "full basis configured, prorated due to service < 12 months"
            }
        } else {
            "prorated basis configured"
        };

        entries.push(json!({
            "resource_id": resource_id,
            "month": payout_period,
            "service_months": service_months,
            "calculation_basis": basis.as_str(),
            "calculation_basis_explanation": basis_explanation,
            "thr_basis_amount": bd_to_i64(&thr_basis_amount)?,
            "annual_entitlement": bd_to_i64(&calculation.annual_entitlement)?,
            "accrued_to_date": bd_to_i64(&accrued_to_date)?,
            "remaining_top_up": bd_to_i64(&remaining_top_up)?,
        }));
    }

    log_audit(
        &pool,
        Some(user_id),
        "THR_REPORT_VIEW",
        "thr_report",
        user_id,
        json!({
            "status": "encrypted",
            "month": payout_period,
            "limit": limit,
            "offset": offset,
        }),
    )
    .await?;

    Ok(Json(json!({
        "month": payout_period,
        "limit": limit,
        "offset": offset,
        "entries": entries,
    })))
}

pub fn thr_routes() -> Router<PgPool> {
    Router::new()
        .route("/thr/configure/:resource_id", post(configure_thr))
        .route("/thr/config/:resource_id", get(get_thr_config))
        .route("/thr/accrual/run", post(run_thr_accrual))
        .route("/thr/accrual/:resource_id", get(get_thr_accrual_history))
        .route("/thr/report", get(get_thr_payout_report))
}
