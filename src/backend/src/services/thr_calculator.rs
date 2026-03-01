//! THR (Tunjangan Hari Raya) calculation service
//!
//! Provides deterministic THR entitlement, accrual, and payout calculations
//! for Indonesian religious holiday allowance compliance per Manpower Regulation No. 6/2016.
//!
//! Key rules:
//! - Full entitlement: 1 month salary (base + fixed allowances) for >= 12 months service
//! - Prorated: (service_months / 12) * basis_amount for < 12 months service
//! - Monthly accrual: annual_entitlement / 12
//! - Payout deadline: no later than H-7 before religious holiday

use bigdecimal::BigDecimal;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

/// THR calculation basis options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThrCalculationBasis {
    /// Full entitlement: 1 month salary for >= 12 months service
    Full,
    /// Prorated: (service_months / 12) * basis for < 12 months service
    Prorated,
}

impl std::str::FromStr for ThrCalculationBasis {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "full" => Ok(Self::Full),
            "prorated" => Ok(Self::Prorated),
            _ => Err(format!("invalid THR calculation basis: '{}'", s)),
        }
    }
}

impl ThrCalculationBasis {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Prorated => "prorated",
        }
    }
}

/// THR configuration for an employee CTC record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrConfig {
    pub eligible: bool,
    pub calculation_basis: ThrCalculationBasis,
    /// Employment start date for service-month computation
    pub employment_start_date: Option<NaiveDate>,
}

impl Default for ThrConfig {
    fn default() -> Self {
        Self {
            eligible: true,
            calculation_basis: ThrCalculationBasis::Full,
            employment_start_date: None,
        }
    }
}

/// Complete THR calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrCalculation {
    pub eligible: bool,
    pub calculation_basis: ThrCalculationBasis,
    pub service_months: i32,
    /// THR basis amount: base_salary + fixed allowances
    pub basis_amount: BigDecimal,
    /// Annual entitlement (full or prorated)
    pub annual_entitlement: BigDecimal,
    /// Monthly accrual: annual_entitlement / 12
    pub monthly_accrual: BigDecimal,
}

/// THR accrual entry for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrAccrualEntry {
    pub resource_id: uuid::Uuid,
    pub accrual_period: String,
    pub service_months: i32,
    pub calculation_basis: String,
    pub accrual_amount: BigDecimal,
    pub annual_entitlement: BigDecimal,
}

/// Compute service months between employment start date and a reference date.
///
/// Returns 0 if employment_start_date is None or in the future.
/// Uses whole-month counting: counts completed calendar months.
pub fn compute_service_months(
    employment_start_date: Option<NaiveDate>,
    reference_date: NaiveDate,
) -> i32 {
    let start = match employment_start_date {
        Some(d) => d,
        None => return 0,
    };

    if start > reference_date {
        return 0;
    }

    let years = reference_date.year() - start.year();
    let months = reference_date.month() as i32 - start.month() as i32;
    let day_adjustment = if reference_date.day() < start.day() {
        1
    } else {
        0
    };

    let total_months = years * 12 + months - day_adjustment;
    total_months.max(0)
}

/// Calculate THR basis amount (base salary + fixed allowances).
///
/// Per Indonesian regulation, THR basis includes base salary and all fixed allowances.
pub fn calculate_thr_basis(
    base_salary: &BigDecimal,
    hra_allowance: &BigDecimal,
    medical_allowance: &BigDecimal,
    transport_allowance: &BigDecimal,
    meal_allowance: &BigDecimal,
) -> BigDecimal {
    base_salary + hra_allowance + medical_allowance + transport_allowance + meal_allowance
}

/// Calculate annual THR entitlement based on service months and calculation basis.
///
/// - Full basis (>= 12 months service): 1x basis_amount
/// - Prorated (< 12 months service): (service_months / 12) * basis_amount
/// - Not eligible: 0
pub fn calculate_annual_entitlement(
    basis_amount: &BigDecimal,
    service_months: i32,
    calculation_basis: &ThrCalculationBasis,
    eligible: bool,
) -> BigDecimal {
    if !eligible || service_months <= 0 {
        return BigDecimal::from(0i64);
    }

    match calculation_basis {
        ThrCalculationBasis::Full => {
            if service_months >= 12 {
                basis_amount.clone()
            } else {
                // Even if set to "full", employees with < 12 months get prorated
                let months_bd = BigDecimal::from(service_months);
                let twelve = BigDecimal::from(12i64);
                (basis_amount * &months_bd) / &twelve
            }
        }
        ThrCalculationBasis::Prorated => {
            let months_bd = BigDecimal::from(service_months.min(12));
            let twelve = BigDecimal::from(12i64);
            (basis_amount * &months_bd) / &twelve
        }
    }
}

/// Calculate monthly THR accrual: annual_entitlement / 12.
pub fn calculate_monthly_accrual(annual_entitlement: &BigDecimal) -> BigDecimal {
    annual_entitlement / BigDecimal::from(12i64)
}

/// Perform complete THR calculation for an employee.
pub fn calculate_thr(
    config: &ThrConfig,
    base_salary: &BigDecimal,
    hra_allowance: &BigDecimal,
    medical_allowance: &BigDecimal,
    transport_allowance: &BigDecimal,
    meal_allowance: &BigDecimal,
    reference_date: NaiveDate,
) -> ThrCalculation {
    let service_months = compute_service_months(config.employment_start_date, reference_date);
    let basis_amount = calculate_thr_basis(
        base_salary,
        hra_allowance,
        medical_allowance,
        transport_allowance,
        meal_allowance,
    );
    let annual_entitlement = calculate_annual_entitlement(
        &basis_amount,
        service_months,
        &config.calculation_basis,
        config.eligible,
    );
    let monthly_accrual = calculate_monthly_accrual(&annual_entitlement);

    ThrCalculation {
        eligible: config.eligible,
        calculation_basis: config.calculation_basis.clone(),
        service_months,
        basis_amount,
        annual_entitlement,
        monthly_accrual,
    }
}

/// Generate accrual period string in YYYY-MM format.
pub fn format_accrual_period(year: i32, month: u32) -> String {
    format!("{:04}-{:02}", year, month)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_months_full_year() {
        let start = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
        let ref_date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        assert_eq!(compute_service_months(Some(start), ref_date), 12);
    }

    #[test]
    fn test_service_months_partial_year() {
        let start = NaiveDate::from_ymd_opt(2025, 8, 15).unwrap();
        let ref_date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        // 2025-08-15 to 2026-03-01: 6 completed months (Aug15→Sep15, Sep15→Oct15, ..., Jan15→Feb15)
        assert_eq!(compute_service_months(Some(start), ref_date), 6);
    }

    #[test]
    fn test_service_months_same_month() {
        let start = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let ref_date = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        assert_eq!(compute_service_months(Some(start), ref_date), 0);
    }

    #[test]
    fn test_service_months_future_start() {
        let start = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        let ref_date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        assert_eq!(compute_service_months(Some(start), ref_date), 0);
    }

    #[test]
    fn test_service_months_none_start() {
        assert_eq!(
            compute_service_months(None, NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
            0
        );
    }

    #[test]
    fn test_thr_basis_calculation() {
        let basis = calculate_thr_basis(
            &BigDecimal::from(10_000_000i64),
            &BigDecimal::from(2_000_000i64),
            &BigDecimal::from(1_000_000i64),
            &BigDecimal::from(500_000i64),
            &BigDecimal::from(500_000i64),
        );
        assert_eq!(basis, BigDecimal::from(14_000_000i64));
    }

    #[test]
    fn test_full_entitlement_12_plus_months() {
        let basis = BigDecimal::from(14_000_000i64);
        let entitlement =
            calculate_annual_entitlement(&basis, 14, &ThrCalculationBasis::Full, true);
        // Full entitlement = 1x basis
        assert_eq!(entitlement, BigDecimal::from(14_000_000i64));
    }

    #[test]
    fn test_full_basis_under_12_months_prorates() {
        let basis = BigDecimal::from(12_000_000i64);
        let entitlement = calculate_annual_entitlement(&basis, 6, &ThrCalculationBasis::Full, true);
        // Even "full" basis prorates under 12 months: 6/12 * 12M = 6M
        assert_eq!(entitlement, BigDecimal::from(6_000_000i64));
    }

    #[test]
    fn test_prorated_entitlement() {
        let basis = BigDecimal::from(12_000_000i64);
        let entitlement =
            calculate_annual_entitlement(&basis, 8, &ThrCalculationBasis::Prorated, true);
        // Prorated: 8/12 * 12M = 8M
        assert_eq!(entitlement, BigDecimal::from(8_000_000i64));
    }

    #[test]
    fn test_prorated_caps_at_12_months() {
        let basis = BigDecimal::from(12_000_000i64);
        let entitlement =
            calculate_annual_entitlement(&basis, 18, &ThrCalculationBasis::Prorated, true);
        // Prorated caps at 12 months: 12/12 * 12M = 12M
        assert_eq!(entitlement, BigDecimal::from(12_000_000i64));
    }

    #[test]
    fn test_not_eligible_returns_zero() {
        let basis = BigDecimal::from(14_000_000i64);
        let entitlement =
            calculate_annual_entitlement(&basis, 12, &ThrCalculationBasis::Full, false);
        assert_eq!(entitlement, BigDecimal::from(0i64));
    }

    #[test]
    fn test_zero_service_months_returns_zero() {
        let basis = BigDecimal::from(14_000_000i64);
        let entitlement = calculate_annual_entitlement(&basis, 0, &ThrCalculationBasis::Full, true);
        assert_eq!(entitlement, BigDecimal::from(0i64));
    }

    #[test]
    fn test_monthly_accrual() {
        let annual = BigDecimal::from(12_000_000i64);
        let monthly = calculate_monthly_accrual(&annual);
        assert_eq!(monthly, BigDecimal::from(1_000_000i64));
    }

    #[test]
    fn test_complete_thr_calculation() {
        let config = ThrConfig {
            eligible: true,
            calculation_basis: ThrCalculationBasis::Full,
            employment_start_date: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        };
        let ref_date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();

        let result = calculate_thr(
            &config,
            &BigDecimal::from(10_000_000i64),
            &BigDecimal::from(2_000_000i64),
            &BigDecimal::from(1_000_000i64),
            &BigDecimal::from(500_000i64),
            &BigDecimal::from(500_000i64),
            ref_date,
        );

        assert!(result.eligible);
        assert_eq!(result.service_months, 14);
        assert_eq!(result.basis_amount, BigDecimal::from(14_000_000i64));
        // Full entitlement for >= 12 months
        assert_eq!(result.annual_entitlement, BigDecimal::from(14_000_000i64));
    }

    #[test]
    fn test_format_accrual_period() {
        assert_eq!(format_accrual_period(2026, 1), "2026-01");
        assert_eq!(format_accrual_period(2026, 12), "2026-12");
    }
}
