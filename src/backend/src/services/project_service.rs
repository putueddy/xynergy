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
