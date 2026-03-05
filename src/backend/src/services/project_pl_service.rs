use chrono::{Datelike, NaiveDate, Utc};
use serde::Serialize;
use sqlx::types::BigDecimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::project_cost_service::{
    compute_project_resource_costs, compute_project_resource_costs_in_window,
};
use crate::services::project_revenue_service::get_revenue_grid;

// ── Result types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProjectPlDashboardResult {
    pub project_id: Uuid,
    pub year: i32,
    pub total_revenue_idr: i64,
    pub total_cost_idr: i64,
    pub gross_profit_idr: i64,
    pub margin_pct: f64,
    pub target_margin_pct: f64,
    pub margin_alert_threshold_pct: f64,
    pub margin_alert: Option<String>,
    pub months: Vec<ProjectPlMonth>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectPlMonth {
    pub month: u32,
    pub month_label: String,
    pub revenue_idr: i64,
    pub resource_cost_idr: i64,
    pub non_resource_cost_idr: i64,
    pub total_cost_idr: i64,
    pub gross_profit_idr: i64,
    pub margin_pct: f64,
}

// ── Main service function ────────────────────────────────────────────────────

pub async fn get_project_pl_dashboard(
    pool: &PgPool,
    project_id: Uuid,
    year: i32,
) -> Result<ProjectPlDashboardResult> {
    // Compute year boundaries
    let start_date = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| AppError::Validation("Invalid year".to_string()))?;
    let end_date = NaiveDate::from_ymd_opt(year + 1, 1, 1)
        .ok_or_else(|| AppError::Validation("Invalid year".to_string()))?;

    // Fetch target margin settings and all three data sources in parallel
    let (pl_settings, revenue_grid, resource_costs, expense_months) = tokio::try_join!(
        fetch_pl_settings(pool, project_id),
        get_revenue_grid(pool, project_id, year),
        compute_project_resource_costs(pool, project_id),
        fetch_monthly_expenses(pool, project_id, start_date, end_date)
    )?;

    // Build dense 12-month P&L grid
    let month_labels = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let mut months: Vec<ProjectPlMonth> = Vec::with_capacity(12);

    // Create a map from resource costs (month format: "YYYY-MM")
    let mut resource_cost_map = std::collections::HashMap::new();
    for entry in resource_costs.monthly_breakdown {
        // Parse "YYYY-MM" format
        if let Ok(date) = NaiveDate::parse_from_str(&format!("{}-01", entry.month), "%Y-%m-%d") {
            let month_num = date.month();
            let entry_year = date.year();
            // Only include entries for the requested year
            if entry_year == year {
                resource_cost_map.insert(month_num, entry.cost_idr);
            }
        }
    }

    // Create a map from expenses (month number: 1..12)
    let mut expense_map = std::collections::HashMap::new();
    for (month_num, cost) in expense_months {
        expense_map.insert(month_num, cost);
    }

    // Build 12-month grid
    for month_idx in 0..12 {
        let month_num = (month_idx + 1) as u32; // 1..12

        // Revenue: revenue_grid.months is already dense 1..12
        let revenue_idr = revenue_grid
            .months
            .get(month_idx)
            .map(|m| m.amount_idr)
            .unwrap_or(0);

        // Resource cost from map (0 if not found)
        let resource_cost_idr = resource_cost_map.get(&month_num).copied().unwrap_or(0);

        // Non-resource cost (expenses) from map (0 if not found)
        let non_resource_cost_idr = expense_map.get(&month_num).copied().unwrap_or(0);

        // Total cost
        let total_cost_idr = resource_cost_idr + non_resource_cost_idr;

        // Gross profit
        let gross_profit_idr = revenue_idr - total_cost_idr;

        // Margin percentage (handle division by zero)
        let margin_pct = if revenue_idr > 0 {
            (gross_profit_idr as f64 / revenue_idr as f64) * 100.0
        } else {
            0.0
        };

        months.push(ProjectPlMonth {
            month: month_num,
            month_label: month_labels[month_idx].to_string(),
            revenue_idr,
            resource_cost_idr,
            non_resource_cost_idr,
            total_cost_idr,
            gross_profit_idr,
            margin_pct,
        });
    }

    // Compute summary totals from monthly values
    let total_revenue_idr = months.iter().map(|m| m.revenue_idr).sum();
    let total_cost_idr = months.iter().map(|m| m.total_cost_idr).sum();
    let gross_profit_idr = months.iter().map(|m| m.gross_profit_idr).sum();

    // Overall margin percentage
    let margin_pct = if total_revenue_idr > 0 {
        (gross_profit_idr as f64 / total_revenue_idr as f64) * 100.0
    } else {
        0.0
    };

    // Compute margin alert if deviation exceeds threshold (only when there's revenue)
    let margin_alert = if total_revenue_idr > 0
        && pl_settings.target_margin_pct - margin_pct > pl_settings.margin_alert_threshold_pct
    {
        Some(format!(
            "Margin below target: {:.0}% vs {:.0}% target",
            margin_pct, pl_settings.target_margin_pct
        ))
    } else {
        None
    };

    Ok(ProjectPlDashboardResult {
        project_id,
        year,
        total_revenue_idr,
        total_cost_idr,
        gross_profit_idr,
        margin_pct,
        target_margin_pct: pl_settings.target_margin_pct,
        margin_alert_threshold_pct: pl_settings.margin_alert_threshold_pct,
        margin_alert,
        months,
    })
}

// ── Helper: Fetch monthly expenses ───────────────────────────────────────────

async fn fetch_monthly_expenses(
    pool: &PgPool,
    project_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<(u32, i64)>> {
    let rows = sqlx::query(
        r#"SELECT DATE_TRUNC('month', expense_date)::DATE as expense_month,
                  SUM(amount_idr)::BIGINT as total_idr
           FROM project_expenses
           WHERE project_id = $1
             AND expense_date >= $2 AND expense_date < $3
           GROUP BY DATE_TRUNC('month', expense_date)
           ORDER BY expense_month"#,
    )
    .bind(project_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut result = Vec::new();
    for row in rows {
        let expense_month: NaiveDate = row
            .try_get("expense_month")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let total_idr: i64 = row
            .try_get("total_idr")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let month_num = expense_month.month();
        result.push((month_num, total_idr));
    }

    Ok(result)
}

// ── Helper: Fetch P&L settings ────────────────────────────────────────────────

struct PlSettings {
    target_margin_pct: f64,
    margin_alert_threshold_pct: f64,
}

async fn fetch_pl_settings(pool: &PgPool, project_id: Uuid) -> Result<PlSettings> {
    let row = sqlx::query(
        r#"SELECT target_margin_pct,
                  margin_alert_threshold_pct
           FROM projects
           WHERE id = $1"#,
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let target_margin_pct_bd: BigDecimal = row
        .try_get("target_margin_pct")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let margin_alert_threshold_pct_bd: BigDecimal = row
        .try_get("margin_alert_threshold_pct")
        .map_err(|e| AppError::Database(e.to_string()))?;

    let target_margin_pct = target_margin_pct_bd
        .to_string()
        .parse::<f64>()
        .map_err(|e| AppError::Database(format!("failed to parse target_margin_pct: {}", e)))?;
    let margin_alert_threshold_pct = margin_alert_threshold_pct_bd
        .to_string()
        .parse::<f64>()
        .map_err(|e| {
            AppError::Database(format!("failed to parse margin_alert_threshold_pct: {}", e))
        })?;

    Ok(PlSettings {
        target_margin_pct,
        margin_alert_threshold_pct,
    })
}

// ── Forecast Result Types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProjectPlForecastResult {
    pub project_id: Uuid,
    pub year: i32,
    pub as_of_date: NaiveDate,
    pub elapsed_days: i64,
    pub total_project_days: i64,
    pub current_spend_idr: i64,
    pub current_revenue_idr: i64,
    pub burn_rate_idr_per_day: f64,
    pub projected_total_cost_idr: i64,
    pub remaining_cost_projection_idr: i64,
    pub forecast_margin_pct: f64,
    pub target_margin_pct: f64,
    pub variance_from_target_pct: f64,
    pub categories: Vec<ForecastCategoryEntry>,
    pub resource_drivers: Vec<ForecastResourceDriver>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForecastCategoryEntry {
    pub category: String,
    pub budget_idr: i64,
    pub current_spend_idr: i64,
    pub projected_idr: i64,
    pub overrun_idr: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForecastResourceDriver {
    pub resource_name: String,
    pub total_cost_idr: i64,
    pub share_pct: f64,
}

// ── Forecast Service ─────────────────────────────────────────────────────────

/// Compute profitability forecast for a project based on current burn rate.
///
/// Canonical formulas:
///   current_spend = resource_cost + non_resource_cost (up to as_of)
///   burn_rate = current_spend / max(elapsed_days, 1)
///   projected_total_cost = round(burn_rate * total_project_days)
///   remaining_cost = projected_total_cost - current_spend
///   forecast_margin = if revenue > 0 { (revenue - projected_total_cost) / revenue * 100 } else { 0 }
///   variance_from_target = forecast_margin - target_margin
pub async fn get_project_pl_forecast(
    pool: &PgPool,
    project_id: Uuid,
    year: i32,
    as_of: Option<NaiveDate>,
) -> Result<ProjectPlForecastResult> {
    // Fetch project timeline, budget, and P&L settings
    let project_row = sqlx::query(
        r#"SELECT start_date, end_date,
                  total_budget_idr, budget_hr_idr, budget_software_idr,
                  budget_hardware_idr, budget_overhead_idr,
                  target_margin_pct
           FROM projects WHERE id = $1"#,
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let proj_start: NaiveDate = project_row
        .try_get("start_date")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let proj_end: NaiveDate = project_row
        .try_get("end_date")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let _total_budget_idr: i64 = project_row
        .try_get("total_budget_idr")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let budget_hr_idr: i64 = project_row
        .try_get("budget_hr_idr")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let budget_software_idr: i64 = project_row
        .try_get("budget_software_idr")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let budget_hardware_idr: i64 = project_row
        .try_get("budget_hardware_idr")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let budget_overhead_idr: i64 = project_row
        .try_get("budget_overhead_idr")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let target_margin_pct_bd: BigDecimal = project_row
        .try_get("target_margin_pct")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let target_margin_pct = target_margin_pct_bd
        .to_string()
        .parse::<f64>()
        .map_err(|e| AppError::Database(format!("failed to parse target_margin_pct: {}", e)))?;

    // Determine as_of_date, clamped to project range
    let raw_as_of = as_of.unwrap_or_else(|| Utc::now().date_naive());
    let as_of_date = raw_as_of.clamp(proj_start, proj_end);

    // Elapsed and total project days (inclusive, guard against zero)
    let elapsed_days = (as_of_date - proj_start).num_days() + 1; // inclusive
    let total_project_days = (proj_end - proj_start).num_days() + 1; // inclusive
    let elapsed_days = std::cmp::max(elapsed_days, 1);
    let total_project_days = std::cmp::max(total_project_days, 1);

    NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| AppError::Validation("Invalid year".to_string()))?;
    let expense_end_exclusive = as_of_date
        .succ_opt()
        .ok_or_else(|| AppError::Validation("Invalid as_of date".to_string()))?;

    // Fetch all data sources in parallel
    let (revenue_grid, resource_costs, expense_by_category) = tokio::try_join!(
        get_revenue_grid(pool, project_id, year),
        compute_project_resource_costs_in_window(pool, project_id, proj_start, as_of_date),
        fetch_expenses_by_category(pool, project_id, proj_start, expense_end_exclusive)
    )?;

    // Current revenue: sum revenue months up to as_of month
    let as_of_month = as_of_date.month();
    let as_of_year = as_of_date.year();
    let current_revenue_idr: i64 = revenue_grid
        .months
        .iter()
        .enumerate()
        .filter(|(idx, _)| {
            let m = (*idx as u32) + 1;
            year < as_of_year || (year == as_of_year && m <= as_of_month)
        })
        .map(|(_, m)| m.amount_idr)
        .sum();

    let current_resource_cost_idr = resource_costs.total_resource_cost_idr;

    // Current non-resource cost from expense_by_category totals
    let current_non_resource_cost_idr: i64 =
        expense_by_category.iter().map(|(_, total)| *total).sum();

    // Canonical formulas
    let current_spend_idr = current_resource_cost_idr + current_non_resource_cost_idr;
    let burn_rate_idr_per_day = current_spend_idr as f64 / elapsed_days as f64;
    let projected_total_cost_idr =
        (burn_rate_idr_per_day * total_project_days as f64).round() as i64;
    let remaining_cost_projection_idr = projected_total_cost_idr - current_spend_idr;

    // Forecast margin
    let forecast_margin_pct = if current_revenue_idr > 0 {
        ((current_revenue_idr - projected_total_cost_idr) as f64 / current_revenue_idr as f64)
            * 100.0
    } else {
        0.0
    };
    let variance_from_target_pct = forecast_margin_pct - target_margin_pct;

    // Category forecast: proportional projection based on current spend share
    let category_budgets = [
        ("hr", budget_hr_idr),
        ("software", budget_software_idr),
        ("hardware", budget_hardware_idr),
        ("overhead", budget_overhead_idr),
    ];

    // HR category includes resource costs + hr expenses
    let hr_expense_spend: i64 = expense_by_category
        .iter()
        .filter(|(cat, _)| cat == "hr")
        .map(|(_, total)| *total)
        .sum();

    let categories: Vec<ForecastCategoryEntry> = category_budgets
        .iter()
        .map(|(cat, budget)| {
            let cat_current_spend = if *cat == "hr" {
                // HR category = resource costs + hr expenses
                current_resource_cost_idr + hr_expense_spend
            } else {
                expense_by_category
                    .iter()
                    .filter(|(c, _)| c == cat)
                    .map(|(_, total)| *total)
                    .sum()
            };

            // Proportional projection: project current category spend at same rate
            let cat_projected = if elapsed_days > 0 {
                ((cat_current_spend as f64 / elapsed_days as f64) * total_project_days as f64)
                    .round() as i64
            } else {
                0
            };

            let overrun = std::cmp::max(cat_projected - budget, 0);

            ForecastCategoryEntry {
                category: cat.to_string(),
                budget_idr: *budget,
                current_spend_idr: cat_current_spend,
                projected_idr: cat_projected,
                overrun_idr: overrun,
            }
        })
        .collect();

    // Resource drivers: top 5 by cost share
    let total_resource_cost = resource_costs.total_resource_cost_idr;
    let mut resource_drivers: Vec<ForecastResourceDriver> = resource_costs
        .employees
        .iter()
        .map(|e| {
            let share_pct = if total_resource_cost > 0 {
                (e.total_cost_idr as f64 / total_resource_cost as f64) * 100.0
            } else {
                0.0
            };
            ForecastResourceDriver {
                resource_name: e.resource_name.clone(),
                total_cost_idr: e.total_cost_idr,
                share_pct,
            }
        })
        .collect();
    resource_drivers.sort_by(|a, b| b.total_cost_idr.cmp(&a.total_cost_idr));
    resource_drivers.truncate(5);

    Ok(ProjectPlForecastResult {
        project_id,
        year,
        as_of_date,
        elapsed_days,
        total_project_days,
        current_spend_idr,
        current_revenue_idr,
        burn_rate_idr_per_day,
        projected_total_cost_idr,
        remaining_cost_projection_idr,
        forecast_margin_pct,
        target_margin_pct,
        variance_from_target_pct,
        categories,
        resource_drivers,
    })
}

// ── Helper: Fetch expenses grouped by category ─────────────────────────────

async fn fetch_expenses_by_category(
    pool: &PgPool,
    project_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<(String, i64)>> {
    let rows = sqlx::query(
        r#"SELECT category, SUM(amount_idr)::BIGINT as total_idr
           FROM project_expenses
           WHERE project_id = $1
             AND expense_date >= $2 AND expense_date < $3
           GROUP BY category
           ORDER BY category"#,
    )
    .bind(project_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut result = Vec::new();
    for row in rows {
        let category: String = row
            .try_get("category")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let total_idr: i64 = row
            .try_get("total_idr")
            .map_err(|e| AppError::Database(e.to_string()))?;
        result.push((category, total_idr));
    }

    Ok(result)
}
