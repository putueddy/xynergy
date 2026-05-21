//! CTC validation/reconciliation report service.
//!
//! Compares Xynergy CTC records against the local payroll staging source
//! (`payroll_validation_staging`) and produces a finance-facing report
//! with summary metrics, deterministic mismatch ordering, and explicit
//! exclusion tracking. Reuses BPJS recalculation from `compliance_report`
//! and shared crypto/key-provider services.
//!
//! Decision notes (Story 5.3):
//! - **CTC effective-date selection (Decision D):** uses
//!   `effective_date BETWEEN start_date AND end_date`, identical to
//!   `compliance_report::validate_bpjs_compliance`. The two reports stay
//!   comparable. A future "active CTC as-of date X" view is intentionally
//!   a separate endpoint, not a flag on this one.
//! - **Multiple payroll rows per resource (Decision C):** snapshot
//!   reconciliation — one comparison per resource against the latest
//!   payroll baseline whose `effective_date` falls in the range, tied by
//!   `imported_at DESC, import_batch_id DESC`. "Compare every payroll row
//!   in the range" is deferred (can be added later as `?mode=all`).

use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::ctc_calculator::{
    calculate_bpjs, jkk_rate_for_tier, BpjsConfig, CtcComponents,
};
use crate::services::ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService, EncryptedPayload};
use crate::services::key_provider::EnvKeyProvider;

const DEFAULT_MISMATCH_LIMIT: i64 = 100;
const MAX_MISMATCH_LIMIT: i64 = 500;
const MAX_EXCLUDED_ROWS: usize = 500;
/// Max days between `end_date` and the most-recent `imported_at` in the
/// requested range before the payroll staging data is considered stale.
pub const PAYROLL_FRESHNESS_DAYS: i64 = 7;
/// Hard cap on how many employee ids a sampled run may target. Keeps the
/// URL bounded and the PostgreSQL `= ANY($)` parameter list reasonable.
pub const MAX_SAMPLED_EMPLOYEE_IDS: usize = 200;

/// A single mismatch row in the report.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationMismatch {
    pub employee_id: Uuid,
    pub employee_name: String,
    pub field_name: String,
    pub xynergy_value: i64,
    pub payroll_value: i64,
    pub variance_amount: i64,
    pub status: String,
    pub bpjs_metadata: Option<BpjsMismatchMetadata>,
}

/// Optional BPJS-specific metadata attached to a mismatch row.
#[derive(Debug, Clone, Serialize)]
pub struct BpjsMismatchMetadata {
    pub risk_tier: i32,
    pub recalculated_value: i64,
}

/// A record that was excluded from the comparison (kept visible to Finance).
#[derive(Debug, Clone, Serialize)]
pub struct ExcludedRecord {
    pub employee_id: Uuid,
    pub employee_name: String,
    pub reason: String,
}

/// Full validation report response.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_compared: i64,
    pub total_matches: i64,
    pub total_discrepancies: i64,
    pub match_rate_pct: f64,
    pub excluded_count: i64,
    pub bpjs_error_count: i64,
    /// `true` when the caller restricted the run with `employee_ids`.
    pub sampled: bool,
    /// Distinct CTC resources with a payroll row / distinct CTC resources *
    /// 100 over the requested range. Surfaced for Finance to spot incomplete
    /// imports — below 90% typically means the payroll batch missed employees.
    pub payroll_coverage_pct: f64,
    pub mismatches: Vec<ValidationMismatch>,
    pub excluded: Vec<ExcludedRecord>,
}

/// Filter inputs for the validation report.
#[derive(Debug, Clone)]
pub struct ValidationReportFilters {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Optional sample restriction. `None` → validate every employee with
    /// an active CTC in the range (comprehensive run). `Some(ids)` →
    /// validate only the listed resource ids. Capped at
    /// `MAX_SAMPLED_EMPLOYEE_IDS` by the route handler.
    pub employee_ids: Option<Vec<Uuid>>,
}

fn match_rate(total_matches: i64, total_compared: i64) -> f64 {
    if total_compared <= 0 {
        return 0.0;
    }
    (total_matches as f64 / total_compared as f64) * 100.0
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

fn value_as_i64(value: Option<&serde_json::Value>) -> Option<i64> {
    let v = value?;
    if let Some(n) = v.as_i64() {
        return Some(n);
    }
    if let Some(s) = v.as_str() {
        return s.parse::<i64>().ok();
    }
    None
}

fn value_as_i32_with_default(
    value: Option<&serde_json::Value>,
    default_value: i32,
    field_name: &str,
) -> Result<i32> {
    let Some(value) = value else {
        return Ok(default_value);
    };

    if let Some(v) = value.as_i64() {
        return i32::try_from(v)
            .map_err(|_| AppError::Validation(format!("Invalid i32 for field: {}", field_name)));
    }

    if let Some(v) = value.as_str() {
        return v
            .parse::<i32>()
            .map_err(|_| AppError::Validation(format!("Invalid i32 for field: {}", field_name)));
    }

    Err(AppError::Validation(format!(
        "Invalid numeric field: {}",
        field_name
    )))
}

struct PayrollRow {
    resource_id: Uuid,
    employee_name: String,
    effective_date: NaiveDate,
    imported_at: DateTime<Utc>,
    base_salary: i64,
    hra_allowance: i64,
    medical_allowance: i64,
    transport_allowance: i64,
    meal_allowance: i64,
    bpjs_kesehatan_employer: i64,
    bpjs_ketenagakerjaan_employer: i64,
}

struct CtcRow {
    resource_id: Uuid,
    name: String,
    encrypted_payload: Option<EncryptedPayload>,
}

async fn load_payroll_rows(
    tx: &mut Transaction<'_, Postgres>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    employee_ids: Option<&[Uuid]>,
) -> Result<Vec<PayrollRow>> {
    let id_filter: Vec<Uuid> = employee_ids.map(<[Uuid]>::to_vec).unwrap_or_default();
    let rows = sqlx::query(
        r#"
        SELECT
            p.resource_id,
            r.name AS employee_name,
            p.effective_date,
            p.imported_at,
            p.base_salary,
            p.hra_allowance,
            p.medical_allowance,
            p.transport_allowance,
            p.meal_allowance,
            p.bpjs_kesehatan_employer,
            p.bpjs_ketenagakerjaan_employer
        FROM payroll_validation_staging p
        JOIN resources r ON r.id = p.resource_id
        WHERE p.effective_date BETWEEN $1 AND $2
          AND r.resource_type = 'employee'
          AND (cardinality($3::uuid[]) = 0 OR p.resource_id = ANY($3::uuid[]))
        ORDER BY p.resource_id, p.effective_date DESC, p.imported_at DESC, p.import_batch_id DESC
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .bind(&id_filter)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(PayrollRow {
            resource_id: row
                .try_get("resource_id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            employee_name: row
                .try_get("employee_name")
                .map_err(|e| AppError::Database(e.to_string()))?,
            effective_date: row
                .try_get("effective_date")
                .map_err(|e| AppError::Database(e.to_string()))?,
            imported_at: row
                .try_get("imported_at")
                .map_err(|e| AppError::Database(e.to_string()))?,
            base_salary: row
                .try_get("base_salary")
                .map_err(|e| AppError::Database(e.to_string()))?,
            hra_allowance: row
                .try_get("hra_allowance")
                .map_err(|e| AppError::Database(e.to_string()))?,
            medical_allowance: row
                .try_get("medical_allowance")
                .map_err(|e| AppError::Database(e.to_string()))?,
            transport_allowance: row
                .try_get("transport_allowance")
                .map_err(|e| AppError::Database(e.to_string()))?,
            meal_allowance: row
                .try_get("meal_allowance")
                .map_err(|e| AppError::Database(e.to_string()))?,
            bpjs_kesehatan_employer: row
                .try_get("bpjs_kesehatan_employer")
                .map_err(|e| AppError::Database(e.to_string()))?,
            bpjs_ketenagakerjaan_employer: row
                .try_get("bpjs_ketenagakerjaan_employer")
                .map_err(|e| AppError::Database(e.to_string()))?,
        });
    }

    Ok(out)
}

async fn load_ctc_rows(
    tx: &mut Transaction<'_, Postgres>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    employee_ids: Option<&[Uuid]>,
) -> Result<Vec<CtcRow>> {
    let id_filter: Vec<Uuid> = employee_ids.map(<[Uuid]>::to_vec).unwrap_or_default();
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
          AND r.resource_type = 'employee'
          AND c.effective_date BETWEEN $1 AND $2
          AND (cardinality($3::uuid[]) = 0 OR c.resource_id = ANY($3::uuid[]))
        ORDER BY r.name ASC, c.resource_id ASC
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .bind(&id_filter)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let encrypted_components: Option<String> = row
            .try_get("encrypted_components")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let key_version: Option<String> = row
            .try_get("key_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encryption_version: Option<String> = row
            .try_get("encryption_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let algorithm: Option<String> = row
            .try_get("encryption_algorithm")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encrypted_at: Option<DateTime<Utc>> = row
            .try_get("encrypted_at")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let encrypted_payload = match (
            encrypted_components,
            key_version,
            encryption_version,
            algorithm,
            encrypted_at,
        ) {
            (
                Some(ciphertext),
                Some(key_version),
                Some(encryption_version),
                Some(algorithm),
                Some(encrypted_at),
            ) => Some(EncryptedPayload {
                ciphertext,
                key_version,
                encryption_version,
                algorithm,
                encrypted_at,
            }),
            _ => None,
        };

        out.push(CtcRow {
            resource_id: row
                .try_get("resource_id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            name: row
                .try_get("name")
                .map_err(|e| AppError::Database(e.to_string()))?,
            encrypted_payload,
        });
    }

    Ok(out)
}

async fn validate_sampled_resources(
    tx: &mut Transaction<'_, Postgres>,
    employee_ids: &[Uuid],
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<()> {
    let visible_employees: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT r.id)
          FROM resources r
         WHERE r.resource_type = 'employee'
           AND r.id = ANY($1::uuid[])
        "#,
    )
    .bind(employee_ids)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if visible_employees != employee_ids.len() as i64 {
        return Err(AppError::Validation(format!(
            "Sample contains {} employee id(s) that are not visible employee resources",
            employee_ids
                .len()
                .saturating_sub(visible_employees as usize)
        )));
    }

    let ctc_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT c.resource_id)
          FROM ctc_records c
          JOIN resources r ON r.id = c.resource_id
         WHERE c.status = 'Active'
           AND r.resource_type = 'employee'
           AND c.effective_date BETWEEN $2 AND $3
           AND c.resource_id = ANY($1::uuid[])
        "#,
    )
    .bind(employee_ids)
    .bind(start_date)
    .bind(end_date)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if ctc_count != employee_ids.len() as i64 {
        return Err(AppError::Validation(format!(
            "Sample contains {} employee id(s) without active Xynergy CTC records in the selected range",
            employee_ids.len().saturating_sub(ctc_count as usize)
        )));
    }

    let payroll_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT p.resource_id)
          FROM payroll_validation_staging p
          JOIN resources r ON r.id = p.resource_id
         WHERE p.effective_date BETWEEN $2 AND $3
           AND r.resource_type = 'employee'
           AND p.resource_id = ANY($1::uuid[])
        "#,
    )
    .bind(employee_ids)
    .bind(start_date)
    .bind(end_date)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if payroll_count != employee_ids.len() as i64 {
        return Err(AppError::Validation(format!(
            "Sample contains {} employee id(s) without payroll staging data in the selected range",
            employee_ids.len().saturating_sub(payroll_count as usize)
        )));
    }

    Ok(())
}

/// Validate that the payroll staging source is usable for the requested range.
///
/// Returns an error if:
/// - There is no data at all in the requested range (finance cannot reconcile
///   without a payroll-side baseline), or
/// - For sampled reports, every requested id must resolve to a visible
///   employee with both active CTC and payroll staging rows in the range.
async fn validate_payroll_source(
    tx: &mut Transaction<'_, Postgres>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    employee_ids: Option<&[Uuid]>,
) -> Result<()> {
    let id_filter: Vec<Uuid> = employee_ids.map(<[Uuid]>::to_vec).unwrap_or_default();
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM payroll_validation_staging p
          JOIN resources r ON r.id = p.resource_id
         WHERE p.effective_date BETWEEN $1 AND $2
           AND r.resource_type = 'employee'
           AND (cardinality($3::uuid[]) = 0 OR p.resource_id = ANY($3::uuid[]))
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .bind(&id_filter)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if count == 0 {
        return Err(AppError::Validation(format!(
            "No payroll staging data found between {} and {}. Import payroll records before running the validation report.",
            start_date, end_date
        )));
    }

    if let Some(ids) = employee_ids {
        validate_sampled_resources(tx, ids, start_date, end_date).await?;
    }

    Ok(())
}

/// Distinct CTC resources covered by payroll / distinct CTC resources * 100,
/// rounded to two decimals. Returns 0.0 when there are no CTC rows (the
/// denominator) so finance does not see a divide-by-zero anomaly.
async fn compute_payroll_coverage_pct(
    tx: &mut Transaction<'_, Postgres>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    employee_ids: Option<&[Uuid]>,
) -> Result<f64> {
    let id_filter: Vec<Uuid> = employee_ids.map(<[Uuid]>::to_vec).unwrap_or_default();

    let ctc_distinct: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(DISTINCT c.resource_id)
             FROM ctc_records c
             JOIN resources r ON r.id = c.resource_id
            WHERE c.status = 'Active'
              AND r.resource_type = 'employee'
              AND c.effective_date BETWEEN $1 AND $2
              AND (cardinality($3::uuid[]) = 0 OR c.resource_id = ANY($3::uuid[]))"#,
    )
    .bind(start_date)
    .bind(end_date)
    .bind(&id_filter)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if ctc_distinct <= 0 {
        return Ok(0.0);
    }

    let covered_ctc_resources: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(DISTINCT c.resource_id)
             FROM ctc_records c
             JOIN resources r ON r.id = c.resource_id
            WHERE c.status = 'Active'
              AND r.resource_type = 'employee'
              AND c.effective_date BETWEEN $1 AND $2
              AND (cardinality($3::uuid[]) = 0 OR c.resource_id = ANY($3::uuid[]))
              AND EXISTS (
                  SELECT 1
                    FROM payroll_validation_staging p
                   WHERE p.resource_id = c.resource_id
                     AND p.effective_date BETWEEN $1 AND $2
              )"#,
    )
    .bind(start_date)
    .bind(end_date)
    .bind(&id_filter)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let pct = (covered_ctc_resources as f64 / ctc_distinct as f64) * 100.0;
    Ok((pct * 100.0).round() / 100.0)
}

fn selected_payroll_freshness(reference_end_date: NaiveDate, rows: &[&PayrollRow]) -> Result<()> {
    let today = chrono::Utc::now().date_naive();
    let reference_date = reference_end_date.min(today);

    for row in rows {
        let imported_date = row.imported_at.date_naive();
        let days_old = (reference_date - imported_date).num_days();
        if days_old > PAYROLL_FRESHNESS_DAYS {
            return Err(AppError::Validation(format!(
                "Payroll staging data is stale for {}: selected baseline import was {} days before {} (max {} days allowed). Re-import payroll records before running the validation report.",
                row.employee_name, days_old, reference_date, PAYROLL_FRESHNESS_DAYS
            )));
        }
    }

    Ok(())
}

fn payroll_components(payroll: &PayrollRow) -> CtcComponents {
    CtcComponents {
        base_salary: BigDecimal::from(payroll.base_salary),
        hra_allowance: BigDecimal::from(payroll.hra_allowance),
        medical_allowance: BigDecimal::from(payroll.medical_allowance),
        transport_allowance: BigDecimal::from(payroll.transport_allowance),
        meal_allowance: BigDecimal::from(payroll.meal_allowance),
    }
}

fn xynergy_components(xynergy: &serde_json::Value) -> Option<CtcComponents> {
    Some(CtcComponents {
        base_salary: BigDecimal::from(value_as_i64(xynergy.get("base_salary"))?),
        hra_allowance: BigDecimal::from(value_as_i64(xynergy.get("hra_allowance"))?),
        medical_allowance: BigDecimal::from(value_as_i64(xynergy.get("medical_allowance"))?),
        transport_allowance: BigDecimal::from(value_as_i64(xynergy.get("transport_allowance"))?),
        meal_allowance: BigDecimal::from(value_as_i64(xynergy.get("meal_allowance"))?),
    })
}

/// Compare one decrypted Xynergy CTC row against the matching payroll row.
///
/// Pushes mismatches in canonical field order (deterministic). The BPJS
/// fields compare Xynergy and payroll values while validating each side
/// against the regulation-derived value for its own salary/allowance basis.
fn compare_record(
    employee_id: Uuid,
    employee_name: &str,
    xynergy: &serde_json::Value,
    payroll: &PayrollRow,
    mismatches: &mut Vec<ValidationMismatch>,
) -> Result<i64> {
    let mut bpjs_errors: i64 = 0;

    // Plain fields - compare Xynergy stored value vs payroll value.
    let plain_fields: &[(&str, i64)] = &[
        ("base_salary", payroll.base_salary),
        ("hra_allowance", payroll.hra_allowance),
        ("medical_allowance", payroll.medical_allowance),
        ("transport_allowance", payroll.transport_allowance),
        ("meal_allowance", payroll.meal_allowance),
    ];

    for (field_name, payroll_value) in plain_fields {
        let Some(xy_value) = value_as_i64(xynergy.get(*field_name)) else {
            mismatches.push(ValidationMismatch {
                employee_id,
                employee_name: employee_name.to_string(),
                field_name: (*field_name).to_string(),
                xynergy_value: 0,
                payroll_value: *payroll_value,
                variance_amount: payroll_value.unsigned_abs() as i64,
                status: "MISSING_IN_XYNERGY".to_string(),
                bpjs_metadata: None,
            });
            continue;
        };

        if xy_value != *payroll_value {
            mismatches.push(ValidationMismatch {
                employee_id,
                employee_name: employee_name.to_string(),
                field_name: (*field_name).to_string(),
                xynergy_value: xy_value,
                payroll_value: *payroll_value,
                variance_amount: (xy_value - *payroll_value).abs(),
                status: "DISCREPANCY".to_string(),
                bpjs_metadata: None,
            });
        }
    }

    let Some(risk_tier_value) = xynergy.get("risk_tier") else {
        bpjs_errors += 1;
        mismatches.push(ValidationMismatch {
            employee_id,
            employee_name: employee_name.to_string(),
            field_name: "risk_tier".to_string(),
            xynergy_value: 0,
            payroll_value: 0,
            variance_amount: 0,
            status: "MISSING_RISK_TIER".to_string(),
            bpjs_metadata: None,
        });
        return Ok(bpjs_errors);
    };

    let risk_tier = match value_as_i32_with_default(Some(risk_tier_value), 1, "risk_tier")
        .and_then(|tier| jkk_rate_for_tier(tier).map(|rate| (tier, rate)))
    {
        Ok((tier, rate)) => {
            let mut config = BpjsConfig::default();
            config.ketenagakerjaan_jkk_rate = rate;
            (tier, config)
        }
        Err(_) => {
            bpjs_errors += 1;
            mismatches.push(ValidationMismatch {
                employee_id,
                employee_name: employee_name.to_string(),
                field_name: "risk_tier".to_string(),
                xynergy_value: value_as_i64(Some(risk_tier_value)).unwrap_or(0),
                payroll_value: 0,
                variance_amount: 0,
                status: "INVALID_RISK_TIER".to_string(),
                bpjs_metadata: None,
            });
            return Ok(bpjs_errors);
        }
    };

    let (risk_tier, config) = risk_tier;
    // Payroll-side risk-tier reconciliation is deferred; tier drift still surfaces through BPJS amount variance.
    let payroll_recalculated = calculate_bpjs(&payroll_components(payroll), &config);
    let payroll_expected_kes = bd_to_i64(&payroll_recalculated.kesehatan_employer);
    let payroll_expected_kt = bd_to_i64(&payroll_recalculated.ketenagakerjaan_employer);

    let xynergy_expected = xynergy_components(xynergy).map(|components| {
        let recalculated = calculate_bpjs(&components, &config);
        (
            bd_to_i64(&recalculated.kesehatan_employer),
            bd_to_i64(&recalculated.ketenagakerjaan_employer),
        )
    });

    let bpjs_fields: &[(&str, i64, i64, Option<i64>)] = &[
        (
            "bpjs_kesehatan_employer",
            payroll.bpjs_kesehatan_employer,
            payroll_expected_kes,
            xynergy_expected.map(|(expected, _)| expected),
        ),
        (
            "bpjs_ketenagakerjaan_employer",
            payroll.bpjs_ketenagakerjaan_employer,
            payroll_expected_kt,
            xynergy_expected.map(|(_, expected)| expected),
        ),
    ];

    for (field_name, payroll_value, payroll_expected_value, xynergy_expected_value) in bpjs_fields {
        let Some(xy_value) = value_as_i64(xynergy.get(*field_name)) else {
            if (*payroll_value - *payroll_expected_value).abs() > 1 {
                bpjs_errors += 1;
            }
            mismatches.push(ValidationMismatch {
                employee_id,
                employee_name: employee_name.to_string(),
                field_name: (*field_name).to_string(),
                xynergy_value: 0,
                payroll_value: *payroll_value,
                variance_amount: payroll_value.unsigned_abs() as i64,
                status: "MISSING_IN_XYNERGY".to_string(),
                bpjs_metadata: Some(BpjsMismatchMetadata {
                    risk_tier,
                    recalculated_value: *payroll_expected_value,
                }),
            });
            continue;
        };

        // Tolerance: 1 IDR (matches compliance_report semantics).
        let variance_vs_payroll = (xy_value - *payroll_value).abs();
        let xynergy_variance_vs_expected = xynergy_expected_value
            .map(|expected| (xy_value - expected).abs())
            .unwrap_or(0);
        let payroll_variance_vs_expected = (*payroll_value - *payroll_expected_value).abs();
        let has_bpjs_regulation_error =
            xynergy_variance_vs_expected > 1 || payroll_variance_vs_expected > 1;
        let has_discrepancy = variance_vs_payroll > 1 || has_bpjs_regulation_error;

        if has_discrepancy {
            if has_bpjs_regulation_error {
                bpjs_errors += 1;
            }
            mismatches.push(ValidationMismatch {
                employee_id,
                employee_name: employee_name.to_string(),
                field_name: (*field_name).to_string(),
                xynergy_value: xy_value,
                payroll_value: *payroll_value,
                variance_amount: if has_bpjs_regulation_error {
                    xynergy_variance_vs_expected.max(payroll_variance_vs_expected)
                } else {
                    variance_vs_payroll
                },
                status: if has_bpjs_regulation_error {
                    "BPJS_REGULATION_ERROR".to_string()
                } else {
                    "DISCREPANCY".to_string()
                },
                bpjs_metadata: Some(BpjsMismatchMetadata {
                    risk_tier,
                    recalculated_value: if xynergy_variance_vs_expected > 1 {
                        xynergy_expected_value.unwrap_or(*payroll_expected_value)
                    } else {
                        *payroll_expected_value
                    },
                }),
            });
        }
    }

    Ok(bpjs_errors)
}

/// Generate the validation/reconciliation report.
///
/// Snapshot reconciliation: one comparison per resource against the latest
/// payroll baseline (effective_date DESC, imported_at DESC, batch id DESC)
/// whose effective_date falls in the requested range. CTC selection uses
/// `effective_date BETWEEN start_date AND end_date`, matching the existing
/// `/api/v1/ctc/compliance-report` rule so the two reports remain
/// comparable. Sampling is opt-in via `filters.employee_ids`.
pub async fn generate_validation_report(
    tx: &mut Transaction<'_, Postgres>,
    filters: ValidationReportFilters,
) -> Result<ValidationReport> {
    if filters.end_date < filters.start_date {
        return Err(AppError::Validation(
            "end_date must be greater than or equal to start_date".to_string(),
        ));
    }

    if let Some(ids) = filters.employee_ids.as_ref() {
        if ids.is_empty() {
            return Err(AppError::Validation(
                "employee_ids must contain at least one resource id".to_string(),
            ));
        }
        if ids.len() > MAX_SAMPLED_EMPLOYEE_IDS {
            return Err(AppError::Validation(format!(
                "employee_ids may contain at most {} ids per request",
                MAX_SAMPLED_EMPLOYEE_IDS
            )));
        }
    }

    let employee_ids_slice = filters.employee_ids.as_deref();
    let sampled = filters.employee_ids.is_some();

    validate_payroll_source(tx, filters.start_date, filters.end_date, employee_ids_slice).await?;

    let payroll_coverage_pct =
        compute_payroll_coverage_pct(tx, filters.start_date, filters.end_date, employee_ids_slice)
            .await?;

    let payroll_rows =
        load_payroll_rows(tx, filters.start_date, filters.end_date, employee_ids_slice).await?;
    let ctc_rows =
        load_ctc_rows(tx, filters.start_date, filters.end_date, employee_ids_slice).await?;

    // Index payroll by resource_id - keep most recent effective_date per resource within range.
    let mut payroll_by_resource: std::collections::HashMap<Uuid, PayrollRow> =
        std::collections::HashMap::new();
    for row in payroll_rows {
        payroll_by_resource
            .entry(row.resource_id)
            .and_modify(|existing| {
                if row.effective_date > existing.effective_date {
                    *existing = PayrollRow {
                        resource_id: row.resource_id,
                        employee_name: row.employee_name.clone(),
                        effective_date: row.effective_date,
                        imported_at: row.imported_at,
                        base_salary: row.base_salary,
                        hra_allowance: row.hra_allowance,
                        medical_allowance: row.medical_allowance,
                        transport_allowance: row.transport_allowance,
                        meal_allowance: row.meal_allowance,
                        bpjs_kesehatan_employer: row.bpjs_kesehatan_employer,
                        bpjs_ketenagakerjaan_employer: row.bpjs_ketenagakerjaan_employer,
                    };
                }
            })
            .or_insert(row);
    }
    let selected_payroll_rows: Vec<&PayrollRow> = payroll_by_resource.values().collect();
    selected_payroll_freshness(filters.end_date, &selected_payroll_rows)?;

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());

    let mut mismatches: Vec<ValidationMismatch> = Vec::new();
    let mut excluded: Vec<ExcludedRecord> = Vec::new();
    let mut total_compared: i64 = 0;
    let mut total_matches: i64 = 0;
    let mut bpjs_error_count: i64 = 0;

    let mut ctc_resource_ids: std::collections::HashSet<Uuid> = std::collections::HashSet::new();

    for ctc_row in &ctc_rows {
        ctc_resource_ids.insert(ctc_row.resource_id);

        let Some(payroll) = payroll_by_resource.get(&ctc_row.resource_id) else {
            excluded.push(ExcludedRecord {
                employee_id: ctc_row.resource_id,
                employee_name: ctc_row.name.clone(),
                reason: "No payroll record in range".to_string(),
            });
            continue;
        };

        let Some(encrypted_payload) = ctc_row.encrypted_payload.as_ref() else {
            excluded.push(ExcludedRecord {
                employee_id: ctc_row.resource_id,
                employee_name: ctc_row.name.clone(),
                reason: "CTC encryption metadata missing".to_string(),
            });
            continue;
        };

        let decrypted = match crypto_svc.decrypt_components(encrypted_payload).await {
            Ok(d) => d,
            Err(_) => {
                excluded.push(ExcludedRecord {
                    employee_id: ctc_row.resource_id,
                    employee_name: ctc_row.name.clone(),
                    reason: "CTC decryption failed".to_string(),
                });
                continue;
            }
        };

        let before_count = mismatches.len();
        let errors = match compare_record(
            ctc_row.resource_id,
            &ctc_row.name,
            &decrypted,
            payroll,
            &mut mismatches,
        ) {
            Ok(errors) => errors,
            Err(_) => {
                excluded.push(ExcludedRecord {
                    employee_id: ctc_row.resource_id,
                    employee_name: ctc_row.name.clone(),
                    reason: "Invalid BPJS risk tier for validation".to_string(),
                });
                continue;
            }
        };
        total_compared += 1;
        bpjs_error_count += errors;

        if mismatches.len() == before_count {
            total_matches += 1;
        }
    }

    for payroll in payroll_by_resource.values() {
        if !ctc_resource_ids.contains(&payroll.resource_id) {
            excluded.push(ExcludedRecord {
                employee_id: payroll.resource_id,
                employee_name: payroll.employee_name.clone(),
                reason: "No active Xynergy CTC record in range".to_string(),
            });
        }
    }

    // Deterministic mismatch ordering: employee name ASC, field name ASC.
    mismatches.sort_by(|a, b| {
        a.employee_name
            .cmp(&b.employee_name)
            .then_with(|| a.field_name.cmp(&b.field_name))
            .then_with(|| a.employee_id.cmp(&b.employee_id))
    });
    excluded.sort_by(|a, b| {
        a.employee_name
            .cmp(&b.employee_name)
            .then_with(|| a.employee_id.cmp(&b.employee_id))
            .then_with(|| a.reason.cmp(&b.reason))
    });

    let total_discrepancies = i64::try_from(mismatches.len()).unwrap_or(0);
    let excluded_count = i64::try_from(excluded.len()).unwrap_or(0);
    let rate = match_rate(total_matches, total_compared);

    // Apply bounded pagination to mismatches only - summary stays stable.
    let offset = filters.offset.unwrap_or(0).max(0) as usize;
    let limit = filters
        .limit
        .unwrap_or(DEFAULT_MISMATCH_LIMIT)
        .clamp(0, MAX_MISMATCH_LIMIT) as usize;
    let paged_mismatches = mismatches.into_iter().skip(offset).take(limit).collect();
    let capped_excluded = excluded.into_iter().take(MAX_EXCLUDED_ROWS).collect();

    Ok(ValidationReport {
        start_date: filters.start_date,
        end_date: filters.end_date,
        total_compared,
        total_matches,
        total_discrepancies,
        match_rate_pct: rate,
        excluded_count,
        bpjs_error_count,
        sampled,
        payroll_coverage_pct,
        mismatches: paged_mismatches,
        excluded: capped_excluded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_rate_zero_total() {
        assert_eq!(match_rate(0, 0), 0.0);
    }

    #[test]
    fn match_rate_perfect() {
        assert_eq!(match_rate(10, 10), 100.0);
    }

    #[test]
    fn match_rate_partial() {
        assert_eq!(match_rate(3, 4), 75.0);
    }

    #[test]
    fn jkk_rate_invalid_tier_rejected() {
        assert!(jkk_rate_for_tier(5).is_err());
    }

    fn payroll_fixture(resource_id: Uuid) -> PayrollRow {
        PayrollRow {
            resource_id,
            employee_name: "Unit Test Employee".to_string(),
            effective_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            imported_at: Utc::now(),
            base_salary: 15_000_000,
            hra_allowance: 3_000_000,
            medical_allowance: 1_000_000,
            transport_allowance: 500_000,
            meal_allowance: 500_000,
            bpjs_kesehatan_employer: 480_000,
            bpjs_ketenagakerjaan_employer: 1_058_948,
        }
    }

    fn xynergy_fixture(base: i64) -> serde_json::Value {
        serde_json::json!({
            "base_salary": base,
            "hra_allowance": 3_000_000,
            "medical_allowance": 1_000_000,
            "transport_allowance": 500_000,
            "meal_allowance": 500_000,
            "bpjs_kesehatan_employer": 480_000,
            "bpjs_ketenagakerjaan_employer": 1_058_948,
            "risk_tier": 1,
        })
    }

    #[test]
    fn compare_record_emits_discrepancy_for_plain_field() {
        let emp = Uuid::new_v4();
        let payroll = payroll_fixture(emp);
        // Xynergy says base_salary=14M, payroll says 15M → variance 1M.
        // Note: shifting base_salary necessarily shifts BPJS basis, so a BPJS
        // regulation cascade is expected here too. This test scopes its
        // assertions to the plain-field row only.
        let xynergy = xynergy_fixture(14_000_000);

        let mut out = Vec::new();
        let _ = compare_record(emp, "Unit Test Employee", &xynergy, &payroll, &mut out)
            .expect("compare_record should succeed");

        let base_row = out
            .iter()
            .find(|m| m.field_name == "base_salary")
            .expect("expected base_salary mismatch row");

        assert_eq!(base_row.employee_id, emp);
        assert_eq!(base_row.employee_name, "Unit Test Employee");
        assert_eq!(base_row.xynergy_value, 14_000_000);
        assert_eq!(base_row.payroll_value, 15_000_000);
        assert_eq!(base_row.variance_amount, 1_000_000);
        assert_eq!(base_row.status, "DISCREPANCY");
        assert!(
            base_row.bpjs_metadata.is_none(),
            "plain-field rows must not carry bpjs_metadata"
        );

        // Non-BPJS plain fields beyond base_salary must NOT be flagged since
        // hra/medical/transport/meal match between xynergy and payroll.
        for field in [
            "hra_allowance",
            "medical_allowance",
            "transport_allowance",
            "meal_allowance",
        ] {
            assert!(
                !out.iter().any(|m| m.field_name == field),
                "field `{}` should not be flagged when values agree",
                field
            );
        }
    }

    #[test]
    fn compare_record_flags_bpjs_regulation_error_with_metadata() {
        let emp = Uuid::new_v4();
        // Payroll BPJS values diverge from both stored Xynergy value and regulation formula.
        let mut payroll = payroll_fixture(emp);
        payroll.bpjs_kesehatan_employer = 100_000;
        payroll.bpjs_ketenagakerjaan_employer = 200_000;

        // Xynergy also has wrong BPJS values (different from payroll AND from regulation).
        let xynergy = serde_json::json!({
            "base_salary": 15_000_000,
            "hra_allowance": 3_000_000,
            "medical_allowance": 1_000_000,
            "transport_allowance": 500_000,
            "meal_allowance": 500_000,
            "bpjs_kesehatan_employer": 50_000,
            "bpjs_ketenagakerjaan_employer": 90_000,
            "risk_tier": 1,
        });

        let mut out = Vec::new();
        let bpjs_errors = compare_record(emp, "BPJS Unit Employee", &xynergy, &payroll, &mut out)
            .expect("compare_record should succeed");

        assert!(
            bpjs_errors >= 1,
            "expected at least one BPJS regulation error"
        );

        let regulation_rows: Vec<&ValidationMismatch> = out
            .iter()
            .filter(|m| m.status == "BPJS_REGULATION_ERROR")
            .collect();
        assert!(
            !regulation_rows.is_empty(),
            "expected at least one BPJS_REGULATION_ERROR row, got {:?}",
            out
        );
        for row in regulation_rows {
            let meta = row
                .bpjs_metadata
                .as_ref()
                .expect("BPJS_REGULATION_ERROR rows must carry bpjs_metadata");
            assert_eq!(meta.risk_tier, 1);
            assert!(meta.recalculated_value > 0);
        }
    }

    #[test]
    fn compare_record_uses_payroll_basis_for_payroll_bpjs_regulation() {
        let emp = Uuid::new_v4();
        let mut payroll = payroll_fixture(emp);
        payroll.base_salary = 14_000_000;
        payroll.bpjs_ketenagakerjaan_employer = 1_016_548;

        let xynergy = xynergy_fixture(15_000_000);

        let mut out = Vec::new();
        let bpjs_errors =
            compare_record(emp, "Payroll Basis Employee", &xynergy, &payroll, &mut out)
                .expect("compare_record should succeed");

        assert_eq!(
            bpjs_errors, 0,
            "payroll BPJS should be regulation-valid against payroll's own salary basis"
        );
        assert!(
            !out.iter().any(|m| m.status == "BPJS_REGULATION_ERROR"),
            "different Xynergy/payroll bases should not by itself become a BPJS regulation error"
        );
        assert!(out.iter().any(|m| {
            m.field_name == "bpjs_ketenagakerjaan_employer" && m.status == "DISCREPANCY"
        }));
    }

    #[test]
    fn invalid_risk_tier_still_preserves_plain_field_mismatches() {
        let emp = Uuid::new_v4();
        let payroll = payroll_fixture(emp);
        let mut xynergy = xynergy_fixture(14_000_000);
        xynergy["risk_tier"] = serde_json::json!(9);

        let mut out = Vec::new();
        let bpjs_errors =
            compare_record(emp, "Invalid Tier Employee", &xynergy, &payroll, &mut out)
                .expect("compare_record should succeed");

        assert_eq!(bpjs_errors, 1);
        assert!(
            out.iter().any(|m| m.field_name == "base_salary"),
            "plain-field mismatch should not be hidden by invalid BPJS risk tier"
        );
        assert!(out
            .iter()
            .any(|m| { m.field_name == "risk_tier" && m.status == "INVALID_RISK_TIER" }));
    }

    #[test]
    fn missing_xynergy_bpjs_field_counts_regulation_error() {
        let emp = Uuid::new_v4();
        let mut payroll = payroll_fixture(emp);
        payroll.bpjs_kesehatan_employer = 100_000;

        let xynergy = serde_json::json!({
            "base_salary": 15_000_000,
            "hra_allowance": 3_000_000,
            "medical_allowance": 1_000_000,
            "transport_allowance": 500_000,
            "meal_allowance": 500_000,
            "bpjs_ketenagakerjaan_employer": 1_058_948,
            "risk_tier": 1,
        });

        let mut out = Vec::new();
        let bpjs_errors = compare_record(emp, "Missing BPJS Field", &xynergy, &payroll, &mut out)
            .expect("compare_record should succeed");

        assert_eq!(
            bpjs_errors, 1,
            "missing stored BPJS value should still count payroll regulation failure"
        );
        assert!(out.iter().any(|m| {
            m.field_name == "bpjs_kesehatan_employer" && m.status == "MISSING_IN_XYNERGY"
        }));
    }
}
