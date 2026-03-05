use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::project_cost_service::compute_project_resource_costs;
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
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
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
        let revenue_idr = revenue_grid.months
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
    let margin_alert = if total_revenue_idr > 0 && pl_settings.target_margin_pct - margin_pct > pl_settings.margin_alert_threshold_pct {
        Some(format!(
            "Margin below target: {:.0}% vs {:.0}% target",
            margin_pct,
            pl_settings.target_margin_pct
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
        r#"SELECT target_margin_pct::FLOAT8 as target_margin_pct,
                  margin_alert_threshold_pct::FLOAT8 as margin_alert_threshold_pct
           FROM projects
           WHERE id = $1"#,
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let target_margin_pct: f64 = row
        .try_get("target_margin_pct")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let margin_alert_threshold_pct: f64 = row
        .try_get("margin_alert_threshold_pct")
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(PlSettings {
        target_margin_pct,
        margin_alert_threshold_pct,
    })
}
