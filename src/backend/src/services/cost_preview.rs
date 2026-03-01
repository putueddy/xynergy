use bigdecimal::BigDecimal;
use chrono::{Datelike, NaiveDate, Weekday};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MonthlyBucket {
    pub month: String,
    pub working_days: i32,
    pub cost_idr: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostPreviewResult {
    pub total_cost_idr: i64,
    pub working_days: i32,
    pub monthly_breakdown: Vec<MonthlyBucket>,
}

pub fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

fn is_working_day(
    date: NaiveDate,
    include_weekend: bool,
    holiday_set: &HashSet<NaiveDate>,
) -> bool {
    (include_weekend || !is_weekend(date)) && !holiday_set.contains(&date)
}

fn month_key(date: NaiveDate) -> String {
    format!("{:04}-{:02}", date.year(), date.month())
}

fn cost_for_days(daily_rate_idr: i64, days: i32, allocation_percentage: f64) -> i64 {
    let daily_rate = BigDecimal::from(daily_rate_idr);
    let day_count = BigDecimal::from(days);
    let percentage = allocation_percentage
        .to_string()
        .parse::<BigDecimal>()
        .unwrap_or_else(|_| BigDecimal::from(0));
    let hundred = "100"
        .parse::<BigDecimal>()
        .unwrap_or_else(|_| BigDecimal::from(100));

    let raw_cost = daily_rate * day_count * (percentage / hundred);
    raw_cost
        .to_string()
        .split('.')
        .next()
        .unwrap_or("0")
        .parse::<i64>()
        .unwrap_or(0)
}

pub fn count_working_days(
    start_date: NaiveDate,
    end_date: NaiveDate,
    include_weekend: bool,
    holidays: &[NaiveDate],
) -> i32 {
    if start_date > end_date {
        return 0;
    }

    let holiday_set: HashSet<NaiveDate> = holidays.iter().copied().collect();
    let mut current = start_date;
    let mut count = 0;

    while current <= end_date {
        if is_working_day(current, include_weekend, &holiday_set) {
            count += 1;
        }
        current = current.succ_opt().unwrap_or(current);
        if current == end_date.succ_opt().unwrap_or(end_date) {
            break;
        }
    }

    count
}

pub fn calculate_cost_preview(
    daily_rate_idr: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
    allocation_percentage: f64,
    include_weekend: bool,
    holidays: &[NaiveDate],
) -> CostPreviewResult {
    if start_date > end_date {
        return CostPreviewResult {
            total_cost_idr: 0,
            working_days: 0,
            monthly_breakdown: Vec::new(),
        };
    }

    let holiday_set: HashSet<NaiveDate> = holidays.iter().copied().collect();
    let mut current = start_date;
    let mut monthly_days: BTreeMap<String, i32> = BTreeMap::new();
    let mut working_days = 0;

    while current <= end_date {
        if is_working_day(current, include_weekend, &holiday_set) {
            working_days += 1;
            let key = month_key(current);
            *monthly_days.entry(key).or_insert(0) += 1;
        }
        current = current.succ_opt().unwrap_or(current);
        if current == end_date.succ_opt().unwrap_or(end_date) {
            break;
        }
    }

    let monthly_breakdown = monthly_days
        .into_iter()
        .map(|(month, days)| MonthlyBucket {
            cost_idr: cost_for_days(daily_rate_idr, days, allocation_percentage),
            month,
            working_days: days,
        })
        .collect::<Vec<_>>();

    CostPreviewResult {
        total_cost_idr: cost_for_days(daily_rate_idr, working_days, allocation_percentage),
        working_days,
        monthly_breakdown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
    }

    #[test]
    fn count_working_days_weekdays_only() {
        let count = count_working_days(d(2026, 2, 2), d(2026, 2, 8), false, &[]);
        assert_eq!(count, 5);
    }

    #[test]
    fn count_working_days_including_weekends() {
        let count = count_working_days(d(2026, 2, 2), d(2026, 2, 8), true, &[]);
        assert_eq!(count, 7);
    }

    #[test]
    fn count_working_days_excludes_holidays() {
        let holidays = vec![d(2026, 2, 3), d(2026, 2, 5)];
        let count = count_working_days(d(2026, 2, 2), d(2026, 2, 6), false, &holidays);
        assert_eq!(count, 3);
    }

    #[test]
    fn monthly_bucketing_across_month_boundary() {
        let result =
            calculate_cost_preview(1_000_000, d(2026, 2, 27), d(2026, 3, 3), 50.0, false, &[]);

        assert_eq!(result.working_days, 3);
        assert_eq!(result.total_cost_idr, 1_500_000);
        assert_eq!(result.monthly_breakdown.len(), 2);
        assert_eq!(result.monthly_breakdown[0].month, "2026-02");
        assert_eq!(result.monthly_breakdown[0].working_days, 1);
        assert_eq!(result.monthly_breakdown[0].cost_idr, 500_000);
        assert_eq!(result.monthly_breakdown[1].month, "2026-03");
        assert_eq!(result.monthly_breakdown[1].working_days, 2);
        assert_eq!(result.monthly_breakdown[1].cost_idr, 1_000_000);
    }

    #[test]
    fn cost_calculation_with_various_allocation_percentages() {
        let full =
            calculate_cost_preview(1_200_000, d(2026, 2, 2), d(2026, 2, 6), 100.0, false, &[]);
        assert_eq!(full.total_cost_idr, 6_000_000);

        let half =
            calculate_cost_preview(1_200_000, d(2026, 2, 2), d(2026, 2, 6), 50.0, false, &[]);
        assert_eq!(half.total_cost_idr, 3_000_000);

        let fractional =
            calculate_cost_preview(1_200_000, d(2026, 2, 2), d(2026, 2, 6), 33.33, false, &[]);
        assert_eq!(fractional.total_cost_idr, 1_999_800);
    }
}
