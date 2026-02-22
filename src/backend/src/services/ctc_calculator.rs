//! CTC (Cost to Company) calculation service
//!
//! Provides deterministic BPJS calculations and daily rate computation
//! for Indonesian payroll compliance.

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

/// BPJS configuration constants (configurable)
pub struct BpjsConfig {
    /// BPJS Kesehatan employer rate (default: 4%)
    pub kesehatan_employer_rate: BigDecimal,
    /// BPJS Kesehatan employee rate (default: 1%)
    pub kesehatan_employee_rate: BigDecimal,
    /// BPJS Kesehatan wage cap in IDR (default: 12,000,000)
    pub kesehatan_wage_cap: BigDecimal,
    /// BPJS Ketenagakerjaan JKK rate range (default: 0.24% - 1.74% based on risk tier)
    pub ketenagakerjaan_jkk_rate: BigDecimal,
    /// BPJS Ketenagakerjaan JKM rate (default: 0.30%)
    pub ketenagakerjaan_jkm_rate: BigDecimal,
    /// BPJS Ketenagakerjaan JHT employer rate (default: 3.7%)
    pub ketenagakerjaan_jht_employer_rate: BigDecimal,
    /// BPJS Ketenagakerjaan JHT employee rate (default: 2%)
    pub ketenagakerjaan_jht_employee_rate: BigDecimal,
    /// BPJS Ketenagakerjaan JP employer rate (default: 2%)
    pub ketenagakerjaan_jp_employer_rate: BigDecimal,
    /// BPJS Ketenagakerjaan JP employee rate (default: 1%)
    pub ketenagakerjaan_jp_employee_rate: BigDecimal,
    /// BPJS Ketenagakerjaan JP wage cap in IDR (default: 10,547,400)
    pub ketenagakerjaan_jp_cap: BigDecimal,
}

impl Default for BpjsConfig {
    fn default() -> Self {
        Self {
            kesehatan_employer_rate: "0.04".parse().unwrap(),
            kesehatan_employee_rate: "0.01".parse().unwrap(),
            kesehatan_wage_cap: BigDecimal::from(12_000_000i64),
            ketenagakerjaan_jkk_rate: "0.0024".parse().unwrap(), // Low risk tier
            ketenagakerjaan_jkm_rate: "0.003".parse().unwrap(),
            ketenagakerjaan_jht_employer_rate: "0.037".parse().unwrap(),
            ketenagakerjaan_jht_employee_rate: "0.02".parse().unwrap(),
            ketenagakerjaan_jp_employer_rate: "0.02".parse().unwrap(),
            ketenagakerjaan_jp_employee_rate: "0.01".parse().unwrap(),
            ketenagakerjaan_jp_cap: BigDecimal::from(10_547_400i64),
        }
    }
}

/// CTC component breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtcComponents {
    pub base_salary: BigDecimal,
    pub hra_allowance: BigDecimal,
    pub medical_allowance: BigDecimal,
    pub transport_allowance: BigDecimal,
    pub meal_allowance: BigDecimal,
}

impl CtcComponents {
    /// Calculate total fixed allowances
    pub fn total_allowances(&self) -> BigDecimal {
        &self.hra_allowance
            + &self.medical_allowance
            + &self.transport_allowance
            + &self.meal_allowance
    }

    /// Calculate base salary + allowances (for BPJS calculation basis)
    pub fn bpjs_calculation_basis(&self) -> BigDecimal {
        &self.base_salary + &self.total_allowances()
    }
}

/// BPJS calculation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BpjsCalculation {
    /// BPJS Kesehatan employer contribution
    pub kesehatan_employer: BigDecimal,
    /// BPJS Kesehatan employee contribution
    pub kesehatan_employee: BigDecimal,
    /// BPJS Ketenagakerjaan total employer contribution (JKK + JKM + JHT employer + JP employer)
    pub ketenagakerjaan_employer: BigDecimal,
    /// BPJS Ketenagakerjaan total employee contribution (JHT employee + JP employee)
    pub ketenagakerjaan_employee: BigDecimal,
}

/// Complete CTC calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtcCalculation {
    pub components: CtcComponents,
    pub bpjs: BpjsCalculation,
    pub thr_monthly_accrual: BigDecimal,
    pub total_monthly_ctc: BigDecimal,
    pub daily_rate: BigDecimal,
    pub working_days_per_month: i32,
}

/// Calculate BPJS contributions based on salary components
pub fn calculate_bpjs(components: &CtcComponents, config: &BpjsConfig) -> BpjsCalculation {
    let basis = components.bpjs_calculation_basis();

    // BPJS Kesehatan (capped at wage cap)
    let kesehatan_basis = basis.clone().min(config.kesehatan_wage_cap.clone());
    let kesehatan_employer = &kesehatan_basis * &config.kesehatan_employer_rate;
    let kesehatan_employee = &kesehatan_basis * &config.kesehatan_employee_rate;

    // BPJS Ketenagakerjaan
    // JKK (Work Accident) - based on risk tier, no cap
    let jkk = &basis * &config.ketenagakerjaan_jkk_rate;

    // JKM (Death) - no cap
    let jkm = &basis * &config.ketenagakerjaan_jkm_rate;

    // JHT (Old Age) - no cap
    let jht_employer = &basis * &config.ketenagakerjaan_jht_employer_rate;
    let jht_employee = &basis * &config.ketenagakerjaan_jht_employee_rate;

    // JP (Pension) - capped
    let jp_basis = basis.clone().min(config.ketenagakerjaan_jp_cap.clone());
    let jp_employer = &jp_basis * &config.ketenagakerjaan_jp_employer_rate;
    let jp_employee = &jp_basis * &config.ketenagakerjaan_jp_employee_rate;

    BpjsCalculation {
        kesehatan_employer,
        kesehatan_employee,
        ketenagakerjaan_employer: &jkk + &jkm + &jht_employer + &jp_employer,
        ketenagakerjaan_employee: &jht_employee + &jp_employee,
    }
}

/// Calculate THR monthly accrual (1 month's salary / 12)
pub fn calculate_thr_monthly(base_salary: &BigDecimal) -> BigDecimal {
    base_salary / BigDecimal::from(12i64)
}

/// Calculate total monthly CTC
pub fn calculate_total_monthly_ctc(
    components: &CtcComponents,
    bpjs: &BpjsCalculation,
    thr_monthly: &BigDecimal,
) -> BigDecimal {
    &components.base_salary
        + &components.total_allowances()
        + &bpjs.kesehatan_employer
        + &bpjs.ketenagakerjaan_employer
        + thr_monthly
}

/// Calculate daily rate
pub fn calculate_daily_rate(monthly_ctc: &BigDecimal, working_days: i32) -> BigDecimal {
    monthly_ctc / BigDecimal::from(working_days)
}

/// Perform complete CTC calculation
pub fn calculate_ctc(
    components: CtcComponents,
    working_days_per_month: i32,
    config: &BpjsConfig,
) -> CtcCalculation {
    let bpjs = calculate_bpjs(&components, config);
    let thr_monthly_accrual = calculate_thr_monthly(&components.base_salary);
    let total_monthly_ctc = calculate_total_monthly_ctc(&components, &bpjs, &thr_monthly_accrual);
    let daily_rate = calculate_daily_rate(&total_monthly_ctc, working_days_per_month);

    CtcCalculation {
        components,
        bpjs,
        thr_monthly_accrual,
        total_monthly_ctc,
        daily_rate,
        working_days_per_month,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpjs_kesehatan_calculation() {
        let config = BpjsConfig::default();
        let components = CtcComponents {
            base_salary: BigDecimal::from(10_000_000i64),
            hra_allowance: BigDecimal::from(2_000_000i64),
            medical_allowance: BigDecimal::from(1_000_000i64),
            transport_allowance: BigDecimal::from(500_000i64),
            meal_allowance: BigDecimal::from(500_000i64),
        };

        let bpjs = calculate_bpjs(&components, &config);

        // Basis = 10M + 2M + 1M + 500K + 500K = 14M
        // Capped at 12M for Kesehatan
        // Employer: 12M * 4% = 480,000
        // Employee: 12M * 1% = 120,000
        assert_eq!(bpjs.kesehatan_employer, BigDecimal::from(480_000i64));
        assert_eq!(bpjs.kesehatan_employee, BigDecimal::from(120_000i64));
    }

    #[test]
    fn test_daily_rate_calculation() {
        let monthly = BigDecimal::from(22_000_000i64);
        let daily = calculate_daily_rate(&monthly, 22);
        // 22M / 22 = 1M
        assert_eq!(daily, BigDecimal::from(1_000_000i64));
    }

    #[test]
    fn test_thr_accrual() {
        let base = BigDecimal::from(12_000_000i64);
        let thr = calculate_thr_monthly(&base);
        // 12M / 12 = 1M
        assert_eq!(thr, BigDecimal::from(1_000_000i64));
    }
}
