//! CTC completeness reporting service.

use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::error::{AppError, Result};

#[derive(Debug, Serialize)]
pub struct DepartmentCompleteness {
    pub department_id: Uuid,
    pub department: String,
    pub total_employees: i64,
    pub with_ctc: i64,
    pub missing_ctc: i64,
    pub completion_pct: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct CompletenessTrendPoint {
    /// Stable `YYYY-MM` month bucket key, ascending order in the trend vector.
    pub month: String,
    /// Employees that existed by the last day of the month (resource_type = 'employee',
    /// resources.created_at in UTC <= month_end), bounded by the optional department filter.
    /// Legacy rows with NULL created_at are counted from the current anchor month only.
    pub total_employees: i64,
    /// Distinct employees with an Active CTC record whose effective_date <= month_end.
    /// The current-month bucket is capped at today's date, not the future month end.
    pub total_with_ctc: i64,
    /// Convenience: employees missing CTC as of month end. Always
    /// `total_employees - total_with_ctc` and never negative.
    pub total_missing: i64,
    /// `total_with_ctc / total_employees * 100`, or `0.0` when no employees.
    pub completion_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct CompletenessReport {
    pub departments: Vec<DepartmentCompleteness>,
    pub total_employees: i64,
    pub total_with_ctc: i64,
    pub total_missing: i64,
    pub overall_completion_pct: f64,
    /// Last 12 months of monthly completeness. Empty if the data window is empty.
    /// The "with CTC" trend signal uses currently-Active ctc_records effective by
    /// month end, capped at today's date for the current month; current summary/missing
    /// counts treat any Active CTC as complete. Historical revisions and department
    /// assignment history are not replayed for this story.
    #[serde(default)]
    pub trend: Vec<CompletenessTrendPoint>,
}

#[derive(Debug, Serialize)]
pub struct MissingCtcEmployee {
    pub id: Uuid,
    pub name: String,
    pub department: String,
}

/// Default number of months exposed in the completeness trend.
pub const COMPLETENESS_TREND_DEFAULT_MONTHS: u32 = 12;
/// Hard upper bound: guards against runaway expansion if a future route accepts a typed range.
pub const COMPLETENESS_TREND_MAX_MONTHS: u32 = 24;

fn percent(numerator: i64, denominator: i64) -> f64 {
    if denominator <= 0 {
        return 0.0;
    }
    (numerator as f64 / denominator as f64) * 100.0
}

pub async fn get_completeness_summary(
    pool: &PgPool,
    department_id: Option<Uuid>,
) -> Result<CompletenessReport> {
    let rows = fetch_department_rows(pool, department_id).await?;
    let mut report = build_completeness_report(rows)?;
    let trend = fetch_completeness_trend(pool, department_id, today_naive()).await?;
    report.trend = trend;
    Ok(report)
}

pub async fn get_completeness_summary_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Option<Uuid>,
) -> Result<CompletenessReport> {
    let mut report = get_completeness_summary_core_in_transaction(tx, department_id).await?;
    let trend = get_completeness_trend_in_transaction(tx, department_id).await?;
    report.trend = trend;
    Ok(report)
}

pub async fn get_completeness_summary_core_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Option<Uuid>,
) -> Result<CompletenessReport> {
    let rows = fetch_department_rows_in_tx(tx, department_id).await?;
    build_completeness_report(rows)
}

pub async fn get_completeness_trend_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Option<Uuid>,
) -> Result<Vec<CompletenessTrendPoint>> {
    fetch_completeness_trend_in_tx(tx, department_id, today_naive()).await
}

async fn fetch_department_rows(pool: &PgPool, department_id: Option<Uuid>) -> Result<Vec<PgRow>> {
    sqlx::query(DEPARTMENT_COMPLETENESS_SQL)
        .bind(department_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

async fn fetch_department_rows_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Option<Uuid>,
) -> Result<Vec<PgRow>> {
    sqlx::query(DEPARTMENT_COMPLETENESS_SQL)
        .bind(department_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

const DEPARTMENT_COMPLETENESS_SQL: &str = r#"
    -- Current completeness is Active-only: a future-dated Active CTC counts
    -- complete here, while TREND_SQL waits for effective_date <= month_end.
    WITH active_ctc AS (
        SELECT resource_id
        FROM ctc_records
        WHERE status = 'Active'
        GROUP BY resource_id
    )
    SELECT
        d.id AS department_id,
        d.name AS department,
        COUNT(DISTINCT r.id) AS total_employees,
        COUNT(DISTINCT c.resource_id) AS with_ctc
    FROM departments d
    LEFT JOIN resources r
        ON r.department_id = d.id
       AND r.resource_type = 'employee'
    LEFT JOIN active_ctc c
        ON c.resource_id = r.id
    WHERE ($1::uuid IS NULL OR d.id = $1)
    GROUP BY d.id, d.name
    ORDER BY d.name
"#;

fn build_completeness_report(rows: Vec<PgRow>) -> Result<CompletenessReport> {
    let mut departments = Vec::with_capacity(rows.len());
    let mut total_employees = 0i64;
    let mut total_with_ctc = 0i64;

    for row in rows {
        let total_department: i64 = row
            .try_get::<i64, _>("total_employees")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let with_ctc: i64 = row
            .try_get::<i64, _>("with_ctc")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let missing_ctc = total_department.saturating_sub(with_ctc);

        total_employees += total_department;
        total_with_ctc += with_ctc;

        departments.push(DepartmentCompleteness {
            department_id: row
                .try_get::<Uuid, _>("department_id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            department: row
                .try_get::<String, _>("department")
                .map_err(|e| AppError::Database(e.to_string()))?,
            total_employees: total_department,
            with_ctc,
            missing_ctc,
            completion_pct: percent(with_ctc, total_department),
        });
    }

    let total_missing = total_employees.saturating_sub(total_with_ctc);

    Ok(CompletenessReport {
        departments,
        total_employees,
        total_with_ctc,
        total_missing,
        overall_completion_pct: percent(total_with_ctc, total_employees),
        trend: Vec::new(),
    })
}

pub async fn get_missing_employees(
    pool: &PgPool,
    department_id: Option<Uuid>,
) -> Result<Vec<MissingCtcEmployee>> {
    let rows = sqlx::query(MISSING_EMPLOYEES_SQL)
        .bind(department_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    build_missing_employees(rows)
}

pub async fn get_missing_employees_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Option<Uuid>,
) -> Result<Vec<MissingCtcEmployee>> {
    let rows = sqlx::query(MISSING_EMPLOYEES_SQL)
        .bind(department_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    build_missing_employees(rows)
}

const MISSING_EMPLOYEES_SQL: &str = r#"
    -- Current missing-list semantics mirror the summary: any Active CTC
    -- removes the employee, regardless of future effective_date.
    WITH active_ctc AS (
        SELECT resource_id
        FROM ctc_records
        WHERE status = 'Active'
        GROUP BY resource_id
    )
    SELECT
        r.id,
        r.name,
        d.name AS department
    FROM resources r
    JOIN departments d ON d.id = r.department_id
    LEFT JOIN active_ctc c
        ON c.resource_id = r.id
    WHERE r.resource_type = 'employee'
      AND c.resource_id IS NULL
      AND ($1::uuid IS NULL OR d.id = $1)
    ORDER BY d.name, r.name
"#;

fn build_missing_employees(rows: Vec<PgRow>) -> Result<Vec<MissingCtcEmployee>> {
    let mut employees = Vec::with_capacity(rows.len());
    for row in rows {
        employees.push(MissingCtcEmployee {
            id: row
                .try_get::<Uuid, _>("id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            name: row
                .try_get::<String, _>("name")
                .map_err(|e| AppError::Database(e.to_string()))?,
            department: row
                .try_get::<String, _>("department")
                .map_err(|e| AppError::Database(e.to_string()))?,
        });
    }

    Ok(employees)
}

// -------------------------------------------------------------------------
// Monthly completeness trend
// -------------------------------------------------------------------------

fn today_naive() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

/// Build the ascending list of last `COMPLETENESS_TREND_DEFAULT_MONTHS` month-end dates
/// up to and including the month of `anchor`. Bounded by `COMPLETENESS_TREND_MAX_MONTHS`.
pub fn completeness_trend_month_ends(anchor: NaiveDate, months: u32) -> Vec<NaiveDate> {
    let months = months.clamp(1, COMPLETENESS_TREND_MAX_MONTHS) as i32;
    let mut out = Vec::with_capacity(months as usize);
    for offset in (0..months).rev() {
        let (year, month0) = subtract_months(anchor.year(), anchor.month0() as i32, offset);
        out.push(month_end(year, month0 as u32));
    }
    out
}

fn subtract_months(year: i32, month0: i32, delta: i32) -> (i32, i32) {
    let total = year * 12 + month0 - delta;
    let new_year = total.div_euclid(12);
    let new_month0 = total.rem_euclid(12);
    (new_year, new_month0)
}

fn month_end(year: i32, month0: u32) -> NaiveDate {
    let month = month0 + 1; // chrono months are 1-based
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_of_next = NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, month, 28).expect("valid month"));
    first_of_next
        .pred_opt()
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, month, 1).expect("valid month"))
}

fn month_key(date: NaiveDate) -> String {
    format!("{:04}-{:02}", date.year(), date.month())
}

async fn fetch_completeness_trend(
    pool: &PgPool,
    department_id: Option<Uuid>,
    anchor: NaiveDate,
) -> Result<Vec<CompletenessTrendPoint>> {
    let month_ends = completeness_trend_month_ends(anchor, COMPLETENESS_TREND_DEFAULT_MONTHS);
    let rows = sqlx::query(TREND_SQL)
        .bind(department_id)
        .bind(&month_ends)
        .bind(anchor)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    build_trend_points(month_ends, rows)
}

async fn fetch_completeness_trend_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Option<Uuid>,
    anchor: NaiveDate,
) -> Result<Vec<CompletenessTrendPoint>> {
    let month_ends = completeness_trend_month_ends(anchor, COMPLETENESS_TREND_DEFAULT_MONTHS);
    let rows = sqlx::query(TREND_SQL)
        .bind(department_id)
        .bind(&month_ends)
        .bind(anchor)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    build_trend_points(month_ends, rows)
}

const TREND_SQL: &str = r#"
    WITH active_ctc AS (
        SELECT
            resource_id,
            MIN(effective_date) AS effective_date
        FROM ctc_records
        WHERE status = 'Active'
        GROUP BY resource_id
    )
    SELECT
        m.month_end::DATE AS month_end,
        COALESCE(SUM(CASE
            WHEN r.id IS NOT NULL
                 AND COALESCE((r.created_at AT TIME ZONE 'UTC')::DATE, $3::date) <= LEAST(m.month_end::DATE, $3::date)
            THEN 1
            ELSE 0
        END), 0)::BIGINT AS total_employees,
        COALESCE(SUM(CASE
            WHEN c.resource_id IS NOT NULL
                 AND c.effective_date <= LEAST(m.month_end::DATE, $3::date)
                 AND COALESCE((r.created_at AT TIME ZONE 'UTC')::DATE, $3::date) <= LEAST(m.month_end::DATE, $3::date)
            THEN 1
            ELSE 0
        END), 0)::BIGINT AS total_with_ctc
    FROM UNNEST($2::date[]) AS m(month_end)
    LEFT JOIN resources r
        ON r.resource_type = 'employee'
       AND r.department_id IS NOT NULL
       AND ($1::uuid IS NULL OR r.department_id = $1)
    LEFT JOIN active_ctc c
        ON c.resource_id = r.id
    GROUP BY m.month_end
    ORDER BY m.month_end ASC
"#;

fn build_trend_points(
    month_ends: Vec<NaiveDate>,
    rows: Vec<PgRow>,
) -> Result<Vec<CompletenessTrendPoint>> {
    // Map DB rows by their month_end date for deterministic alignment with the requested window.
    let mut by_month_end: std::collections::HashMap<NaiveDate, (i64, i64)> =
        std::collections::HashMap::with_capacity(rows.len());
    for row in rows {
        let month_end: NaiveDate = row
            .try_get::<NaiveDate, _>("month_end")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let total_employees: i64 = row
            .try_get::<i64, _>("total_employees")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let total_with_ctc: i64 = row
            .try_get::<i64, _>("total_with_ctc")
            .map_err(|e| AppError::Database(e.to_string()))?;
        by_month_end.insert(month_end, (total_employees, total_with_ctc));
    }

    let mut out = Vec::with_capacity(month_ends.len());
    for end in month_ends {
        let (total_employees, total_with_ctc) = by_month_end.get(&end).copied().unwrap_or((0, 0));
        let total_with_ctc = total_with_ctc.min(total_employees);
        let total_missing = total_employees.saturating_sub(total_with_ctc);
        out.push(CompletenessTrendPoint {
            month: month_key(end),
            total_employees,
            total_with_ctc,
            total_missing,
            completion_pct: (percent(total_with_ctc, total_employees) * 10.0).round() / 10.0,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_with_zero_denominator() {
        assert_eq!(percent(5, 0), 0.0);
    }

    #[test]
    fn test_percent_with_values() {
        assert_eq!(percent(5, 10), 50.0);
    }

    #[test]
    fn trend_month_ends_default_returns_12_buckets_ending_with_anchor_month() {
        let anchor = NaiveDate::from_ymd_opt(2026, 5, 22).unwrap();
        let ends = completeness_trend_month_ends(anchor, COMPLETENESS_TREND_DEFAULT_MONTHS);
        assert_eq!(ends.len(), 12);
        // First bucket is June 2025 (12 months ago), last is May 2026.
        assert_eq!(month_key(ends[0]), "2025-06");
        assert_eq!(month_key(ends[11]), "2026-05");
        // Last day of May 2026 is 31.
        assert_eq!(ends[11], NaiveDate::from_ymd_opt(2026, 5, 31).unwrap());
    }

    #[test]
    fn trend_month_ends_handles_year_boundary() {
        let anchor = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let ends = completeness_trend_month_ends(anchor, 3);
        assert_eq!(ends.len(), 3);
        assert_eq!(month_key(ends[0]), "2025-11");
        assert_eq!(month_key(ends[1]), "2025-12");
        assert_eq!(month_key(ends[2]), "2026-01");
    }

    #[test]
    fn trend_month_ends_clamps_to_max_months() {
        let anchor = NaiveDate::from_ymd_opt(2026, 5, 22).unwrap();
        let ends = completeness_trend_month_ends(anchor, 99);
        assert_eq!(ends.len(), COMPLETENESS_TREND_MAX_MONTHS as usize);
    }

    #[test]
    fn trend_month_ends_clamps_to_minimum_one() {
        let anchor = NaiveDate::from_ymd_opt(2026, 5, 22).unwrap();
        let ends = completeness_trend_month_ends(anchor, 0);
        assert_eq!(ends.len(), 1);
        assert_eq!(month_key(ends[0]), "2026-05");
    }

    #[test]
    fn trend_point_clamps_with_ctc_to_total_employees() {
        let ends = vec![NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()];
        let points = build_trend_points(ends, Vec::new()).expect("build trend");
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].month, "2026-05");
        assert_eq!(points[0].total_employees, 0);
        assert_eq!(points[0].total_with_ctc, 0);
        assert_eq!(points[0].total_missing, 0);
        assert_eq!(points[0].completion_pct, 0.0);
    }

    /// Story 6.5 defense-in-depth: month buckets where DB returns no row
    /// fall back to (0, 0); the alignment is by `NaiveDate` lookup, not by
    /// row order. The fallback path also yields `total_missing = 0`,
    /// `completion_pct = 0.0` (never NaN/Inf), and the bucket keeps the
    /// requested month_end's `YYYY-MM` key.
    #[test]
    fn trend_point_aligns_by_month_end_and_zero_fills_missing_rows() {
        let april = NaiveDate::from_ymd_opt(2026, 4, 30).unwrap();
        let may = NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();
        // Pass only the May window; the DB-rows vector is empty so the
        // requested April bucket is zero-filled rather than dropped.
        let points = build_trend_points(vec![april, may], Vec::new()).expect("build trend");
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].month, "2026-04");
        assert_eq!(points[0].total_employees, 0);
        assert_eq!(points[0].total_with_ctc, 0);
        assert_eq!(points[0].total_missing, 0);
        assert_eq!(points[0].completion_pct, 0.0);
        assert_eq!(points[1].month, "2026-05");
    }
}
