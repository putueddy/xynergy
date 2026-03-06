use sqlx::PgPool;
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
