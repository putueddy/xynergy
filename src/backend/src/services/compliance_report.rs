//! BPJS compliance validation report service.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::ctc_calculator::{calculate_bpjs, BpjsConfig, CtcComponents};
use crate::services::ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService, EncryptedPayload};
use crate::services::key_provider::EnvKeyProvider;

#[derive(Debug, Serialize)]
pub struct EmployeeComplianceResult {
    pub resource_id: Uuid,
    pub name: String,
    pub stored_bpjs_kes: i64,
    pub expected_bpjs_kes: i64,
    pub stored_bpjs_kt: i64,
    pub expected_bpjs_kt: i64,
    pub risk_tier: i32,
    pub status: String,
    pub variance_amount: i64,
}

#[derive(Debug, Serialize)]
pub struct ComplianceReport {
    pub results: Vec<EmployeeComplianceResult>,
    pub total_validated: i64,
    pub total_passed: i64,
    pub total_discrepancies: i64,
    pub compliance_rate_pct: f64,
}

fn jkk_rate_for_tier(tier: i32) -> Result<BigDecimal> {
    let rate = match tier {
        1 => "0.0024",
        2 => "0.0054",
        3 => "0.0089",
        4 => "0.0174",
        _ => {
            return Err(AppError::Validation(
                "Risk tier must be between 1 and 4".to_string(),
            ))
        }
    };

    rate.parse::<BigDecimal>()
        .map_err(|e| AppError::Internal(format!("Failed to parse JKK rate: {}", e)))
}

fn value_as_i64(value: &serde_json::Value, field: &str) -> Result<i64> {
    if let Some(v) = value.as_i64() {
        return Ok(v);
    }

    if let Some(v) = value.as_str() {
        return v
            .parse::<i64>()
            .map_err(|_| AppError::Validation(format!("Invalid integer for {}", field)));
    }

    Err(AppError::Validation(format!(
        "Missing or invalid numeric field: {}",
        field
    )))
}

fn value_as_i32_with_default(value: Option<&serde_json::Value>, default_value: i32) -> i32 {
    value
        .and_then(|v| v.as_i64())
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or(default_value)
}

fn bd_to_i64(value: &BigDecimal) -> i64 {
    value
        .to_string()
        .split('.')
        .next()
        .unwrap_or("0")
        .parse::<i64>()
        .unwrap_or(0)
}

fn compliance_rate(total_passed: i64, total_validated: i64) -> f64 {
    if total_validated <= 0 {
        return 0.0;
    }
    (total_passed as f64 / total_validated as f64) * 100.0
}

pub async fn validate_bpjs_compliance(
    pool: &PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<ComplianceReport> {
    let rows = sqlx::query(
        r#"
        SELECT
            c.resource_id,
            r.name,
            c.encrypted_components,
            c.key_version,
            c.encryption_version,
            c.encryption_algorithm,
            c.encrypted_at
        FROM ctc_records c
        JOIN resources r ON r.id = c.resource_id
        WHERE c.status = 'Active'
          AND r.status = 'Active'
          AND c.effective_date BETWEEN $1 AND $2
        ORDER BY r.name ASC
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let mut results = Vec::with_capacity(rows.len());

    for row in rows {
        let payload = EncryptedPayload {
            ciphertext: row
                .try_get::<String, _>("encrypted_components")
                .map_err(|e| AppError::Database(e.to_string()))?,
            key_version: row
                .try_get::<String, _>("key_version")
                .map_err(|e| AppError::Database(e.to_string()))?,
            encryption_version: row
                .try_get::<String, _>("encryption_version")
                .map_err(|e| AppError::Database(e.to_string()))?,
            algorithm: row
                .try_get::<String, _>("encryption_algorithm")
                .map_err(|e| AppError::Database(e.to_string()))?,
            encrypted_at: row
                .try_get("encrypted_at")
                .map_err(|e| AppError::Database(e.to_string()))?,
        };

        let decrypted = crypto_svc.decrypt_components(&payload).await?;
        let resource_id: Uuid = row
            .try_get("resource_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let name: String = row
            .try_get("name")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let base_salary = value_as_i64(
            decrypted.get("base_salary").ok_or_else(|| {
                AppError::Validation("Missing base_salary in CTC payload".to_string())
            })?,
            "base_salary",
        )?;
        let hra_allowance = value_as_i64(
            decrypted.get("hra_allowance").ok_or_else(|| {
                AppError::Validation("Missing hra_allowance in CTC payload".to_string())
            })?,
            "hra_allowance",
        )?;
        let medical_allowance = value_as_i64(
            decrypted.get("medical_allowance").ok_or_else(|| {
                AppError::Validation("Missing medical_allowance in CTC payload".to_string())
            })?,
            "medical_allowance",
        )?;
        let transport_allowance = value_as_i64(
            decrypted.get("transport_allowance").ok_or_else(|| {
                AppError::Validation("Missing transport_allowance in CTC payload".to_string())
            })?,
            "transport_allowance",
        )?;
        let meal_allowance = value_as_i64(
            decrypted.get("meal_allowance").ok_or_else(|| {
                AppError::Validation("Missing meal_allowance in CTC payload".to_string())
            })?,
            "meal_allowance",
        )?;
        let stored_bpjs_kes = value_as_i64(
            decrypted.get("bpjs_kesehatan_employer").ok_or_else(|| {
                AppError::Validation("Missing bpjs_kesehatan_employer in CTC payload".to_string())
            })?,
            "bpjs_kesehatan_employer",
        )?;
        let stored_bpjs_kt = value_as_i64(
            decrypted
                .get("bpjs_ketenagakerjaan_employer")
                .ok_or_else(|| {
                    AppError::Validation(
                        "Missing bpjs_ketenagakerjaan_employer in CTC payload".to_string(),
                    )
                })?,
            "bpjs_ketenagakerjaan_employer",
        )?;
        let risk_tier = value_as_i32_with_default(decrypted.get("risk_tier"), 1);

        let components = CtcComponents {
            base_salary: BigDecimal::from(base_salary),
            hra_allowance: BigDecimal::from(hra_allowance),
            medical_allowance: BigDecimal::from(medical_allowance),
            transport_allowance: BigDecimal::from(transport_allowance),
            meal_allowance: BigDecimal::from(meal_allowance),
        };

        let mut config = BpjsConfig::default();
        config.ketenagakerjaan_jkk_rate = jkk_rate_for_tier(risk_tier)?;
        let recalculated = calculate_bpjs(&components, &config);

        let expected_bpjs_kes = bd_to_i64(&recalculated.kesehatan_employer);
        let expected_bpjs_kt = bd_to_i64(&recalculated.ketenagakerjaan_employer);

        let diff_kes = (stored_bpjs_kes - expected_bpjs_kes).abs();
        let diff_kt = (stored_bpjs_kt - expected_bpjs_kt).abs();
        let has_discrepancy = diff_kes > 1 || diff_kt > 1;
        let variance_amount = diff_kes + diff_kt;

        results.push(EmployeeComplianceResult {
            resource_id,
            name,
            stored_bpjs_kes,
            expected_bpjs_kes,
            stored_bpjs_kt,
            expected_bpjs_kt,
            risk_tier,
            status: if has_discrepancy {
                "DISCREPANCY".to_string()
            } else {
                "PASS".to_string()
            },
            variance_amount,
        });
    }

    let total_validated = i64::try_from(results.len()).unwrap_or(0);
    let total_passed =
        i64::try_from(results.iter().filter(|r| r.status == "PASS").count()).unwrap_or(0);
    let total_discrepancies = total_validated.saturating_sub(total_passed);

    Ok(ComplianceReport {
        results,
        total_validated,
        total_passed,
        total_discrepancies,
        compliance_rate_pct: compliance_rate(total_passed, total_validated),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_rate_zero_total() {
        assert_eq!(compliance_rate(0, 0), 0.0);
    }

    #[test]
    fn test_compliance_rate_non_zero_total() {
        assert_eq!(compliance_rate(8, 10), 80.0);
    }

    #[test]
    fn test_jkk_rate_for_tier() {
        assert_eq!(
            jkk_rate_for_tier(1).unwrap(),
            "0.0024".parse::<BigDecimal>().unwrap()
        );
        assert!(jkk_rate_for_tier(9).is_err());
    }
}
