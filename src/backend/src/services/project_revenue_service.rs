use chrono::{DateTime, Datelike, NaiveDate, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::routes::project::{IngestErpRevenueRequest, UpsertProjectRevenueRequest};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProjectRevenueRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub revenue_month: NaiveDate,
    pub amount_idr: i64,
    pub source_type: String,
    pub source_reference: Option<String>,
    pub entered_by: Option<Uuid>,
    pub entry_date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MonthRevenueEntry {
    pub month: u32,
    pub month_label: String,
    pub revenue_id: Option<Uuid>,
    pub amount_idr: i64,
    pub source_type: Option<String>,
    pub source_reference: Option<String>,
    pub entered_by: Option<Uuid>,
    pub entry_date: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct ProjectRevenueGridResult {
    pub project_id: Uuid,
    pub year: i32,
    pub months: Vec<MonthRevenueEntry>,
    pub ytd_total_idr: i64,
}

fn parse_revenue_month(input: &str) -> Result<NaiveDate> {
    let trimmed = input.trim();
    if trimmed.len() != 7 || trimmed.chars().nth(4) != Some('-') {
        return Err(AppError::Validation(
            "revenue_month must use YYYY-MM format".into(),
        ));
    }

    NaiveDate::parse_from_str(&format!("{}-01", trimmed), "%Y-%m-%d")
        .map_err(|_| AppError::Validation("revenue_month must use YYYY-MM format".into()))
}

fn validate_non_negative_amount(amount_idr: i64) -> Result<()> {
    if amount_idr < 0 {
        return Err(AppError::Validation(
            "amount_idr must be a non-negative integer".into(),
        ));
    }
    Ok(())
}

pub async fn upsert_project_revenue(
    pool: &PgPool,
    project_id: Uuid,
    user_id: Uuid,
    req: &UpsertProjectRevenueRequest,
) -> Result<ProjectRevenueRow> {
    let revenue_month = parse_revenue_month(&req.revenue_month)?;
    validate_non_negative_amount(req.amount_idr)?;

    let existing = sqlx::query(
        r#"SELECT source_type
           FROM project_revenues
           WHERE project_id = $1 AND revenue_month = $2"#,
    )
    .bind(project_id)
    .bind(revenue_month)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let source_type = if let Some(row) = existing {
        let existing_source_type: String = row
            .try_get("source_type")
            .map_err(|e| AppError::Database(e.to_string()))?;

        if existing_source_type == "erp_synced" {
            if !req.override_erp {
                return Err(AppError::Validation(
                    "Must set override_erp=true to overwrite ERP-synced value".into(),
                ));
            }
            "manual_override"
        } else {
            "manual"
        }
    } else {
        "manual"
    };

    let entry_date = Utc::now().date_naive();
    let row = sqlx::query_as::<_, ProjectRevenueRow>(
        r#"INSERT INTO project_revenues (project_id, revenue_month, amount_idr, source_type, source_reference, entered_by, entry_date)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           ON CONFLICT (project_id, revenue_month) DO UPDATE SET
               amount_idr = EXCLUDED.amount_idr,
               source_type = EXCLUDED.source_type,
               source_reference = EXCLUDED.source_reference,
               entered_by = EXCLUDED.entered_by,
               entry_date = EXCLUDED.entry_date,
               updated_at = CURRENT_TIMESTAMP
           RETURNING id, project_id, revenue_month, amount_idr, source_type, source_reference, entered_by, entry_date, created_at, updated_at"#,
    )
    .bind(project_id)
    .bind(revenue_month)
    .bind(req.amount_idr)
    .bind(source_type)
    .bind(req.source_reference.clone())
    .bind(user_id)
    .bind(entry_date)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(row)
}

pub async fn get_revenue_grid(
    pool: &PgPool,
    project_id: Uuid,
    year: i32,
) -> Result<ProjectRevenueGridResult> {
    let start_date = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| AppError::Validation("Invalid year".into()))?;
    let end_date = NaiveDate::from_ymd_opt(year + 1, 1, 1)
        .ok_or_else(|| AppError::Validation("Invalid year".into()))?;

    let rows = sqlx::query_as::<_, ProjectRevenueRow>(
        r#"SELECT id, project_id, revenue_month, amount_idr, source_type, source_reference, entered_by, entry_date, created_at, updated_at
           FROM project_revenues
           WHERE project_id = $1
             AND revenue_month >= $2
             AND revenue_month < $3
           ORDER BY revenue_month ASC"#,
    )
    .bind(project_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut month_rows: [Option<ProjectRevenueRow>; 12] = [
        None, None, None, None, None, None, None, None, None, None, None, None,
    ];

    let mut ytd_total_idr = 0i64;
    for row in rows {
        let month_idx = row.revenue_month.month0() as usize;
        ytd_total_idr += row.amount_idr;
        month_rows[month_idx] = Some(row);
    }

    let month_labels = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let months = month_rows
        .into_iter()
        .enumerate()
        .map(|(idx, row_opt)| {
            if let Some(row) = row_opt {
                MonthRevenueEntry {
                    month: (idx as u32) + 1,
                    month_label: month_labels[idx].to_string(),
                    revenue_id: Some(row.id),
                    amount_idr: row.amount_idr,
                    source_type: Some(row.source_type),
                    source_reference: row.source_reference,
                    entered_by: row.entered_by,
                    entry_date: Some(row.entry_date),
                }
            } else {
                MonthRevenueEntry {
                    month: (idx as u32) + 1,
                    month_label: month_labels[idx].to_string(),
                    revenue_id: None,
                    amount_idr: 0,
                    source_type: None,
                    source_reference: None,
                    entered_by: None,
                    entry_date: None,
                }
            }
        })
        .collect();

    Ok(ProjectRevenueGridResult {
        project_id,
        year,
        months,
        ytd_total_idr,
    })
}

pub async fn ingest_erp_revenue(
    pool: &PgPool,
    project_id: Uuid,
    user_id: Uuid,
    req: &IngestErpRevenueRequest,
    idempotency_key: Option<&str>,
) -> Result<ProjectRevenueRow> {
    let revenue_month = parse_revenue_month(&req.revenue_month)?;
    validate_non_negative_amount(req.amount_idr)?;

    let source_reference = req.source_reference.trim();
    if source_reference.is_empty() {
        return Err(AppError::Validation(
            "source_reference is required for ERP ingest".into(),
        ));
    }

    let normalized_idempotency_key = idempotency_key
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(key) = normalized_idempotency_key {
        let source_prefix_like = format!("erp:{}:%", key);
        let existing_by_key = sqlx::query_as::<_, ProjectRevenueRow>(
            r#"SELECT id, project_id, revenue_month, amount_idr, source_type, source_reference, entered_by, entry_date, created_at, updated_at
               FROM project_revenues
               WHERE project_id = $1
                 AND revenue_month = $2
                 AND source_reference LIKE $3"#,
        )
        .bind(project_id)
        .bind(revenue_month)
        .bind(source_prefix_like)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(existing_row) = existing_by_key {
            return Ok(existing_row);
        }
    }

    let normalized_source_reference = if let Some(key) = normalized_idempotency_key {
        format!("erp:{}:{}", key, source_reference)
    } else {
        source_reference.to_string()
    };

    let existing = sqlx::query_as::<_, ProjectRevenueRow>(
        r#"SELECT id, project_id, revenue_month, amount_idr, source_type, source_reference, entered_by, entry_date, created_at, updated_at
           FROM project_revenues
           WHERE project_id = $1 AND revenue_month = $2"#,
    )
    .bind(project_id)
    .bind(revenue_month)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some(existing_row) = existing {
        if existing_row.source_type == "manual" || existing_row.source_type == "manual_override" {
            return Ok(existing_row);
        }
    }

    let entry_date = Utc::now().date_naive();
    let row = sqlx::query_as::<_, ProjectRevenueRow>(
        r#"INSERT INTO project_revenues (project_id, revenue_month, amount_idr, source_type, source_reference, entered_by, entry_date)
           VALUES ($1, $2, $3, 'erp_synced', $4, $5, $6)
           ON CONFLICT (project_id, revenue_month) DO UPDATE SET
               amount_idr = EXCLUDED.amount_idr,
               source_type = EXCLUDED.source_type,
               source_reference = EXCLUDED.source_reference,
               entered_by = EXCLUDED.entered_by,
               entry_date = EXCLUDED.entry_date,
               updated_at = CURRENT_TIMESTAMP
           RETURNING id, project_id, revenue_month, amount_idr, source_type, source_reference, entered_by, entry_date, created_at, updated_at"#,
    )
    .bind(project_id)
    .bind(revenue_month)
    .bind(req.amount_idr)
    .bind(normalized_source_reference)
    .bind(user_id)
    .bind(entry_date)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(row)
}
