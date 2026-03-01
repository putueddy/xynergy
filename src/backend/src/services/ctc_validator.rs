//! CTC validation service
//!
//! Provides deterministic validation rules for CTC calculations,
//! BPJS compliance checks, and derived payroll values.

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Maximum allowed fixed allowances ratio against base salary.
pub const MAX_ALLOWANCE_RATIO: f64 = 2.0;
/// Integer IDR tolerance for rounded monetary comparison.
pub const ROUNDING_TOLERANCE_IDR: i64 = 1;

/// Validation issue severity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    Error,
    Warning,
}

impl FromStr for ValidationSeverity {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "error" => Ok(Self::Error),
            "warning" => Ok(Self::Warning),
            _ => Err(format!("invalid validation severity: '{}'", s)),
        }
    }
}

impl Display for ValidationSeverity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub field: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CtcValidationInput {
    pub base_salary: i64,
    pub hra_allowance: i64,
    pub medical_allowance: i64,
    pub transport_allowance: i64,
    pub meal_allowance: i64,
    pub bpjs_kesehatan_employer: i64,
    pub bpjs_ketenagakerjaan_employer: i64,
    pub thr_monthly_accrual: i64,
    pub total_monthly_ctc: i64,
    pub daily_rate: BigDecimal,
    pub working_days_per_month: i32,
    pub risk_tier: i32,
    pub thr_eligible: bool,
}

fn parse_decimal(value: &str) -> Option<BigDecimal> {
    value.parse::<BigDecimal>().ok()
}

fn parse_ratio_decimal() -> Option<BigDecimal> {
    MAX_ALLOWANCE_RATIO.to_string().parse::<BigDecimal>().ok()
}

fn tolerance_decimal() -> BigDecimal {
    BigDecimal::from(ROUNDING_TOLERANCE_IDR)
}

fn jkk_rate_for_tier(tier: i32) -> Option<BigDecimal> {
    match tier {
        1 => parse_decimal("0.0024"),
        2 => parse_decimal("0.0054"),
        3 => parse_decimal("0.0089"),
        4 => parse_decimal("0.0174"),
        _ => None,
    }
}

fn push_issue(
    issues: &mut Vec<ValidationIssue>,
    severity: ValidationSeverity,
    field: &str,
    expected: Option<String>,
    actual: Option<String>,
    message: impl Into<String>,
) {
    issues.push(ValidationIssue {
        severity,
        field: field.to_string(),
        expected,
        actual,
        message: message.into(),
    });
}

const MONETARY_FIELDS: &[&str] = &[
    "base_salary",
    "hra_allowance",
    "medical_allowance",
    "transport_allowance",
    "meal_allowance",
];

pub fn validate_monetary_whole_numbers(body: &Value) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for field in MONETARY_FIELDS {
        if let Some(value) = body.get(*field) {
            if let Some(f) = value.as_f64() {
                if f.fract() != 0.0 {
                    push_issue(
                        &mut issues,
                        ValidationSeverity::Error,
                        field,
                        Some("whole number (IDR)".to_string()),
                        Some(value.to_string()),
                        format!("{} must be a whole number (no decimal portions)", field),
                    );
                }
            }
            if let Some(s) = value.as_str() {
                if s.contains('.') {
                    push_issue(
                        &mut issues,
                        ValidationSeverity::Error,
                        field,
                        Some("whole number (IDR)".to_string()),
                        Some(s.to_string()),
                        format!("{} must be a whole number (no decimal portions)", field),
                    );
                }
            }
        }
    }
    issues
}

/// Validate calculated CTC components and derived values.
pub fn validate_ctc(input: &CtcValidationInput) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if input.base_salary <= 0 {
        push_issue(
            &mut issues,
            ValidationSeverity::Error,
            "base_salary",
            Some("greater than 0".to_string()),
            Some(input.base_salary.to_string()),
            "Base salary must be greater than zero",
        );
    }

    let allowances = [
        ("hra_allowance", input.hra_allowance),
        ("medical_allowance", input.medical_allowance),
        ("transport_allowance", input.transport_allowance),
        ("meal_allowance", input.meal_allowance),
    ];

    for (field, value) in allowances {
        if value < 0 {
            push_issue(
                &mut issues,
                ValidationSeverity::Error,
                field,
                Some("greater than or equal to 0".to_string()),
                Some(value.to_string()),
                format!("{} must be non-negative", field),
            );
        }
    }

    let total_allowances = input.hra_allowance
        + input.medical_allowance
        + input.transport_allowance
        + input.meal_allowance;
    if let Some(ratio) = parse_ratio_decimal() {
        let allowance_limit = BigDecimal::from(input.base_salary) * ratio;
        let total_allowances_bd = BigDecimal::from(total_allowances);
        if total_allowances_bd > allowance_limit {
            push_issue(
                &mut issues,
                ValidationSeverity::Error,
                "allowances_total",
                Some(allowance_limit.to_string()),
                Some(total_allowances.to_string()),
                "Total allowances exceed allowed ratio of base salary",
            );
        }
    }

    let bpjs_basis = BigDecimal::from(input.base_salary + total_allowances);

    let kesehatan_cap = BigDecimal::from(12_000_000i64);
    if let Some(kesehatan_rate) = parse_decimal("0.04") {
        let expected_kes = bpjs_basis.clone().min(kesehatan_cap) * kesehatan_rate;
        let expected_kes_i64 = expected_kes
            .to_string()
            .split('.')
            .next()
            .unwrap_or("0")
            .parse::<i64>()
            .unwrap_or(0);
        let diff = (input.bpjs_kesehatan_employer - expected_kes_i64).abs();
        if diff > ROUNDING_TOLERANCE_IDR {
            push_issue(
                &mut issues,
                ValidationSeverity::Warning,
                "bpjs_kesehatan_employer",
                Some(expected_kes_i64.to_string()),
                Some(input.bpjs_kesehatan_employer.to_string()),
                "BPJS Kesehatan employer contribution differs from expected formula",
            );
        }
    }

    let jp_cap = BigDecimal::from(10_547_400i64);
    let jkk_rate = jkk_rate_for_tier(input.risk_tier);
    let jkm_rate = parse_decimal("0.003");
    let jht_employer_rate = parse_decimal("0.037");
    let jp_employer_rate = parse_decimal("0.02");

    if let (Some(jkk_rate), Some(jkm_rate), Some(jht_employer_rate), Some(jp_employer_rate)) =
        (jkk_rate, jkm_rate, jht_employer_rate, jp_employer_rate)
    {
        let jkk = bpjs_basis.clone() * jkk_rate;
        let jkm = bpjs_basis.clone() * jkm_rate;
        let jht_employer = bpjs_basis.clone() * jht_employer_rate;
        let jp_basis = bpjs_basis.clone().min(jp_cap);
        let jp_employer = jp_basis * jp_employer_rate;
        let expected_kt = jkk + jkm + jht_employer + jp_employer;
        let expected_kt_i64 = expected_kt
            .to_string()
            .split('.')
            .next()
            .unwrap_or("0")
            .parse::<i64>()
            .unwrap_or(0);
        let diff = (input.bpjs_ketenagakerjaan_employer - expected_kt_i64).abs();
        if diff > ROUNDING_TOLERANCE_IDR {
            push_issue(
                &mut issues,
                ValidationSeverity::Warning,
                "bpjs_ketenagakerjaan_employer",
                Some(expected_kt_i64.to_string()),
                Some(input.bpjs_ketenagakerjaan_employer.to_string()),
                "BPJS Ketenagakerjaan employer contribution differs from expected formula",
            );
        }
    } else {
        push_issue(
            &mut issues,
            ValidationSeverity::Warning,
            "risk_tier",
            Some("1-4".to_string()),
            Some(input.risk_tier.to_string()),
            "Risk tier is invalid for JKK calculation",
        );
    }

    if input.thr_eligible {
        let expected_thr = BigDecimal::from(input.base_salary) / BigDecimal::from(12i64);
        let expected_thr_i64 = expected_thr
            .to_string()
            .split('.')
            .next()
            .unwrap_or("0")
            .parse::<i64>()
            .unwrap_or(0);
        let diff = (input.thr_monthly_accrual - expected_thr_i64).abs();
        if diff > ROUNDING_TOLERANCE_IDR {
            push_issue(
                &mut issues,
                ValidationSeverity::Warning,
                "thr_monthly_accrual",
                Some(expected_thr_i64.to_string()),
                Some(input.thr_monthly_accrual.to_string()),
                "THR monthly accrual differs from expected formula",
            );
        }
    }

    let expected_total = input.base_salary
        + total_allowances
        + input.bpjs_kesehatan_employer
        + input.bpjs_ketenagakerjaan_employer
        + input.thr_monthly_accrual;
    if input.total_monthly_ctc != expected_total {
        push_issue(
            &mut issues,
            ValidationSeverity::Error,
            "total_monthly_ctc",
            Some(expected_total.to_string()),
            Some(input.total_monthly_ctc.to_string()),
            "Total monthly CTC must equal sum of all monthly components",
        );
    }

    if input.working_days_per_month <= 0 {
        push_issue(
            &mut issues,
            ValidationSeverity::Error,
            "working_days_per_month",
            Some("greater than 0".to_string()),
            Some(input.working_days_per_month.to_string()),
            "Working days per month must be greater than zero",
        );
    } else {
        let expected_daily = BigDecimal::from(input.total_monthly_ctc)
            / BigDecimal::from(input.working_days_per_month);
        let diff = (&input.daily_rate - &expected_daily).abs();
        let tolerance = tolerance_decimal() / BigDecimal::from(input.working_days_per_month);

        if diff > tolerance {
            push_issue(
                &mut issues,
                ValidationSeverity::Error,
                "daily_rate",
                Some(expected_daily.to_string()),
                Some(input.daily_rate.to_string()),
                "Daily rate differs from total_monthly_ctc / working_days beyond tolerance",
            );
        }
    }

    issues
}

pub fn has_errors(issues: &[ValidationIssue]) -> bool {
    issues
        .iter()
        .any(|issue| issue.severity == ValidationSeverity::Error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_input() -> CtcValidationInput {
        let total_monthly_ctc = 14_554_281i64;
        CtcValidationInput {
            base_salary: 10_000_000,
            hra_allowance: 1_000_000,
            medical_allowance: 500_000,
            transport_allowance: 500_000,
            meal_allowance: 500_000,
            bpjs_kesehatan_employer: 480_000,
            bpjs_ketenagakerjaan_employer: 740_948,
            thr_monthly_accrual: 833_333,
            total_monthly_ctc,
            daily_rate: BigDecimal::from(total_monthly_ctc) / BigDecimal::from(22i64),
            working_days_per_month: 22,
            risk_tier: 1,
            thr_eligible: true,
        }
    }

    #[test]
    fn test_negative_base_salary_error() {
        let mut input = valid_input();
        input.base_salary = -1;

        let issues = validate_ctc(&input);
        assert!(issues
            .iter()
            .any(|i| i.field == "base_salary" && i.severity == ValidationSeverity::Error));
    }

    #[test]
    fn test_zero_base_salary_error() {
        let mut input = valid_input();
        input.base_salary = 0;

        let issues = validate_ctc(&input);
        assert!(issues
            .iter()
            .any(|i| i.field == "base_salary" && i.severity == ValidationSeverity::Error));
    }

    #[test]
    fn test_valid_base_salary_passes() {
        let input = valid_input();
        let issues = validate_ctc(&input);
        assert!(!issues
            .iter()
            .any(|i| i.field == "base_salary" && i.severity == ValidationSeverity::Error));
    }

    #[test]
    fn test_negative_allowance_error() {
        let mut input = valid_input();
        input.hra_allowance = -100;

        let issues = validate_ctc(&input);
        assert!(issues
            .iter()
            .any(|i| i.field == "hra_allowance" && i.severity == ValidationSeverity::Error));
    }

    #[test]
    fn test_allowance_exceeding_200pct_error() {
        let mut input = valid_input();
        input.hra_allowance = 21_000_000;
        input.medical_allowance = 0;
        input.transport_allowance = 0;
        input.meal_allowance = 0;

        let issues = validate_ctc(&input);
        assert!(issues
            .iter()
            .any(|i| i.field == "allowances_total" && i.severity == ValidationSeverity::Error));
    }

    #[test]
    fn test_allowance_within_limit_passes() {
        let input = valid_input();

        let issues = validate_ctc(&input);
        assert!(!issues
            .iter()
            .any(|i| i.field == "allowances_total" && i.severity == ValidationSeverity::Error));
    }

    #[test]
    fn test_bpjs_kesehatan_mismatch_warning() {
        let mut input = valid_input();
        input.bpjs_kesehatan_employer = 1;

        let issues = validate_ctc(&input);
        assert!(issues.iter().any(|i| {
            i.field == "bpjs_kesehatan_employer" && i.severity == ValidationSeverity::Warning
        }));
    }

    #[test]
    fn test_bpjs_ketenagakerjaan_mismatch_warning() {
        let mut input = valid_input();
        input.bpjs_ketenagakerjaan_employer = 1;

        let issues = validate_ctc(&input);
        assert!(issues.iter().any(|i| {
            i.field == "bpjs_ketenagakerjaan_employer" && i.severity == ValidationSeverity::Warning
        }));
    }

    #[test]
    fn test_thr_accrual_mismatch_warning() {
        let mut input = valid_input();
        input.thr_monthly_accrual = 1;

        let issues = validate_ctc(&input);
        assert!(
            issues
                .iter()
                .any(|i| i.field == "thr_monthly_accrual"
                    && i.severity == ValidationSeverity::Warning)
        );
    }

    #[test]
    fn test_total_monthly_ctc_mismatch_error() {
        let mut input = valid_input();
        input.total_monthly_ctc = 1;

        let issues = validate_ctc(&input);
        assert!(issues
            .iter()
            .any(|i| i.field == "total_monthly_ctc" && i.severity == ValidationSeverity::Error));
    }

    #[test]
    fn test_daily_rate_mismatch_error() {
        let mut input = valid_input();
        input.daily_rate = "1".parse::<BigDecimal>().unwrap();

        let issues = validate_ctc(&input);
        assert!(issues
            .iter()
            .any(|i| i.field == "daily_rate" && i.severity == ValidationSeverity::Error));
    }

    #[test]
    fn test_valid_ctc_no_issues() {
        let input = valid_input();

        let issues = validate_ctc(&input);
        assert!(issues.is_empty());
        assert!(!has_errors(&issues));
    }

    #[test]
    fn test_validation_severity_from_str() {
        assert_eq!(
            ValidationSeverity::from_str("error").unwrap(),
            ValidationSeverity::Error
        );
        assert_eq!(
            ValidationSeverity::from_str("warning").unwrap(),
            ValidationSeverity::Warning
        );
        assert!(ValidationSeverity::from_str("invalid").is_err());
    }

    #[test]
    fn test_decimal_in_monetary_field_error() {
        let body = serde_json::json!({
            "base_salary": 1000.5,
            "hra_allowance": 0,
            "medical_allowance": 0,
            "transport_allowance": 0,
            "meal_allowance": 0,
        });
        let issues = validate_monetary_whole_numbers(&body);
        assert!(
            !issues.is_empty(),
            "Decimal base_salary should produce an error"
        );
        assert!(issues
            .iter()
            .any(|i| i.field == "base_salary" && i.severity == ValidationSeverity::Error));
    }

    #[test]
    fn test_whole_number_monetary_fields_pass() {
        let body = serde_json::json!({
            "base_salary": 10000000,
            "hra_allowance": 1000000,
            "medical_allowance": 500000,
            "transport_allowance": 500000,
            "meal_allowance": 500000,
        });
        let issues = validate_monetary_whole_numbers(&body);
        assert!(
            issues.is_empty(),
            "Whole number fields should pass validation"
        );
    }
}
