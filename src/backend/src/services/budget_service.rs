use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::types::BigDecimal;
use sqlx::{Postgres, Row, Transaction};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::cost_preview::{calculate_cost_preview, is_weekend};
use crate::services::ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService, EncryptedPayload};
use crate::services::key_provider::EnvKeyProvider;

#[derive(Debug, Serialize)]
pub struct DepartmentBudgetSummaryResponse {
    pub department_id: Uuid,
    pub department_name: String,
    pub budget_period: String,
    pub total_budget_idr: i64,
    pub total_committed_idr: i64,
    pub spent_actual_idr: i64,
    pub spent_actual_source: String,
    pub remaining_idr: i64,
    pub utilization_percentage: f64,
    pub budget_health: String,
    pub alert_threshold_pct: i32,
    pub budget_configured: bool,
}

#[derive(Debug, Serialize)]
pub struct BudgetBreakdownResponse {
    pub department_id: Uuid,
    pub period: String,
    pub by_employee: Vec<EmployeeBudgetEntry>,
    pub by_project: Vec<ProjectBudgetEntry>,
    pub by_period: Vec<PeriodBudgetEntry>,
}

#[derive(Debug, Serialize)]
pub struct EmployeeBudgetEntry {
    pub resource_id: Uuid,
    pub resource_name: String,
    pub daily_rate_idr: Option<i64>,
    pub allocation_count: i32,
    pub working_days: i32,
    pub committed_cost_idr: i64,
}

#[derive(Debug, Serialize)]
pub struct ProjectBudgetEntry {
    pub project_id: Uuid,
    pub project_name: String,
    pub resource_count: i32,
    pub committed_cost_idr: i64,
}

#[derive(Debug, Serialize)]
pub struct PeriodBudgetEntry {
    pub period: String,
    pub total_budget_idr: i64,
    pub committed_idr: i64,
    pub remaining_idr: i64,
    pub utilization_percentage: f64,
    pub budget_health: String,
    pub budget_configured: bool,
}

struct EmployeeAccumulator {
    resource_id: Uuid,
    resource_name: String,
    daily_rate_idr: Option<i64>,
    allocation_count: i32,
    working_days: i32,
    committed_cost_idr: i64,
}

struct ProjectAccumulator {
    project_id: Uuid,
    project_name: String,
    resource_ids: HashSet<Uuid>,
    committed_cost_idr: i64,
}

fn budget_health(utilization_percentage: f64) -> String {
    if utilization_percentage < 50.0 {
        "healthy".to_string()
    } else if utilization_percentage < 80.0 {
        "warning".to_string()
    } else {
        "critical".to_string()
    }
}

fn parse_period(period: &str) -> Result<(i32, u32)> {
    let parts: Vec<&str> = period.split('-').collect();
    if parts.len() != 2 || parts[0].len() != 4 || parts[1].len() != 2 {
        return Err(AppError::Validation(
            "period must be in YYYY-MM format".to_string(),
        ));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| AppError::Validation("Invalid year in period".to_string()))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| AppError::Validation("Invalid month in period".to_string()))?;

    if month == 0 || month > 12 {
        return Err(AppError::Validation(
            "Month in period must be between 01 and 12".to_string(),
        ));
    }

    let normalized = format!("{:04}-{:02}", year, month);
    if normalized != period {
        return Err(AppError::Validation(
            "period must be in YYYY-MM format".to_string(),
        ));
    }

    Ok((year, month))
}

fn period_start_end(period: &str) -> Result<(NaiveDate, NaiveDate)> {
    let (year, month) = parse_period(period)?;
    let start = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| AppError::Validation("Invalid period date".to_string()))?;

    let next_month_start = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| AppError::Validation("Invalid period date".to_string()))?;

    let end = next_month_start
        .pred_opt()
        .ok_or_else(|| AppError::Internal("Failed to determine period end date".to_string()))?;

    Ok((start, end))
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
            AppError::Internal(format!(
                "Failed to parse '{}' from decrypted payload",
                field
            ))
        }),
        serde_json::Value::Number(n) => n.to_string().parse::<BigDecimal>().map_err(|_| {
            AppError::Internal(format!(
                "Failed to parse '{}' from decrypted payload",
                field
            ))
        }),
        _ => Err(AppError::Internal(format!(
            "Invalid '{}' value in decrypted payload",
            field
        ))),
    }
}

async fn extract_daily_rate_from_allocation_row(
    row: &sqlx::postgres::PgRow,
    crypto_svc: &DefaultCtcCryptoService<EnvKeyProvider>,
) -> Result<Option<i64>> {
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

async fn load_holidays(tx: &mut Transaction<'_, Postgres>) -> Result<Vec<NaiveDate>> {
    let holiday_rows = sqlx::query("SELECT date::TEXT as date FROM holidays")
        .fetch_all(&mut **tx)
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

pub async fn compute_department_budget_utilization(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Uuid,
    period: &str,
) -> Result<DepartmentBudgetSummaryResponse> {
    let (period_start, period_end) = period_start_end(period)?;

    let department_name: String = sqlx::query_scalar("SELECT name FROM departments WHERE id = $1")
        .bind(department_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Department {} not found", department_id)))?;

    let budget_row = sqlx::query(
        "SELECT total_budget_idr, alert_threshold_pct
         FROM department_budgets
         WHERE department_id = $1 AND budget_period = $2",
    )
    .bind(department_id)
    .bind(period)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let holidays = load_holidays(tx).await?;

    let allocation_rows = sqlx::query(
        "SELECT a.resource_id, a.project_id, a.start_date, a.end_date, a.allocation_percentage, a.include_weekend,
                r.name AS resource_name,
                p.name AS project_name,
                c.daily_rate, c.encrypted_daily_rate, c.key_version, c.encryption_version,
                c.encryption_algorithm, c.encrypted_at
         FROM allocations a
         JOIN resources r ON r.id = a.resource_id
         JOIN projects p ON p.id = a.project_id
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
    .bind(period_start)
    .bind(period_end)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let mut total_committed_idr = 0i64;
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
        let allocation_percentage = allocation_percentage_bd
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0);

        let overlap_start = std::cmp::max(allocation_start, period_start);
        let overlap_end = std::cmp::min(allocation_end, period_end);
        if overlap_start > overlap_end {
            continue;
        }

        let Some(daily_rate_idr) = extract_daily_rate_from_allocation_row(&row, &crypto_svc).await? else {
            continue;
        };

        let preview = calculate_cost_preview(
            daily_rate_idr,
            overlap_start,
            overlap_end,
            allocation_percentage,
            include_weekend,
            &holidays,
        );

        total_committed_idr += preview
            .monthly_breakdown
            .iter()
            .filter(|bucket| bucket.month == period)
            .map(|bucket| bucket.cost_idr)
            .sum::<i64>();
    }

    let total_budget_idr = if let Some(row) = &budget_row {
        row.try_get::<i64, _>("total_budget_idr")
            .map_err(|e| AppError::Database(e.to_string()))?
    } else {
        0
    };

    let alert_threshold_pct = if let Some(row) = &budget_row {
        let raw: i16 = row
            .try_get("alert_threshold_pct")
            .map_err(|e| AppError::Database(e.to_string()))?;
        i32::from(raw)
    } else {
        80
    };

    let spent_actual_idr = total_committed_idr;
    let remaining_idr = total_budget_idr - total_committed_idr;
    let utilization_percentage = if total_budget_idr <= 0 {
        0.0
    } else {
        (total_committed_idr as f64 / total_budget_idr as f64) * 100.0
    };

    Ok(DepartmentBudgetSummaryResponse {
        department_id,
        department_name,
        budget_period: period.to_string(),
        total_budget_idr,
        total_committed_idr,
        spent_actual_idr,
        spent_actual_source: "committed_proxy".to_string(),
        remaining_idr,
        utilization_percentage,
        budget_health: budget_health(utilization_percentage),
        alert_threshold_pct,
        budget_configured: budget_row.is_some(),
    })
}

pub async fn compute_budget_breakdown(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Uuid,
    periods: Vec<String>,
) -> Result<BudgetBreakdownResponse> {
    if periods.is_empty() {
        return Err(AppError::Validation(
            "At least one period is required".to_string(),
        ));
    }

    let mut period_bounds = Vec::new();
    for period in &periods {
        let (start, end) = period_start_end(period)?;
        period_bounds.push((period.clone(), start, end));
    }

    let min_start = period_bounds
        .iter()
        .map(|(_, start, _)| *start)
        .min()
        .ok_or_else(|| AppError::Internal("Failed to compute period range".to_string()))?;
    let max_end = period_bounds
        .iter()
        .map(|(_, _, end)| *end)
        .max()
        .ok_or_else(|| AppError::Internal("Failed to compute period range".to_string()))?;

    let holidays = load_holidays(tx).await?;

    let allocation_rows = sqlx::query(
        "SELECT a.resource_id, a.project_id, a.start_date, a.end_date, a.allocation_percentage, a.include_weekend,
                r.name AS resource_name,
                p.name AS project_name,
                c.daily_rate, c.encrypted_daily_rate, c.key_version, c.encryption_version,
                c.encryption_algorithm, c.encrypted_at
         FROM allocations a
         JOIN resources r ON r.id = a.resource_id
         JOIN projects p ON p.id = a.project_id
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
    .bind(min_start)
    .bind(max_end)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let budget_rows = sqlx::query(
        "SELECT budget_period, total_budget_idr, alert_threshold_pct
         FROM department_budgets
         WHERE department_id = $1
           AND budget_period = ANY($2)",
    )
    .bind(department_id)
    .bind(&periods)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut budgets_by_period: HashMap<String, (i64, i32)> = HashMap::new();
    for row in budget_rows {
        let budget_period: String = row
            .try_get("budget_period")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let total_budget_idr: i64 = row
            .try_get("total_budget_idr")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let alert_threshold_raw: i16 = row
            .try_get("alert_threshold_pct")
            .map_err(|e| AppError::Database(e.to_string()))?;
        budgets_by_period.insert(budget_period, (total_budget_idr, i32::from(alert_threshold_raw)));
    }

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let mut employee_map: HashMap<Uuid, EmployeeAccumulator> = HashMap::new();
    let mut project_map: HashMap<Uuid, ProjectAccumulator> = HashMap::new();
    let mut period_committed: HashMap<String, i64> = periods.iter().map(|p| (p.clone(), 0)).collect();

    for row in allocation_rows {
        let resource_id: Uuid = row
            .try_get("resource_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let project_id: Uuid = row
            .try_get("project_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let resource_name: String = row
            .try_get("resource_name")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let project_name: String = row
            .try_get("project_name")
            .map_err(|e| AppError::Database(e.to_string()))?;
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
        let allocation_percentage = allocation_percentage_bd
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0);

        let daily_rate_idr = extract_daily_rate_from_allocation_row(&row, &crypto_svc).await?;

        let employee_entry = employee_map
            .entry(resource_id)
            .or_insert_with(|| EmployeeAccumulator {
                resource_id,
                resource_name: resource_name.clone(),
                daily_rate_idr,
                allocation_count: 0,
                working_days: 0,
                committed_cost_idr: 0,
            });
        employee_entry.allocation_count += 1;
        if employee_entry.daily_rate_idr.is_none() {
            employee_entry.daily_rate_idr = daily_rate_idr;
        }

        let project_entry = project_map
            .entry(project_id)
            .or_insert_with(|| ProjectAccumulator {
                project_id,
                project_name: project_name.clone(),
                resource_ids: HashSet::new(),
                committed_cost_idr: 0,
            });
        project_entry.resource_ids.insert(resource_id);

        let Some(rate) = daily_rate_idr else {
            continue;
        };

        for (period, period_start, period_end) in &period_bounds {
            let overlap_start = std::cmp::max(allocation_start, *period_start);
            let overlap_end = std::cmp::min(allocation_end, *period_end);
            if overlap_start > overlap_end {
                continue;
            }

            let preview = calculate_cost_preview(
                rate,
                overlap_start,
                overlap_end,
                allocation_percentage,
                include_weekend,
                &holidays,
            );

            let month_cost = preview
                .monthly_breakdown
                .iter()
                .filter(|bucket| &bucket.month == period)
                .map(|bucket| bucket.cost_idr)
                .sum::<i64>();
            let month_working_days = preview
                .monthly_breakdown
                .iter()
                .filter(|bucket| &bucket.month == period)
                .map(|bucket| bucket.working_days)
                .sum::<i32>();

            if month_cost == 0 && month_working_days == 0 {
                continue;
            }

            employee_entry.committed_cost_idr += month_cost;
            employee_entry.working_days += month_working_days;
            project_entry.committed_cost_idr += month_cost;

            if let Some(committed) = period_committed.get_mut(period) {
                *committed += month_cost;
            }
        }
    }

    let mut by_employee = employee_map
        .into_values()
        .map(|entry| EmployeeBudgetEntry {
            resource_id: entry.resource_id,
            resource_name: entry.resource_name,
            daily_rate_idr: entry.daily_rate_idr,
            allocation_count: entry.allocation_count,
            working_days: entry.working_days,
            committed_cost_idr: entry.committed_cost_idr,
        })
        .collect::<Vec<_>>();
    by_employee.sort_by(|a, b| b.committed_cost_idr.cmp(&a.committed_cost_idr));

    let mut by_project = project_map
        .into_values()
        .map(|entry| ProjectBudgetEntry {
            project_id: entry.project_id,
            project_name: entry.project_name,
            resource_count: entry.resource_ids.len() as i32,
            committed_cost_idr: entry.committed_cost_idr,
        })
        .collect::<Vec<_>>();
    by_project.sort_by(|a, b| b.committed_cost_idr.cmp(&a.committed_cost_idr));

    let mut by_period = Vec::new();
    for period in &periods {
        let committed_idr = period_committed.get(period).copied().unwrap_or(0);
        let (total_budget_idr, budget_configured) = match budgets_by_period.get(period) {
            Some((total, _)) => (*total, true),
            None => (0, false),
        };

        let remaining_idr = total_budget_idr - committed_idr;
        let utilization_percentage = if total_budget_idr <= 0 {
            0.0
        } else {
            (committed_idr as f64 / total_budget_idr as f64) * 100.0
        };

        by_period.push(PeriodBudgetEntry {
            period: period.clone(),
            total_budget_idr,
            committed_idr,
            remaining_idr,
            utilization_percentage,
            budget_health: budget_health(utilization_percentage),
            budget_configured,
        });
    }

    let period_label = if periods.len() == 1 {
        periods[0].clone()
    } else {
        format!(
            "{}..{}",
            periods.first().cloned().unwrap_or_default(),
            periods.last().cloned().unwrap_or_default()
        )
    };

    Ok(BudgetBreakdownResponse {
        department_id,
        period: period_label,
        by_employee,
        by_project,
        by_period,
    })
}
