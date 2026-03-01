use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, put},
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{types::BigDecimal, PgPool, Row};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::{
    audit_log::user_claims_from_headers, audit_payload, log_audit, user_id_from_headers,
    cost_preview::{calculate_cost_preview, is_weekend, MonthlyBucket},
    ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService, EncryptedPayload},
    key_provider::EnvKeyProvider,
};

fn required_uuid(value: Option<Uuid>, field: &str) -> Result<Uuid> {
    value.ok_or_else(|| AppError::Internal(format!("{} is unexpectedly null", field)))
}

async fn ensure_allocation_access(
    pool: &PgPool,
    headers: &HeaderMap,
    action: &str,
    entity_id: Uuid,
    project_id: Option<Uuid>,
) -> Result<(String, Uuid)> {
    let claims = user_claims_from_headers(headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    let can_manage = matches!(
        claims.role.as_str(),
        "admin" | "department_head" | "project_manager"
    );
    if !can_manage {
        log_audit(
            pool,
            Some(user_id),
            "ACCESS_DENIED",
            "allocation",
            entity_id,
            serde_json::json!({
                "reason": "insufficient_permissions",
                "attempted_role": claims.role,
                "action": action,
            }),
        )
        .await
        .ok();
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    if claims.role == "project_manager" {
        let pid = project_id.ok_or_else(|| {
            AppError::Validation("project_id is required for project manager actions".to_string())
        })?;
        let pm_id =
            sqlx::query_scalar!("SELECT project_manager_id FROM projects WHERE id = $1", pid)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?
                .flatten();
        if pm_id != Some(user_id) {
            log_audit(
                pool,
                Some(user_id),
                "ACCESS_DENIED",
                "allocation",
                entity_id,
                serde_json::json!({
                    "reason": "not_project_manager",
                    "attempted_role": claims.role,
                    "project_id": pid,
                    "action": action,
                }),
            )
            .await
            .ok();
            return Err(AppError::Forbidden("Insufficient permissions".to_string()));
        }
    }

    Ok((claims.role, user_id))
}

/// Allocation response structure
#[derive(Debug, Serialize)]
pub struct AllocationResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub resource_id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub allocation_percentage: f64,
    pub include_weekend: bool,
    pub project_name: String,
    pub resource_name: String,
}

/// Create allocation request
#[derive(Debug, Deserialize)]
pub struct CreateAllocationRequest {
    pub project_id: Uuid,
    pub resource_id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub allocation_percentage: f64,
    pub include_weekend: bool,
}

/// Update allocation request
#[derive(Debug, Deserialize)]
pub struct UpdateAllocationRequest {
    pub project_id: Option<Uuid>,
    pub resource_id: Option<Uuid>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub allocation_percentage: Option<f64>,
    pub include_weekend: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CostPreviewQuery {
    pub resource_id: Uuid,
    pub project_id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub allocation_percentage: f64,
    pub include_weekend: bool,
}

#[derive(Debug, Serialize)]
pub struct BudgetImpact {
    pub department_budget_total_idr: i64,
    pub current_committed_idr: i64,
    pub projected_committed_idr: i64,
    pub remaining_after_assignment_idr: i64,
    pub utilization_percentage: f64,
    pub budget_health: String,
}

#[derive(Debug, Serialize)]
pub struct CostPreviewResponse {
    pub daily_rate_idr: i64,
    pub working_days: i32,
    pub allocation_percentage: f64,
    pub total_cost_idr: i64,
    pub monthly_breakdown: Vec<MonthlyBucket>,
    pub budget_impact: Option<BudgetImpact>,
    pub warning: Option<String>,
    pub requires_approval: bool,
}

/// Daily allocation info for validation
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DailyAllocation {
    date: NaiveDate,
    allocated_hours: f64,
    assignments: Vec<AssignmentInfo>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AssignmentInfo {
    allocation_id: Uuid,
    project_id: Uuid,
    hours: f64,
}

/// Convert BigDecimal to f64
fn bigdecimal_to_f64(bd: sqlx::types::BigDecimal) -> f64 {
    bd.to_string().parse().unwrap_or(0.0)
}

/// Convert f64 to BigDecimal
fn f64_to_bigdecimal(f: f64) -> sqlx::types::BigDecimal {
    sqlx::types::BigDecimal::try_from(f).unwrap_or_default()
}

fn bigdecimal_to_i64_trunc(value: &BigDecimal) -> Result<i64> {
    value
        .to_string()
        .split('.')
        .next()
        .unwrap_or("0")
        .parse::<i64>()
        .map_err(|_| AppError::Internal("Failed to convert decimal to i64".to_string()))
}

fn parse_json_decimal(value: &serde_json::Value, field: &str) -> Result<BigDecimal> {
    match value {
        serde_json::Value::String(s) => s.parse::<BigDecimal>().map_err(|_| {
            AppError::Internal(format!("Failed to parse '{}' from decrypted payload", field))
        }),
        serde_json::Value::Number(n) => n.to_string().parse::<BigDecimal>().map_err(|_| {
            AppError::Internal(format!("Failed to parse '{}' from decrypted payload", field))
        }),
        _ => Err(AppError::Internal(format!(
            "Invalid '{}' value in decrypted payload",
            field
        ))),
    }
}

async fn get_active_ctc_daily_rate(pool: &PgPool, resource_id: Uuid) -> Result<i64> {
    let row = sqlx::query(
        "SELECT daily_rate, encrypted_daily_rate, key_version, encryption_version, encryption_algorithm, encrypted_at
         FROM ctc_records
         WHERE resource_id = $1 AND status = 'Active'
         ORDER BY effective_date DESC, updated_at DESC
         LIMIT 1",
    )
    .bind(resource_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| {
        AppError::Validation(
            "Cannot assign resource without CTC data. Contact HR to complete CTC entry for this employee."
                .to_string(),
        )
    })?;

    let encrypted_daily_rate: Option<String> = row
        .try_get("encrypted_daily_rate")
        .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some(ciphertext) = encrypted_daily_rate {
        let key_version: String = row
            .try_get("key_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encryption_version: String = row
            .try_get("encryption_version")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let algorithm: String = row
            .try_get("encryption_algorithm")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let encrypted_at: Option<DateTime<Utc>> = row
            .try_get("encrypted_at")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let payload = EncryptedPayload {
            ciphertext,
            key_version,
            encryption_version,
            algorithm,
            encrypted_at: encrypted_at.unwrap_or_else(Utc::now),
        };

        let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
        let decrypted = crypto_svc.decrypt_components(&payload).await?;
        let daily_rate_value = decrypted.get("daily_rate").ok_or_else(|| {
            AppError::Internal("Missing daily_rate in decrypted CTC payload".to_string())
        })?;
        let daily_rate_bd = parse_json_decimal(daily_rate_value, "daily_rate")?;
        return bigdecimal_to_i64_trunc(&daily_rate_bd);
    }

    let plaintext_daily_rate: Option<BigDecimal> = row
        .try_get("daily_rate")
        .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some(daily_rate) = plaintext_daily_rate {
        return bigdecimal_to_i64_trunc(&daily_rate);
    }

    Err(AppError::Validation(
        "Cannot assign resource without CTC data. Contact HR to complete CTC entry for this employee."
            .to_string(),
    ))
}

#[derive(Debug)]
struct BudgetComputation {
    budget_impact: Option<BudgetImpact>,
    warning: Option<String>,
    requires_approval: bool,
}

fn format_idr_millions(value: i64) -> String {
    format!("{:.1}", value as f64 / 1_000_000.0)
}

async fn extract_daily_rate_from_allocation_row(row: &sqlx::postgres::PgRow, crypto_svc: &DefaultCtcCryptoService<EnvKeyProvider>) -> Result<Option<i64>> {
    let encrypted_daily_rate: Option<String> = row
        .try_get("encrypted_daily_rate")
        .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some(ciphertext) = encrypted_daily_rate {
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

        if let (Some(key_version), Some(encryption_version), Some(algorithm)) =
            (key_version, encryption_version, algorithm)
        {
            let payload = EncryptedPayload {
                ciphertext,
                key_version,
                encryption_version,
                algorithm,
                encrypted_at: encrypted_at.unwrap_or_else(Utc::now),
            };
            let decrypted = crypto_svc.decrypt_components(&payload).await?;
            if let Some(daily_rate_value) = decrypted.get("daily_rate") {
                let daily_rate_bd = parse_json_decimal(daily_rate_value, "daily_rate")?;
                return Ok(Some(bigdecimal_to_i64_trunc(&daily_rate_bd)?));
            }
        }
    }

    let plaintext_daily_rate: Option<BigDecimal> = row
        .try_get("daily_rate")
        .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some(daily_rate) = plaintext_daily_rate {
        return Ok(Some(bigdecimal_to_i64_trunc(&daily_rate)?));
    }

    Ok(None)
}

async fn compute_budget_impact(
    pool: &PgPool,
    headers: &HeaderMap,
    resource_id: Uuid,
    project_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
    monthly_breakdown: &[MonthlyBucket],
    holidays: &[NaiveDate],
) -> Result<BudgetComputation> {
    let department_id: Option<Uuid> =
        sqlx::query_scalar("SELECT department_id FROM resources WHERE id = $1")
            .bind(resource_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .flatten();

    let Some(department_id) = department_id else {
        return Ok(BudgetComputation {
            budget_impact: None,
            warning: None,
            requires_approval: false,
        });
    };

    let budget_periods = monthly_breakdown
        .iter()
        .map(|bucket| bucket.month.clone())
        .collect::<Vec<_>>();

    if budget_periods.is_empty() {
        return Ok(BudgetComputation {
            budget_impact: None,
            warning: None,
            requires_approval: false,
        });
    }

    let budget_rows = sqlx::query(
        "SELECT budget_period, total_budget_idr
         FROM department_budgets
         WHERE department_id = $1
           AND budget_period = ANY($2)",
    )
    .bind(department_id)
    .bind(&budget_periods)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if budget_rows.len() != budget_periods.len() {
        return Ok(BudgetComputation {
            budget_impact: None,
            warning: None,
            requires_approval: false,
        });
    }

    let mut budget_by_period: HashMap<String, i64> = HashMap::new();
    for row in budget_rows {
        let period: String = row
            .try_get("budget_period")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let total: i64 = row
            .try_get("total_budget_idr")
            .map_err(|e| AppError::Database(e.to_string()))?;
        budget_by_period.insert(period, total);
    }

    let department_budget_total_idr = budget_periods
        .iter()
        .map(|period| budget_by_period.get(period).copied().unwrap_or(0))
        .sum::<i64>();

    let allocation_rows = sqlx::query(
        "SELECT a.resource_id, a.start_date, a.end_date, a.allocation_percentage, a.include_weekend,
                c.daily_rate, c.encrypted_daily_rate, c.key_version, c.encryption_version,
                c.encryption_algorithm, c.encrypted_at
         FROM allocations a
         JOIN resources r ON r.id = a.resource_id
         LEFT JOIN LATERAL (
            SELECT daily_rate, encrypted_daily_rate, key_version, encryption_version,
                   encryption_algorithm, encrypted_at
            FROM ctc_records c
            WHERE c.resource_id = a.resource_id AND c.status = 'Active'
            ORDER BY c.effective_date DESC, c.updated_at DESC
            LIMIT 1
         ) c ON TRUE
         WHERE r.department_id = $1
           AND a.start_date <= $3
           AND a.end_date >= $2",
    )
    .bind(department_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let budget_period_set: HashSet<String> = budget_periods.iter().cloned().collect();
    let mut current_committed_idr = 0i64;
    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());

    for row in allocation_rows {
        let allocation_start: NaiveDate = row
            .try_get("start_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let allocation_end: NaiveDate = row
            .try_get("end_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let include_weekend: bool = row
            .try_get("include_weekend")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let allocation_percentage_bd: BigDecimal = row
            .try_get("allocation_percentage")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let overlap_start = std::cmp::max(allocation_start, start_date);
        let overlap_end = std::cmp::min(allocation_end, end_date);
        if overlap_start > overlap_end {
            continue;
        }

        let Some(daily_rate_idr) = extract_daily_rate_from_allocation_row(&row, &crypto_svc).await? else {
            continue;
        };

        let allocation_cost = calculate_cost_preview(
            daily_rate_idr,
            overlap_start,
            overlap_end,
            bigdecimal_to_f64(allocation_percentage_bd),
            include_weekend,
            holidays,
        );

        current_committed_idr += allocation_cost
            .monthly_breakdown
            .iter()
            .filter(|bucket| budget_period_set.contains(&bucket.month))
            .map(|bucket| bucket.cost_idr)
            .sum::<i64>();
    }

    let this_assignment_cost = monthly_breakdown.iter().map(|b| b.cost_idr).sum::<i64>();
    let projected_committed_idr = current_committed_idr + this_assignment_cost;
    let remaining_after_assignment_idr = department_budget_total_idr - projected_committed_idr;
    let utilization_percentage = if department_budget_total_idr <= 0 {
        0.0
    } else {
        (projected_committed_idr as f64 / department_budget_total_idr as f64) * 100.0
    };

    let budget_health = if utilization_percentage < 50.0 {
        "healthy"
    } else if utilization_percentage <= 80.0 {
        "warning"
    } else {
        "critical"
    }
    .to_string();

    let requires_approval = budget_health == "critical"
        && std::env::var("BUDGET_OVERRUN_POLICY")
            .unwrap_or_else(|_| "warn".to_string())
            .eq_ignore_ascii_case("block");

    let warning = if budget_health == "critical" {
        Some(format!(
            "This assignment consumes Rp {}M of your Rp {}M budget ({:.1}% utilized)",
            format_idr_millions(this_assignment_cost),
            format_idr_millions(department_budget_total_idr),
            utilization_percentage
        ))
    } else {
        None
    };

    if budget_health == "critical" {
        let user_id = user_id_from_headers(headers)?;
        log_audit(
            pool,
            user_id,
            "budget_preview_critical",
            "allocation",
            project_id,
            json!({
                "resource_id": resource_id,
                "department_id": department_id,
                "projected_committed_idr": projected_committed_idr,
                "department_budget_total_idr": department_budget_total_idr,
                "utilization_percentage": utilization_percentage,
            }),
        )
        .await
        .ok();
    }

    Ok(BudgetComputation {
        budget_impact: Some(BudgetImpact {
            department_budget_total_idr,
            current_committed_idr,
            projected_committed_idr,
            remaining_after_assignment_idr,
            utilization_percentage,
            budget_health,
        }),
        warning,
        requires_approval,
    })
}

// is_weekend imported from crate::services::cost_preview

/// Get holidays within date range
async fn get_holidays_in_range(
    pool: &PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<NaiveDate>> {
    let holidays: Vec<NaiveDate> = sqlx::query_scalar!(
        "SELECT date FROM holidays WHERE date >= $1 AND date <= $2",
        start_date,
        end_date
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(holidays)
}

/// Get resource working hours configuration
async fn get_resource_working_hours(pool: &PgPool, resource_id: Uuid) -> Result<f64> {
    let working_hours: Option<sqlx::types::BigDecimal> = sqlx::query_scalar!(
        "SELECT working_hours FROM resources WHERE id = $1",
        resource_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let hours = working_hours.map(bigdecimal_to_f64).unwrap_or(8.0);

    Ok(hours)
}

/// Get existing allocations for resource in date range
async fn get_existing_allocations(
    pool: &PgPool,
    resource_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
    exclude_allocation_id: Option<Uuid>,
) -> Result<Vec<(Uuid, NaiveDate, NaiveDate, f64, bool)>> {
    let allocations: Vec<(Uuid, NaiveDate, NaiveDate, f64, bool)> =
        if let Some(exclude_id) = exclude_allocation_id {
            sqlx::query!(
                "SELECT id, start_date, end_date, allocation_percentage, include_weekend
             FROM allocations 
             WHERE resource_id = $1 
             AND id != $2
             AND (
                 (start_date <= $3 AND end_date >= $3) OR
                 (start_date <= $4 AND end_date >= $4) OR
                 (start_date >= $3 AND end_date <= $4)
             )",
                resource_id,
                exclude_id,
                start_date,
                end_date
            )
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .into_iter()
            .map(|row| {
                (
                    row.id,
                    row.start_date,
                    row.end_date,
                    bigdecimal_to_f64(row.allocation_percentage),
                    row.include_weekend,
                )
            })
            .collect()
        } else {
            sqlx::query!(
                "SELECT id, start_date, end_date, allocation_percentage, include_weekend
             FROM allocations 
             WHERE resource_id = $1 
             AND (
                 (start_date <= $2 AND end_date >= $2) OR
                 (start_date <= $3 AND end_date >= $3) OR
                 (start_date >= $2 AND end_date <= $3)
             )",
                resource_id,
                start_date,
                end_date
            )
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .into_iter()
            .map(|row| {
                (
                    row.id,
                    row.start_date,
                    row.end_date,
                    bigdecimal_to_f64(row.allocation_percentage),
                    row.include_weekend,
                )
            })
            .collect()
        };

    Ok(allocations)
}

/// Calculate daily allocations for a resource
async fn calculate_daily_allocations(
    pool: &PgPool,
    resource_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
    new_allocation_percentage: f64,
    new_start_date: NaiveDate,
    new_end_date: NaiveDate,
    new_include_weekend: bool,
    exclude_allocation_id: Option<Uuid>,
) -> Result<(HashMap<NaiveDate, DailyAllocation>, f64)> {
    // Get working hours capacity
    let daily_capacity = get_resource_working_hours(pool, resource_id).await?;

    // Get holidays in range
    let holidays = get_holidays_in_range(pool, start_date, end_date).await?;
    let holiday_set: std::collections::HashSet<_> = holidays.into_iter().collect();

    // Get existing allocations
    let existing_allocations = get_existing_allocations(
        pool,
        resource_id,
        start_date,
        end_date,
        exclude_allocation_id,
    )
    .await?;

    // Initialize daily allocations map
    let mut daily_allocations: HashMap<NaiveDate, DailyAllocation> = HashMap::new();

    // Process existing allocations
    for (alloc_id, alloc_start, alloc_end, percentage, include_weekend) in existing_allocations {
        let mut current_date = alloc_start;
        while current_date <= alloc_end {
            // Skip weekends and holidays
            if (include_weekend || !is_weekend(current_date))
                && !holiday_set.contains(&current_date)
            {
                let hours = daily_capacity * (percentage / 100.0);

                daily_allocations
                    .entry(current_date)
                    .or_insert_with(|| DailyAllocation {
                        date: current_date,
                        allocated_hours: 0.0,
                        assignments: Vec::new(),
                    })
                    .allocated_hours += hours;

                if let Some(entry) = daily_allocations.get_mut(&current_date) {
                    entry.assignments.push(AssignmentInfo {
                        allocation_id: alloc_id,
                        project_id: Uuid::nil(), // Will be filled if needed
                        hours,
                    });
                }
            }
            current_date = current_date.succ_opt().unwrap_or(current_date);
        }
    }

    // Add new allocation
    let mut current_date = new_start_date;
    while current_date <= new_end_date {
        if (new_include_weekend || !is_weekend(current_date))
            && !holiday_set.contains(&current_date)
        {
            let new_hours = daily_capacity * (new_allocation_percentage / 100.0);

            daily_allocations
                .entry(current_date)
                .or_insert_with(|| DailyAllocation {
                    date: current_date,
                    allocated_hours: 0.0,
                    assignments: Vec::new(),
                })
                .allocated_hours += new_hours;
        }
        current_date = current_date.succ_opt().unwrap_or(current_date);
    }

    Ok((daily_allocations, daily_capacity))
}

/// Check if resource has capacity for new allocation
async fn check_resource_capacity(
    pool: &PgPool,
    resource_id: Uuid,
    new_start_date: NaiveDate,
    new_end_date: NaiveDate,
    new_allocation_percentage: f64,
    new_include_weekend: bool,
    exclude_allocation_id: Option<Uuid>,
) -> Result<(bool, String, f64)> {
    // Calculate date range to check (union of all allocations)
    let (existing_start, existing_end) = if let Some(exclude_id) = exclude_allocation_id {
        let row = sqlx::query!(
            "SELECT MIN(start_date) as min_start, MAX(end_date) as max_end
             FROM allocations 
             WHERE resource_id = $1 AND id != $2",
            resource_id,
            exclude_id
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        (
            row.as_ref()
                .and_then(|r| r.min_start)
                .unwrap_or(new_start_date),
            row.as_ref().and_then(|r| r.max_end).unwrap_or(new_end_date),
        )
    } else {
        let row = sqlx::query!(
            "SELECT MIN(start_date) as min_start, MAX(end_date) as max_end
             FROM allocations 
             WHERE resource_id = $1",
            resource_id
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        (
            row.as_ref()
                .and_then(|r| r.min_start)
                .unwrap_or(new_start_date),
            row.as_ref().and_then(|r| r.max_end).unwrap_or(new_end_date),
        )
    };

    let check_start = std::cmp::min(existing_start, new_start_date);
    let check_end = std::cmp::max(existing_end, new_end_date);

    // Calculate daily allocations
    let (daily_allocations, daily_capacity) = calculate_daily_allocations(
        pool,
        resource_id,
        check_start,
        check_end,
        new_allocation_percentage,
        new_start_date,
        new_end_date,
        new_include_weekend,
        exclude_allocation_id,
    )
    .await?;

    // Check for over-allocation
    let mut over_allocated_days: Vec<(NaiveDate, f64)> = Vec::new();

    for (date, allocation) in &daily_allocations {
        if allocation.allocated_hours > daily_capacity {
            over_allocated_days.push((*date, allocation.allocated_hours));
        }
    }

    // Sort by date
    over_allocated_days.sort_by(|a, b| a.0.cmp(&b.0));

    let has_capacity = over_allocated_days.is_empty();

    let message = if has_capacity {
        format!(
            "Resource has sufficient capacity. Daily capacity: {:.1} hours",
            daily_capacity
        )
    } else {
        let days_str = over_allocated_days
            .iter()
            .map(|(date, hours)| format!("{} ({:.1}h allocated)", date, hours))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "Resource over-allocated on: {}. Daily capacity: {:.1} hours",
            days_str, daily_capacity
        )
    };

    Ok((has_capacity, message, daily_capacity))
}

/// Validate allocation dates are within project dates
async fn validate_allocation_dates(
    pool: &PgPool,
    project_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<()> {
    let project = sqlx::query!(
        "SELECT start_date, end_date FROM projects WHERE id = $1",
        project_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    if start_date < project.start_date {
        return Err(AppError::Validation(format!(
            "Allocation start date ({}) cannot be before project start date ({})",
            start_date, project.start_date
        )));
    }

    if end_date > project.end_date {
        return Err(AppError::Validation(format!(
            "Allocation end date ({}) cannot be after project end date ({})",
            end_date, project.end_date
        )));
    }

    Ok(())
}

/// Get all allocations with project and resource names
async fn get_allocations(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<Vec<AllocationResponse>>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    let is_pm = claims.role == "project_manager";
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    let response: Vec<AllocationResponse> = if is_pm {
        sqlx::query!(
            "SELECT a.id, a.project_id, a.resource_id, a.start_date, a.end_date, a.allocation_percentage, a.include_weekend,
                    p.name as project_name, r.name as resource_name
             FROM allocations a
             JOIN projects p ON a.project_id = p.id
             JOIN resources r ON a.resource_id = r.id
             WHERE p.project_manager_id = $1
             ORDER BY a.start_date DESC",
             user_id
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .into_iter()
        .map(|a| {
            Ok(AllocationResponse {
                id: a.id,
                project_id: required_uuid(a.project_id, "allocations.project_id")?,
                resource_id: required_uuid(a.resource_id, "allocations.resource_id")?,
                start_date: a.start_date,
                end_date: a.end_date,
                allocation_percentage: bigdecimal_to_f64(a.allocation_percentage),
                include_weekend: a.include_weekend,
                project_name: a.project_name,
                resource_name: a.resource_name,
            })
        })
        .collect::<Result<Vec<_>>>()?
    } else {
        sqlx::query!(
            "SELECT a.id, a.project_id, a.resource_id, a.start_date, a.end_date, a.allocation_percentage, a.include_weekend,
                    p.name as project_name, r.name as resource_name
             FROM allocations a
             JOIN projects p ON a.project_id = p.id
             JOIN resources r ON a.resource_id = r.id
             ORDER BY a.start_date DESC"
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .into_iter()
        .map(|a| {
            Ok(AllocationResponse {
                id: a.id,
                project_id: required_uuid(a.project_id, "allocations.project_id")?,
                resource_id: required_uuid(a.resource_id, "allocations.resource_id")?,
                start_date: a.start_date,
                end_date: a.end_date,
                allocation_percentage: bigdecimal_to_f64(a.allocation_percentage),
                include_weekend: a.include_weekend,
                project_name: a.project_name,
                resource_name: a.resource_name,
            })
        })
        .collect::<Result<Vec<_>>>()?
    };

    Ok(Json(response))
}

/// Get allocations by project ID
async fn get_allocations_by_project(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<AllocationResponse>>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    let is_pm = claims.role == "project_manager";
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    // Check project assignment if PM
    if is_pm {
        let pm_id = sqlx::query_scalar!(
            "SELECT project_manager_id FROM projects WHERE id = $1",
            project_id
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .flatten();

        if pm_id != Some(user_id) {
            log_audit(
                &pool,
                Some(user_id),
                "ACCESS_DENIED",
                "allocation",
                project_id,
                serde_json::json!({
                    "reason": "not_project_manager",
                    "attempted_role": claims.role
                }),
            )
            .await
            .ok();
            return Err(AppError::Forbidden("Insufficient permissions".to_string()));
        }
    }

    let allocations = sqlx::query!(
        "SELECT a.id, a.project_id, a.resource_id, a.start_date, a.end_date, a.allocation_percentage, a.include_weekend,
                p.name as project_name, r.name as resource_name
         FROM allocations a
         JOIN projects p ON a.project_id = p.id
         JOIN resources r ON a.resource_id = r.id
         WHERE a.project_id = $1
         ORDER BY a.start_date DESC",
        project_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let response: Vec<AllocationResponse> = allocations
        .into_iter()
        .map(|a| {
            Ok(AllocationResponse {
                id: a.id,
                project_id: required_uuid(a.project_id, "allocations.project_id")?,
                resource_id: required_uuid(a.resource_id, "allocations.resource_id")?,
                start_date: a.start_date,
                end_date: a.end_date,
                allocation_percentage: bigdecimal_to_f64(a.allocation_percentage),
                include_weekend: a.include_weekend,
                project_name: a.project_name,
                resource_name: a.resource_name,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Json(response))
}

/// Get allocations by resource ID
async fn get_allocations_by_resource(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(resource_id): Path<Uuid>,
) -> Result<Json<Vec<AllocationResponse>>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    let is_pm = claims.role == "project_manager";
    let allocations = sqlx::query!(
        "SELECT a.id, a.project_id, a.resource_id, a.start_date, a.end_date, a.allocation_percentage, a.include_weekend,
                p.name as project_name, r.name as resource_name
         FROM allocations a
         JOIN projects p ON a.project_id = p.id
         JOIN resources r ON a.resource_id = r.id
         WHERE a.resource_id = $1
           AND ($2 = FALSE OR p.project_manager_id = $3)
         ORDER BY a.start_date DESC",
        resource_id,
        is_pm,
        user_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let response: Vec<AllocationResponse> = allocations
        .into_iter()
        .map(|a| {
            Ok(AllocationResponse {
                id: a.id,
                project_id: required_uuid(a.project_id, "allocations.project_id")?,
                resource_id: required_uuid(a.resource_id, "allocations.resource_id")?,
                start_date: a.start_date,
                end_date: a.end_date,
                allocation_percentage: bigdecimal_to_f64(a.allocation_percentage),
                include_weekend: a.include_weekend,
                project_name: a.project_name,
                resource_name: a.resource_name,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Json(response))
}

async fn cost_preview(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<CostPreviewQuery>,
) -> Result<Json<CostPreviewResponse>> {
    ensure_allocation_access(
        &pool,
        &headers,
        "cost_preview",
        query.project_id,
        Some(query.project_id),
    )
    .await?;

    if query.allocation_percentage <= 0.0 || query.allocation_percentage > 100.0 {
        return Err(AppError::Validation(
            "Allocation percentage must be greater than 0 and at most 100.".to_string(),
        ));
    }

    if query.start_date > query.end_date {
        return Err(AppError::Validation(
            "Start date cannot be after end date.".to_string(),
        ));
    }

    validate_allocation_dates(&pool, query.project_id, query.start_date, query.end_date).await?;

    let daily_rate_idr = get_active_ctc_daily_rate(&pool, query.resource_id).await?;
    let holidays = get_holidays_in_range(&pool, query.start_date, query.end_date).await?;

    let preview_result = calculate_cost_preview(
        daily_rate_idr,
        query.start_date,
        query.end_date,
        query.allocation_percentage,
        query.include_weekend,
        &holidays,
    );

    let budget = compute_budget_impact(
        &pool,
        &headers,
        query.resource_id,
        query.project_id,
        query.start_date,
        query.end_date,
        &preview_result.monthly_breakdown,
        &holidays,
    )
    .await?;

    Ok(Json(CostPreviewResponse {
        daily_rate_idr,
        working_days: preview_result.working_days,
        allocation_percentage: query.allocation_percentage,
        total_cost_idr: preview_result.total_cost_idr,
        monthly_breakdown: preview_result.monthly_breakdown,
        budget_impact: budget.budget_impact,
        warning: budget.warning,
        requires_approval: budget.requires_approval,
    }))
}

/// Create a new allocation
async fn create_allocation(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(req): Json<CreateAllocationRequest>,
) -> Result<Json<AllocationResponse>> {
    ensure_allocation_access(
        &pool,
        &headers,
        "create",
        req.project_id,
        Some(req.project_id),
    )
    .await?;

    // Validate allocation percentage bounds
    if req.allocation_percentage <= 0.0 || req.allocation_percentage > 100.0 {
        return Err(AppError::Validation(
            "Allocation percentage must be greater than 0 and at most 100.".to_string(),
        ));
    }

    // Validate date ordering
    if req.start_date > req.end_date {
        return Err(AppError::Validation(
            "Start date cannot be after end date.".to_string(),
        ));
    }

    let ctc_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM ctc_records WHERE resource_id = $1 AND status = 'Active')",
    )
    .bind(req.resource_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if !ctc_exists {
        return Err(AppError::Validation(
            "Cannot assign resource without CTC data. Contact HR to complete CTC entry for this employee."
                .to_string(),
        ));
    }

    let audit_changes = audit_payload(
        None,
        Some(json!({
            "project_id": req.project_id,
            "resource_id": req.resource_id,
            "start_date": req.start_date,
            "end_date": req.end_date,
            "allocation_percentage": req.allocation_percentage,
            "include_weekend": req.include_weekend,
        })),
    );
    let user_id = user_id_from_headers(&headers)?;
    // Validate dates are within project dates
    validate_allocation_dates(&pool, req.project_id, req.start_date, req.end_date).await?;

    // Check if resource has capacity
    let (has_capacity, message, _daily_capacity) = check_resource_capacity(
        &pool,
        req.resource_id,
        req.start_date,
        req.end_date,
        req.allocation_percentage,
        req.include_weekend,
        None,
    )
    .await?;

    if !has_capacity {
        return Err(AppError::Validation(message));
    }

    let allocation_percentage_bd = f64_to_bigdecimal(req.allocation_percentage);

    let allocation = sqlx::query!(
        "INSERT INTO allocations (project_id, resource_id, start_date, end_date, allocation_percentage, include_weekend)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, project_id, resource_id, start_date, end_date, allocation_percentage, include_weekend",
        req.project_id,
        req.resource_id,
        req.start_date,
        req.end_date,
        allocation_percentage_bd,
        req.include_weekend
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        user_id,
        "create",
        "allocation",
        allocation.id,
        audit_changes,
    )
    .await?;

    // Get project and resource names
    let project_name =
        sqlx::query_scalar!("SELECT name FROM projects WHERE id = $1", req.project_id)
            .fetch_one(&pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

    let resource_name =
        sqlx::query_scalar!("SELECT name FROM resources WHERE id = $1", req.resource_id)
            .fetch_one(&pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(AllocationResponse {
        id: allocation.id,
        project_id: required_uuid(allocation.project_id, "allocations.project_id")?,
        resource_id: required_uuid(allocation.resource_id, "allocations.resource_id")?,
        start_date: allocation.start_date,
        end_date: allocation.end_date,
        allocation_percentage: bigdecimal_to_f64(allocation.allocation_percentage),
        include_weekend: allocation.include_weekend,
        project_name,
        resource_name,
    }))
}

/// Update an allocation
async fn update_allocation(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAllocationRequest>,
) -> Result<Json<AllocationResponse>> {
    // Check if allocation exists
    let existing = sqlx::query!(
        "SELECT id, project_id, resource_id, start_date, end_date, allocation_percentage, include_weekend FROM allocations WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Allocation {} not found", id)))?;

    // Determine values for validation
    let resource_id = req.resource_id.or(existing.resource_id).ok_or_else(|| {
        AppError::Internal("allocations.resource_id is unexpectedly null".to_string())
    })?;
    let project_id = req.project_id.or(existing.project_id).ok_or_else(|| {
        AppError::Internal("allocations.project_id is unexpectedly null".to_string())
    })?;
    let start_date = req
        .start_date
        .or(Some(existing.start_date))
        .ok_or_else(|| {
            AppError::Internal("allocations.start_date is unexpectedly null".to_string())
        })?;
    let end_date = req.end_date.or(Some(existing.end_date)).ok_or_else(|| {
        AppError::Internal("allocations.end_date is unexpectedly null".to_string())
    })?;

    ensure_allocation_access(&pool, &headers, "update", id, Some(project_id)).await?;
    let existing_percentage = bigdecimal_to_f64(existing.allocation_percentage);
    let new_percentage = req.allocation_percentage.unwrap_or(existing_percentage);
    let include_weekend = req.include_weekend.unwrap_or(existing.include_weekend);

    // Validate allocation percentage bounds
    if new_percentage <= 0.0 || new_percentage > 100.0 {
        return Err(AppError::Validation(
            "Allocation percentage must be greater than 0 and at most 100.".to_string(),
        ));
    }

    // Validate date ordering
    if start_date > end_date {
        return Err(AppError::Validation(
            "Start date cannot be after end date.".to_string(),
        ));
    }

    // Validate dates are within project dates
    validate_allocation_dates(&pool, project_id, start_date, end_date).await?;

    // Check if resource has capacity (excluding this allocation)
    let (has_capacity, message, _daily_capacity) = check_resource_capacity(
        &pool,
        resource_id,
        start_date,
        end_date,
        new_percentage,
        include_weekend,
        Some(id),
    )
    .await?;

    if !has_capacity {
        return Err(AppError::Validation(message));
    }

    let audit_changes = audit_payload(
        Some(json!({
            "project_id": existing.project_id,
            "resource_id": existing.resource_id,
            "start_date": existing.start_date,
            "end_date": existing.end_date,
            "allocation_percentage": existing_percentage,
            "include_weekend": existing.include_weekend,
        })),
        Some(json!({
            "project_id": project_id,
            "resource_id": resource_id,
            "start_date": start_date,
            "end_date": end_date,
            "allocation_percentage": new_percentage,
            "include_weekend": include_weekend,
        })),
    );
    let user_id = user_id_from_headers(&headers)?;

    // Convert percentage if provided
    let allocation_percentage_bd = req.allocation_percentage.map(f64_to_bigdecimal);

    // Update with new values or keep existing
    let allocation = sqlx::query!(
        "UPDATE allocations 
         SET project_id = COALESCE($1, project_id),
             resource_id = COALESCE($2, resource_id),
             start_date = COALESCE($3, start_date),
             end_date = COALESCE($4, end_date),
             allocation_percentage = COALESCE($5, allocation_percentage),
             include_weekend = COALESCE($6, include_weekend)
         WHERE id = $7
         RETURNING id, project_id, resource_id, start_date, end_date, allocation_percentage, include_weekend",
        req.project_id,
        req.resource_id,
        req.start_date,
        req.end_date,
        allocation_percentage_bd,
        req.include_weekend,
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        user_id,
        "update",
        "allocation",
        allocation.id,
        audit_changes,
    )
    .await?;

    // Get project and resource names
    let project_id = required_uuid(allocation.project_id, "allocations.project_id")?;
    let resource_id = required_uuid(allocation.resource_id, "allocations.resource_id")?;

    let project_name = sqlx::query_scalar!("SELECT name FROM projects WHERE id = $1", project_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let resource_name =
        sqlx::query_scalar!("SELECT name FROM resources WHERE id = $1", resource_id)
            .fetch_one(&pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(AllocationResponse {
        id: allocation.id,
        project_id,
        resource_id,
        start_date: allocation.start_date,
        end_date: allocation.end_date,
        allocation_percentage: bigdecimal_to_f64(allocation.allocation_percentage),
        include_weekend: allocation.include_weekend,
        project_name,
        resource_name,
    }))
}

/// Delete an allocation
async fn delete_allocation(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check if allocation exists
    let existing = sqlx::query!(
        "SELECT id, project_id, resource_id, start_date, end_date, allocation_percentage, include_weekend FROM allocations WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Allocation {} not found", id)))?;

    ensure_allocation_access(&pool, &headers, "delete", id, existing.project_id).await?;

    // Delete the allocation
    sqlx::query!("DELETE FROM allocations WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let user_id = user_id_from_headers(&headers)?;
    let audit_changes = audit_payload(
        Some(json!({
            "project_id": existing.project_id,
            "resource_id": existing.resource_id,
            "start_date": existing.start_date,
            "end_date": existing.end_date,
            "allocation_percentage": bigdecimal_to_f64(existing.allocation_percentage),
            "include_weekend": existing.include_weekend,
        })),
        None,
    );
    log_audit(&pool, user_id, "delete", "allocation", id, audit_changes).await?;

    Ok(Json(
        serde_json::json!({"message": "Allocation deleted successfully"}),
    ))
}

/// Create allocation routes
pub fn allocation_routes() -> Router<PgPool> {
    Router::new()
        .route("/allocations", get(get_allocations).post(create_allocation))
        .route("/allocations/cost-preview", get(cost_preview))
        .route(
            "/allocations/:id",
            put(update_allocation).delete(delete_allocation),
        )
        .route(
            "/allocations/project/:project_id",
            get(get_allocations_by_project),
        )
        .route(
            "/allocations/resource/:resource_id",
            get(get_allocations_by_resource),
        )
}
