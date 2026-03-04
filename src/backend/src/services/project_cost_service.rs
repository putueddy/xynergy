use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::types::BigDecimal;
use sqlx::{PgPool, Row};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::budget_service::{
    bigdecimal_to_i64_trunc, extract_daily_rate_from_allocation_row, parse_json_decimal,
};
use crate::services::cost_preview::{calculate_cost_preview, count_working_days};
use crate::services::ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService, EncryptedPayload};
use crate::services::key_provider::EnvKeyProvider;

// ── Result types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProjectResourceCostResult {
    pub project_id: Uuid,
    pub total_resource_cost_idr: i64,
    pub employees: Vec<EmployeeCostEntry>,
    pub monthly_breakdown: Vec<MonthlyCostEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmployeeCostEntry {
    pub resource_id: Uuid,
    pub resource_name: String,
    pub daily_rate_idr: Option<i64>,
    pub days_allocated: i32,
    pub allocation_percentage: f64,
    pub total_cost_idr: i64,
    pub has_rate_change: bool,
    pub rate_change_note: Option<String>,
    pub missing_rate: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonthlyCostEntry {
    pub month: String,
    pub working_days: i32,
    pub cost_idr: i64,
}

// ── Internal accumulators ────────────────────────────────────────────────────

struct EmployeeAccumulator {
    resource_id: Uuid,
    resource_name: String,
    daily_rate_idr: Option<i64>,
    days_allocated: i32,
    /// Weighted average allocation percentage across all allocation rows.
    total_cost_idr: i64,
    allocation_pct_weighted_sum: f64,
    allocation_pct_weight_days: i32,
    has_rate_change: bool,
    rate_change_note: Option<String>,
    missing_rate: bool,
}

/// A single rate window derived from CTC revisions.
struct RateWindow {
    daily_rate_idr: i64,
    effective_from: NaiveDate,
    effective_until: NaiveDate,
}

// ── Load holidays from pool (non-transaction variant) ────────────────────────

async fn load_holidays_from_pool(pool: &PgPool) -> Result<Vec<NaiveDate>> {
    use crate::services::cost_preview::is_weekend;

    let holiday_rows = sqlx::query("SELECT date::TEXT as date FROM holidays")
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut holidays = Vec::new();
    for row in holiday_rows {
        let date_str: String = row
            .try_get("date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let parsed = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|_| AppError::Internal("Failed to parse holiday date".to_string()))?;
        if !is_weekend(parsed) {
            holidays.push(parsed);
        }
    }

    Ok(holidays)
}

// ── CTC revision rate extraction ─────────────────────────────────────────────

/// Extract daily_rate_idr from a ctc_revisions row.
/// Try `encrypted_daily_rate` first; if NULL, decrypt `encrypted_components` and
/// extract the `daily_rate` field from the JSON blob.
async fn extract_rate_from_revision_row(
    row: &sqlx::postgres::PgRow,
    crypto_svc: &DefaultCtcCryptoService<EnvKeyProvider>,
) -> Result<Option<i64>> {
    let encrypted_daily_rate: Option<String> = row
        .try_get("encrypted_daily_rate")
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
    let encrypted_at: DateTime<Utc> = row
        .try_get("encrypted_at")
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Try encrypted_daily_rate first
    if let Some(ciphertext) = encrypted_daily_rate {
        let payload = EncryptedPayload {
            ciphertext,
            key_version: key_version.clone(),
            encryption_version: encryption_version.clone(),
            algorithm: algorithm.clone(),
            encrypted_at,
        };
        let decrypted = crypto_svc.decrypt_components(&payload).await?;
        if let Some(daily_rate_value) = decrypted.get("daily_rate") {
            let daily_rate_bd = parse_json_decimal(daily_rate_value, "daily_rate")?;
            return Ok(Some(bigdecimal_to_i64_trunc(&daily_rate_bd)?));
        }
    }

    // Fallback: decrypt encrypted_components and extract daily_rate
    let encrypted_components: String = row
        .try_get("encrypted_components")
        .map_err(|e| AppError::Database(e.to_string()))?;

    let payload = EncryptedPayload {
        ciphertext: encrypted_components,
        key_version,
        encryption_version,
        algorithm,
        encrypted_at,
    };
    let decrypted = crypto_svc.decrypt_components(&payload).await?;
    if let Some(daily_rate_value) = decrypted.get("daily_rate") {
        let daily_rate_bd = parse_json_decimal(daily_rate_value, "daily_rate")?;
        return Ok(Some(bigdecimal_to_i64_trunc(&daily_rate_bd)?));
    }

    Ok(None)
}

// ── Build rate windows from CTC revisions ────────────────────────────────────

async fn fetch_revision_points(
    pool: &PgPool,
    resource_id: Uuid,
    crypto_svc: &DefaultCtcCryptoService<EnvKeyProvider>,
) -> Result<Vec<(NaiveDate, i64)>> {
    let revision_rows = sqlx::query(
        "SELECT revision_number, effective_date, encrypted_daily_rate,
                encrypted_components, key_version, encryption_version,
                encryption_algorithm, encrypted_at
         FROM ctc_revisions
         WHERE resource_id = $1
         ORDER BY effective_date ASC",
    )
    .bind(resource_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if revision_rows.is_empty() {
        return Ok(Vec::new());
    }

    let mut points: Vec<(NaiveDate, i64)> = Vec::new();
    for row in &revision_rows {
        let effective_date: NaiveDate = row
            .try_get("effective_date")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rate = extract_rate_from_revision_row(row, crypto_svc).await?;
        if let Some(rate_idr) = rate {
            points.push((effective_date, rate_idr));
        }
    }

    Ok(points)
}

fn build_rate_windows_from_points(
    points: &[(NaiveDate, i64)],
    alloc_start: NaiveDate,
    alloc_end: NaiveDate,
) -> Vec<RateWindow> {
    if points.is_empty() || alloc_start > alloc_end {
        return vec![];
    }

    let mut start_idx = 0usize;
    for (idx, (eff_date, _)) in points.iter().enumerate() {
        if *eff_date <= alloc_start {
            start_idx = idx;
        } else {
            break;
        }
    }

    let mut windows = Vec::new();
    for i in start_idx..points.len() {
        let (eff_date, rate_idr) = points[i];
        let window_start = if i == start_idx {
            alloc_start
        } else {
            std::cmp::max(eff_date, alloc_start)
        };

        let window_end = if i + 1 < points.len() {
            points[i + 1]
                .0
                .pred_opt()
                .unwrap_or(points[i + 1].0)
        } else {
            alloc_end
        };

        let clamped_end = std::cmp::min(window_end, alloc_end);

        if window_start <= clamped_end {
            windows.push(RateWindow {
                daily_rate_idr: rate_idr,
                effective_from: window_start,
                effective_until: clamped_end,
            });
        }

        if clamped_end >= alloc_end {
            break;
        }
    }

    windows
}

// ── Main entry point ─────────────────────────────────────────────────────────

pub async fn compute_project_resource_costs(
    pool: &PgPool,
    project_id: Uuid,
) -> Result<ProjectResourceCostResult> {
    let holidays = load_holidays_from_pool(pool).await?;
    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let mut revision_points_cache: HashMap<Uuid, Vec<(NaiveDate, i64)>> = HashMap::new();

    // Query all allocations for this project with latest CTC rate from ctc_records
    let allocation_rows = sqlx::query(
        "SELECT a.id AS allocation_id, a.resource_id, a.start_date, a.end_date,
                a.allocation_percentage, a.include_weekend,
                r.name AS resource_name,
                c.daily_rate, c.encrypted_daily_rate, c.key_version,
                c.encryption_version, c.encryption_algorithm, c.encrypted_at
         FROM allocations a
         JOIN resources r ON r.id = a.resource_id
         LEFT JOIN LATERAL (
            SELECT daily_rate, encrypted_daily_rate, key_version,
                   encryption_version, encryption_algorithm, encrypted_at
            FROM ctc_records c
            WHERE c.resource_id = a.resource_id AND c.status = 'Active'
            ORDER BY c.effective_date DESC, c.updated_at DESC
            LIMIT 1
         ) c ON TRUE
         WHERE a.project_id = $1",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut employee_map: HashMap<Uuid, EmployeeAccumulator> = HashMap::new();
    let mut monthly_map: BTreeMap<String, (i32, i64)> = BTreeMap::new();

    for row in &allocation_rows {
        let resource_id: Uuid = row
            .try_get("resource_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let resource_name: String = row
            .try_get("resource_name")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let alloc_start: NaiveDate = row
            .try_get("start_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let alloc_end: NaiveDate = row
            .try_get("end_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let include_weekend: bool = row
            .try_get("include_weekend")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let allocation_percentage_bd: BigDecimal = row
            .try_get("allocation_percentage")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let allocation_percentage = allocation_percentage_bd
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0);

        // Try to get rate from ctc_records (single-rate path)
        let single_rate =
            extract_daily_rate_from_allocation_row(row, &crypto_svc).await?;

        if !revision_points_cache.contains_key(&resource_id) {
            let points = fetch_revision_points(pool, resource_id, &crypto_svc).await?;
            revision_points_cache.insert(resource_id, points);
        }

        let rate_windows = build_rate_windows_from_points(
            revision_points_cache
                .get(&resource_id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
            alloc_start,
            alloc_end,
        );

        let has_multiple_rates = rate_windows.len() > 1;

        let entry = employee_map
            .entry(resource_id)
            .or_insert_with(|| EmployeeAccumulator {
                resource_id,
                resource_name: resource_name.clone(),
                daily_rate_idr: None,
                days_allocated: 0,
                total_cost_idr: 0,
                allocation_pct_weighted_sum: 0.0,
                allocation_pct_weight_days: 0,
                has_rate_change: false,
                rate_change_note: None,
                missing_rate: false,
            });

        if !rate_windows.is_empty() {
            // Use rate windows (supports mid-period rate changes)
            if has_multiple_rates {
                entry.has_rate_change = true;
                let rates: Vec<String> = rate_windows
                    .iter()
                    .map(|w| {
                        format!(
                            "IDR {} ({} to {})",
                            w.daily_rate_idr, w.effective_from, w.effective_until
                        )
                    })
                    .collect();
                entry.rate_change_note = Some(format!("Rate changed during allocation: {}", rates.join(", ")));
            }

            // Set daily_rate_idr to the latest rate window
            entry.daily_rate_idr = rate_windows.last().map(|w| w.daily_rate_idr);

            for window in &rate_windows {
                let preview = calculate_cost_preview(
                    window.daily_rate_idr,
                    window.effective_from,
                    window.effective_until,
                    allocation_percentage,
                    include_weekend,
                    &holidays,
                );

                entry.days_allocated += preview.working_days;
                entry.total_cost_idr += preview.total_cost_idr;
                entry.allocation_pct_weighted_sum +=
                    allocation_percentage * f64::from(preview.working_days);
                entry.allocation_pct_weight_days += preview.working_days;

                for bucket in &preview.monthly_breakdown {
                    let m = monthly_map.entry(bucket.month.clone()).or_insert((0, 0));
                    m.0 += bucket.working_days;
                    m.1 += bucket.cost_idr;
                }
            }
        } else if let Some(rate) = single_rate {
            // Fallback: single rate from ctc_records
            entry.daily_rate_idr = Some(rate);

            let preview = calculate_cost_preview(
                rate,
                alloc_start,
                alloc_end,
                allocation_percentage,
                include_weekend,
                &holidays,
            );

            entry.days_allocated += preview.working_days;
            entry.total_cost_idr += preview.total_cost_idr;
            entry.allocation_pct_weighted_sum +=
                allocation_percentage * f64::from(preview.working_days);
            entry.allocation_pct_weight_days += preview.working_days;

            for bucket in &preview.monthly_breakdown {
                let m = monthly_map.entry(bucket.month.clone()).or_insert((0, 0));
                m.0 += bucket.working_days;
                m.1 += bucket.cost_idr;
            }
        } else {
            entry.missing_rate = true;
            let working_days =
                count_working_days(alloc_start, alloc_end, include_weekend, &holidays);
            entry.days_allocated += working_days;
            entry.allocation_pct_weighted_sum +=
                allocation_percentage * f64::from(working_days);
            entry.allocation_pct_weight_days += working_days;
        }
    }

    // Build result
    let mut employees: Vec<EmployeeCostEntry> = employee_map
        .into_values()
        .map(|acc| EmployeeCostEntry {
            resource_id: acc.resource_id,
            resource_name: acc.resource_name,
            daily_rate_idr: acc.daily_rate_idr,
            days_allocated: acc.days_allocated,
            allocation_percentage: if acc.allocation_pct_weight_days > 0 {
                acc.allocation_pct_weighted_sum / f64::from(acc.allocation_pct_weight_days)
            } else {
                0.0
            },
            total_cost_idr: acc.total_cost_idr,
            has_rate_change: acc.has_rate_change,
            rate_change_note: acc.rate_change_note,
            missing_rate: acc.missing_rate,
        })
        .collect();
    employees.sort_by(|a, b| b.total_cost_idr.cmp(&a.total_cost_idr));

    let monthly_breakdown: Vec<MonthlyCostEntry> = monthly_map
        .into_iter()
        .map(|(month, (working_days, cost_idr))| MonthlyCostEntry {
            month,
            working_days,
            cost_idr,
        })
        .collect();

    let total_resource_cost_idr = employees.iter().map(|e| e.total_cost_idr).sum();

    Ok(ProjectResourceCostResult {
        project_id,
        total_resource_cost_idr,
        employees,
        monthly_breakdown,
    })
}
