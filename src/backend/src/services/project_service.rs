use crate::error::{AppError, Result};

pub fn validate_project_budget(
    total_budget_idr: i64,
    budget_hr_idr: i64,
    budget_software_idr: i64,
    budget_hardware_idr: i64,
    budget_overhead_idr: i64,
) -> Result<()> {
    if total_budget_idr <= 0 {
        return Err(AppError::Validation(
            "total_budget_idr must be greater than 0".into(),
        ));
    }
    if budget_hr_idr < 0
        || budget_software_idr < 0
        || budget_hardware_idr < 0
        || budget_overhead_idr < 0
    {
        return Err(AppError::Validation(
            "Budget category values must be non-negative".into(),
        ));
    }
    let sum = budget_hr_idr + budget_software_idr + budget_hardware_idr + budget_overhead_idr;
    if sum != total_budget_idr {
        return Err(AppError::Validation(format!(
            "Budget categories sum ({}) does not equal total_budget_idr ({})",
            sum, total_budget_idr
        )));
    }
    Ok(())
}

const VALID_EXPENSE_CATEGORIES: &[&str] = &["hr", "software", "hardware", "overhead"];

/// Validate fields for creating a project expense.
pub fn validate_create_expense(category: &str, amount_idr: i64, description: &str) -> Result<()> {
    if !VALID_EXPENSE_CATEGORIES.contains(&category) {
        return Err(AppError::Validation(format!(
            "Invalid category '{}'. Must be one of: hr, software, hardware, overhead",
            category
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

/// Validate fields for updating a project expense.
/// At least one field besides edit_reason should change (not enforced here — just validates present values).
pub fn validate_update_expense(
    category: Option<&str>,
    description: Option<&str>,
    amount_idr: Option<i64>,
    edit_reason: &str,
) -> Result<()> {
    if edit_reason.trim().is_empty() {
        return Err(AppError::Validation("edit_reason must not be empty".into()));
    }
    if let Some(cat) = category {
        if !VALID_EXPENSE_CATEGORIES.contains(&cat) {
            return Err(AppError::Validation(format!(
                "Invalid category '{}'. Must be one of: hr, software, hardware, overhead",
                cat
            )));
        }
    }
    if let Some(desc) = description {
        if desc.trim().is_empty() {
            return Err(AppError::Validation("description must not be empty".into()));
        }
    }
    if let Some(amt) = amount_idr {
        if amt <= 0 {
            return Err(AppError::Validation(
                "amount_idr must be a positive integer".into(),
            ));
        }
    }
    Ok(())
}
