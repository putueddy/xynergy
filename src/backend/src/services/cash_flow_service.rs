use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{AppError, Result};

const CASH_IN_CATEGORIES: &[&str] = &["client_payment", "interest", "other_income"];
const CASH_OUT_CATEGORIES: &[&str] = &["payroll", "vendor_payment", "expense", "tax"];

pub fn validate_cash_flow_entry(
    entry_type: &str,
    category: &str,
    amount_idr: i64,
    description: &str,
) -> Result<()> {
    if !matches!(entry_type, "cash_in" | "cash_out") {
        return Err(AppError::Validation(
            "entry_type must be either 'cash_in' or 'cash_out'".into(),
        ));
    }

    let valid_category = match entry_type {
        "cash_in" => CASH_IN_CATEGORIES.contains(&category),
        "cash_out" => CASH_OUT_CATEGORIES.contains(&category),
        _ => false,
    };

    if !valid_category {
        let allowed = if entry_type == "cash_in" {
            "client_payment, interest, other_income"
        } else {
            "payroll, vendor_payment, expense, tax"
        };
        return Err(AppError::Validation(format!(
            "Invalid category '{}' for entry_type '{}'. Allowed categories: {}",
            category, entry_type, allowed
        )));
    }

    if amount_idr <= 0 {
        return Err(AppError::Validation(
            "amount_idr must be a positive integer".into(),
        ));
    }

    if description.trim().is_empty() {
        return Err(AppError::Validation("description must not be empty".into()));
    }

    Ok(())
}

pub async fn validate_project_exists(pool: &PgPool, project_id: Uuid) -> Result<()> {
    let exists = sqlx::query_scalar!("SELECT id FROM projects WHERE id = $1", project_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if exists.is_none() {
        return Err(AppError::NotFound(format!(
            "Project {} not found",
            project_id
        )));
    }

    Ok(())
}

// ── Dashboard Types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CashFlowDashboardFilters {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CashFlowDashboardResult {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub project_id: Option<Uuid>,
    pub total_cash_in_idr: i64,
    pub total_cash_out_idr: i64,
    pub net_cash_flow_idr: i64,
    pub ending_cumulative_position_idr: i64,
    pub months: Vec<CashFlowDashboardMonth>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CashFlowDashboardMonth {
    pub year: i32,
    pub month: u32,
    pub month_label: String,
    pub cash_in_idr: i64,
    pub cash_out_idr: i64,
    pub net_cash_flow_idr: i64,
    pub cumulative_position_idr: i64,
    pub entries: Vec<CashFlowDashboardEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CashFlowDashboardEntry {
    pub id: Uuid,
    pub entry_type: String,
    pub category: String,
    pub amount_idr: i64,
    pub entry_date: NaiveDate,
    pub description: String,
    pub project_id: Option<Uuid>,
}

// ── Dashboard Service ────────────────────────────────────────────────────────

const MONTH_LABELS: &[&str] = &[
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn start_of_month(date: NaiveDate) -> Result<NaiveDate> {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
        .ok_or_else(|| AppError::Validation("Invalid start month".to_string()))
}

fn end_of_month(date: NaiveDate) -> Result<NaiveDate> {
    let next_month_start = if date.month() == 12 {
        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1)
    }
    .ok_or_else(|| AppError::Validation("Invalid end month".to_string()))?;

    next_month_start
        .pred_opt()
        .ok_or_else(|| AppError::Validation("Invalid end month".to_string()))
}

pub async fn get_cash_flow_dashboard(
    pool: &PgPool,
    filters: CashFlowDashboardFilters,
) -> Result<CashFlowDashboardResult> {
    if filters.start_date > filters.end_date {
        return Err(AppError::Validation(
            "start_date must not be after end_date".into(),
        ));
    }

    if let Some(project_id) = filters.project_id {
        validate_project_exists(pool, project_id).await?;
    }

    let normalized_start_date = start_of_month(filters.start_date)?;
    let normalized_end_date = end_of_month(filters.end_date)?;

    let mut month_buckets: Vec<(i32, u32)> = Vec::new();
    let mut current_year = normalized_start_date.year();
    let mut current_month = normalized_start_date.month();

    loop {
        month_buckets.push((current_year, current_month));

        if current_year == normalized_end_date.year()
            && current_month == normalized_end_date.month()
        {
            break;
        }

        if current_month == 12 {
            current_month = 1;
            current_year += 1;
        } else {
            current_month += 1;
        }

        if month_buckets.len() > 1200 {
            return Err(AppError::Validation(
                "Date range too large (max 100 years)".into(),
            ));
        }
    }

    let monthly_agg_rows = sqlx::query(
        r#"SELECT DATE_TRUNC('month', entry_date)::DATE as month_start,
                  COALESCE(SUM(CASE WHEN entry_type = 'cash_in' THEN amount_idr ELSE 0 END), 0)::BIGINT as cash_in_idr,
                  COALESCE(SUM(CASE WHEN entry_type = 'cash_out' THEN amount_idr ELSE 0 END), 0)::BIGINT as cash_out_idr
           FROM cash_flow_entries
           WHERE entry_date >= $1 AND entry_date <= $2
             AND ($3::UUID IS NULL OR project_id = $3)
           GROUP BY DATE_TRUNC('month', entry_date)
           ORDER BY month_start"#,
    )
    .bind(normalized_start_date)
    .bind(normalized_end_date)
    .bind(filters.project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut agg_map = std::collections::HashMap::new();
    for row in &monthly_agg_rows {
        let month_start: NaiveDate = row
            .try_get("month_start")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let cash_in: i64 = row
            .try_get("cash_in_idr")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let cash_out: i64 = row
            .try_get("cash_out_idr")
            .map_err(|e| AppError::Database(e.to_string()))?;
        agg_map.insert(
            (month_start.year(), month_start.month()),
            (cash_in, cash_out),
        );
    }

    let detail_rows = sqlx::query(
        r#"SELECT id, entry_type, category, amount_idr, entry_date, description, project_id
           FROM cash_flow_entries
           WHERE entry_date >= $1 AND entry_date <= $2
             AND ($3::UUID IS NULL OR project_id = $3)
           ORDER BY entry_date DESC, created_at DESC"#,
    )
    .bind(normalized_start_date)
    .bind(normalized_end_date)
    .bind(filters.project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut entries_map: std::collections::HashMap<(i32, u32), Vec<CashFlowDashboardEntry>> =
        std::collections::HashMap::new();
    for row in &detail_rows {
        let entry_date: NaiveDate = row
            .try_get("entry_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let key = (entry_date.year(), entry_date.month());

        let entry = CashFlowDashboardEntry {
            id: row
                .try_get("id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            entry_type: row
                .try_get("entry_type")
                .map_err(|e| AppError::Database(e.to_string()))?,
            category: row
                .try_get("category")
                .map_err(|e| AppError::Database(e.to_string()))?,
            amount_idr: row
                .try_get("amount_idr")
                .map_err(|e| AppError::Database(e.to_string()))?,
            entry_date,
            description: row
                .try_get("description")
                .map_err(|e| AppError::Database(e.to_string()))?,
            project_id: row
                .try_get("project_id")
                .map_err(|e| AppError::Database(e.to_string()))?,
        };

        entries_map.entry(key).or_default().push(entry);
    }

    let mut months: Vec<CashFlowDashboardMonth> = Vec::with_capacity(month_buckets.len());
    let mut cumulative: i64 = 0;

    for &(year, month) in &month_buckets {
        let (cash_in, cash_out) = agg_map.get(&(year, month)).copied().unwrap_or((0, 0));
        let net = cash_in - cash_out;
        cumulative += net;

        let month_label = MONTH_LABELS
            .get((month - 1) as usize)
            .unwrap_or(&"???")
            .to_string();

        let entries = entries_map.remove(&(year, month)).unwrap_or_default();

        months.push(CashFlowDashboardMonth {
            year,
            month,
            month_label,
            cash_in_idr: cash_in,
            cash_out_idr: cash_out,
            net_cash_flow_idr: net,
            cumulative_position_idr: cumulative,
            entries,
        });
    }

    let total_cash_in: i64 = months.iter().map(|m| m.cash_in_idr).sum();
    let total_cash_out: i64 = months.iter().map(|m| m.cash_out_idr).sum();
    let net_cash_flow = total_cash_in - total_cash_out;
    let ending_cumulative = months
        .last()
        .map(|m| m.cumulative_position_idr)
        .unwrap_or(0);

    Ok(CashFlowDashboardResult {
        start_date: normalized_start_date,
        end_date: normalized_end_date,
        project_id: filters.project_id,
        total_cash_in_idr: total_cash_in,
        total_cash_out_idr: total_cash_out,
        net_cash_flow_idr: net_cash_flow,
        ending_cumulative_position_idr: ending_cumulative,
        months,
    })
}
