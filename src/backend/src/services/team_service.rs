use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::Serialize;
use sqlx::types::BigDecimal;
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService, EncryptedPayload};
use crate::services::key_provider::EnvKeyProvider;

#[derive(Debug, Serialize)]
pub struct AssignmentSummary {
    pub project_name: String,
    pub allocation_pct: f64,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize)]
pub struct TeamMemberResponse {
    pub resource_id: Uuid,
    pub name: String,
    pub role: String,
    pub department_name: String,
    pub daily_rate: Option<i64>,
    pub ctc_status: String,
    pub total_allocation_pct: f64,
    pub current_allocation_percentage: f64,
    pub is_overallocated: bool,
    pub active_assignments: Vec<AssignmentSummary>,
}

#[derive(Debug, Serialize)]
pub struct CapacityPeriod {
    pub period: String,
    pub total_allocation_percentage: f64,
    pub is_overallocated: bool,
    pub allocation_count: i32,
}

#[derive(Debug, Serialize)]
pub struct EmployeeCapacity {
    pub resource_id: Uuid,
    pub resource_name: String,
    pub periods: Vec<CapacityPeriod>,
}

#[derive(Debug, Serialize)]
pub struct CapacityReportResponse {
    pub start_date: String,
    pub end_date: String,
    pub employees: Vec<EmployeeCapacity>,
}

fn bd_to_i64_safe(bd: &BigDecimal) -> Result<i64> {
    let s = bd.to_string();
    let int_part = s.split('.').next().unwrap_or("0");
    int_part
        .parse::<i64>()
        .map_err(|_| AppError::Internal("Failed to convert daily_rate to i64".to_string()))
}

fn parse_json_decimal(value: &serde_json::Value, field: &str) -> Result<BigDecimal> {
    match value {
        serde_json::Value::String(s) => s.parse::<BigDecimal>().map_err(|_| {
            AppError::Internal(format!(
                "Failed to parse '{}' from decrypted payload",
                field
            ))
        }),
        serde_json::Value::Number(n) => n.to_string().parse::<BigDecimal>().map_err(|_| {
            AppError::Internal(format!(
                "Failed to parse '{}' from decrypted payload",
                field
            ))
        }),
        _ => Err(AppError::Internal(format!(
            "Invalid '{}' value in decrypted payload",
            field
        ))),
    }
}

async fn query_team_members<'e, E>(
    executor: E,
    department_id: Option<Uuid>,
    _user_role: &str,
) -> Result<Vec<TeamMemberResponse>>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let rows = sqlx::query(
        r#"
        SELECT
            r.id AS resource_id,
            r.name AS resource_name,
            r.resource_type AS resource_role,
            COALESCE(d.name, 'Unassigned') AS department_name,
            c.encrypted_daily_rate,
            c.daily_rate,
            c.key_version,
            c.encryption_version,
            c.encryption_algorithm,
            c.encrypted_at,
            p.name AS project_name,
            a.allocation_percentage,
            a.start_date::text AS start_date,
            a.end_date::text AS end_date
        FROM resources r
        LEFT JOIN departments d ON d.id = r.department_id
        LEFT JOIN ctc_records c ON c.resource_id = r.id AND c.status = 'Active'
        LEFT JOIN allocations a ON a.resource_id = r.id
            AND a.start_date <= CURRENT_DATE
            AND (a.end_date >= CURRENT_DATE OR a.end_date IS NULL)
        LEFT JOIN projects p ON p.id = a.project_id
        WHERE r.resource_type = 'employee'
          AND ($1::uuid IS NULL OR r.department_id = $1)
        ORDER BY r.name ASC
        "#,
    )
    .bind(department_id)
    .fetch_all(executor)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let mut grouped: HashMap<Uuid, TeamMemberResponse> = HashMap::new();

    for row in rows {
        let resource_id: Uuid = row
            .try_get("resource_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let name: String = row
            .try_get("resource_name")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let role: String = row
            .try_get("resource_role")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let department_name: String = row
            .try_get("department_name")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let encrypted_daily_rate: Option<String> = row
            .try_get("encrypted_daily_rate")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let plaintext_daily_rate: Option<BigDecimal> = row
            .try_get("daily_rate")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let member = if let Some(existing) = grouped.get_mut(&resource_id) {
            existing
        } else {
            let (daily_rate, ctc_status) = if let Some(ciphertext) = encrypted_daily_rate {
                let key_version: String = row
                    .try_get("key_version")
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let encryption_version: String = row
                    .try_get("encryption_version")
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let algorithm: String = row
                    .try_get("encryption_algorithm")
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let encrypted_at: DateTime<Utc> = row
                    .try_get("encrypted_at")
                    .map_err(|e| AppError::Database(e.to_string()))?;

                let payload = EncryptedPayload {
                    ciphertext,
                    key_version,
                    encryption_version,
                    algorithm,
                    encrypted_at,
                };

                let decrypted = crypto_svc.decrypt_components(&payload).await?;
                let daily_rate_value = decrypted.get("daily_rate").ok_or_else(|| {
                    AppError::Internal(
                        "Missing daily_rate in decrypted CTC payload for team view".to_string(),
                    )
                })?;
                let daily_rate_bd = parse_json_decimal(daily_rate_value, "daily_rate")?;
                (Some(bd_to_i64_safe(&daily_rate_bd)?), "Active".to_string())
            } else if let Some(daily_rate_bd) = plaintext_daily_rate {
                (Some(bd_to_i64_safe(&daily_rate_bd)?), "Active".to_string())
            } else {
                (None, "Missing".to_string())
            };

            grouped.entry(resource_id).or_insert(TeamMemberResponse {
                resource_id,
                name,
                role,
                department_name,
                daily_rate,
                ctc_status,
                total_allocation_pct: 0.0,
                current_allocation_percentage: 0.0,
                is_overallocated: false,
                active_assignments: Vec::new(),
            })
        };

        let allocation_pct_bd: Option<BigDecimal> = row
            .try_get("allocation_percentage")
            .map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(allocation_pct_bd) = allocation_pct_bd {
            let allocation_pct = allocation_pct_bd.to_string().parse::<f64>().map_err(|_| {
                AppError::Internal("Failed to parse allocation percentage".to_string())
            })?;

            member.total_allocation_pct += allocation_pct;
            member.current_allocation_percentage += allocation_pct;

            let project_name: Option<String> = row
                .try_get("project_name")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let start_date: Option<String> = row
                .try_get("start_date")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let end_date: Option<String> = row
                .try_get("end_date")
                .map_err(|e| AppError::Database(e.to_string()))?;

            if let (Some(project_name), Some(start_date)) = (project_name, start_date) {
                member.active_assignments.push(AssignmentSummary {
                    project_name,
                    allocation_pct,
                    start_date,
                    end_date: end_date.unwrap_or_else(|| "Ongoing".to_string()),
                });
            }
        }
    }

    let mut team_members: Vec<TeamMemberResponse> = grouped.into_values().collect();
    for member in &mut team_members {
        member.is_overallocated = member.current_allocation_percentage > 100.0;
    }
    team_members.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(team_members)
}

pub async fn get_team_members(
    pool: &PgPool,
    department_id: Option<Uuid>,
    user_role: &str,
) -> Result<Vec<TeamMemberResponse>> {
    query_team_members(pool, department_id, user_role).await
}

pub async fn get_team_members_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Option<Uuid>,
    user_role: &str,
) -> Result<Vec<TeamMemberResponse>> {
    query_team_members(&mut **tx, department_id, user_role).await
}

fn month_key(date: NaiveDate) -> String {
    format!("{:04}-{:02}", date.year(), date.month())
}

fn month_start(date: NaiveDate) -> Result<NaiveDate> {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
        .ok_or_else(|| AppError::Internal("Invalid month start date".to_string()))
}

fn next_month(date: NaiveDate) -> Result<NaiveDate> {
    let (year, month) = if date.month() == 12 {
        (date.year() + 1, 1)
    } else {
        (date.year(), date.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| AppError::Internal("Invalid next month date".to_string()))
}

/// Count weekdays (Mon–Fri) in an inclusive date range.
fn count_weekdays(start: NaiveDate, end: NaiveDate) -> i64 {
    if start > end {
        return 0;
    }
    let total_days = (end - start).num_days() + 1;
    let full_weeks = total_days / 7;
    let remainder = total_days % 7;
    let start_wd = start.weekday().num_days_from_monday(); // Mon=0 … Sun=6
    let weekend_in_remainder = (0..remainder)
        .filter(|i| {
            let wd = (start_wd + *i as u32) % 7;
            wd >= 5 // Sat=5, Sun=6
        })
        .count() as i64;
    full_weeks * 5 + (remainder - weekend_in_remainder)
}

async fn query_capacity_report_with_tx(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Option<Uuid>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<CapacityReportResponse> {
    let resources = sqlx::query(
        r#"
        SELECT r.id AS resource_id, r.name AS resource_name
        FROM resources r
        WHERE r.resource_type = 'employee'
          AND ($1::uuid IS NULL OR r.department_id = $1)
        ORDER BY r.name ASC
        "#,
    )
    .bind(department_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if resources.is_empty() {
        return Ok(CapacityReportResponse {
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
            employees: Vec::new(),
        });
    }

    let mut month_starts = Vec::new();
    let mut cursor = month_start(start_date)?;
    let last_month = month_start(end_date)?;
    while cursor <= last_month {
        month_starts.push(cursor);
        cursor = next_month(cursor)?;
    }

    let resource_ids = resources
        .iter()
        .map(|row| {
            row.try_get::<Uuid, _>("resource_id")
                .map_err(|e| AppError::Database(e.to_string()))
        })
        .collect::<Result<Vec<_>>>()?;

    let allocation_rows = sqlx::query(
        r#"
        SELECT resource_id, start_date, end_date, allocation_percentage
        FROM allocations
        WHERE resource_id = ANY($1)
          AND start_date <= $2
          AND end_date >= $3
        "#,
    )
    .bind(&resource_ids)
    .bind(end_date)
    .bind(start_date)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Precompute working days (weekdays) for each month, clamped to report range.
    let mut working_days_per_month: HashMap<String, i64> = HashMap::new();
    for month in &month_starts {
        let next = next_month(*month)?;
        let month_end = next
            .pred_opt()
            .ok_or_else(|| AppError::Internal("Invalid month end date".to_string()))?;
        let effective_start = std::cmp::max(*month, start_date);
        let effective_end = std::cmp::min(month_end, end_date);
        working_days_per_month.insert(month_key(*month), count_weekdays(effective_start, effective_end));
    }

    // (weighted_fte_days, allocation_count) per resource per month
    let mut by_resource_month: HashMap<Uuid, HashMap<String, (f64, i32)>> = HashMap::new();

    for row in allocation_rows {
        let resource_id: Uuid = row
            .try_get("resource_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let allocation_start: NaiveDate = row
            .try_get("start_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let allocation_end: NaiveDate = row
            .try_get("end_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let allocation_percentage_bd: BigDecimal = row
            .try_get("allocation_percentage")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let allocation_percentage = allocation_percentage_bd
            .to_string()
            .parse::<f64>()
            .map_err(|_| AppError::Internal("Failed to parse allocation percentage".to_string()))?;

        let overlap_start = std::cmp::max(allocation_start, start_date);
        let overlap_end = std::cmp::min(allocation_end, end_date);
        if overlap_start > overlap_end {
            continue;
        }

        for month in &month_starts {
            let period_start = *month;
            let next = next_month(*month)?;
            let period_end = next
                .pred_opt()
                .ok_or_else(|| AppError::Internal("Invalid month end date".to_string()))?;
            if overlap_start <= period_end && overlap_end >= period_start {
                // Clamp allocation to this month's boundaries
                let alloc_month_start = std::cmp::max(overlap_start, period_start);
                let alloc_month_end = std::cmp::min(overlap_end, period_end);
                let allocated_weekdays = count_weekdays(alloc_month_start, alloc_month_end);

                let period = month_key(*month);
                let entry = by_resource_month
                    .entry(resource_id)
                    .or_default()
                    .entry(period)
                    .or_insert((0.0, 0));
                // Accumulate FTE-days: weekdays × (pct / 100)
                entry.0 += allocated_weekdays as f64 * (allocation_percentage / 100.0);
                entry.1 += 1;
            }
        }
    }

    let month_set: BTreeSet<String> = month_starts.iter().map(|m| month_key(*m)).collect();

    let mut employees = Vec::new();
    for row in resources {
        let resource_id: Uuid = row
            .try_get("resource_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let resource_name: String = row
            .try_get("resource_name")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut periods = Vec::new();
        for period in &month_set {
            let (weighted_fte_days, allocation_count) = by_resource_month
                .get(&resource_id)
                .and_then(|m| m.get(period))
                .copied()
                .unwrap_or((0.0, 0));

            let month_working_days = working_days_per_month
                .get(period)
                .copied()
                .unwrap_or(0);

            let total_allocation_percentage = if month_working_days > 0 {
                (weighted_fte_days / month_working_days as f64) * 100.0
            } else {
                0.0
            };

            // Round to 1 decimal place for clean display
            let total_allocation_percentage = (total_allocation_percentage * 10.0).round() / 10.0;

            periods.push(CapacityPeriod {
                period: period.clone(),
                total_allocation_percentage,
                is_overallocated: total_allocation_percentage > 100.0,
                allocation_count,
            });
        }

        employees.push(EmployeeCapacity {
            resource_id,
            resource_name,
            periods,
        });
    }

    Ok(CapacityReportResponse {
        start_date: start_date.to_string(),
        end_date: end_date.to_string(),
        employees,
    })
}

pub async fn get_capacity_report_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Option<Uuid>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<CapacityReportResponse> {
    query_capacity_report_with_tx(tx, department_id, start_date, end_date).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::types::BigDecimal;
    use std::str::FromStr;

    #[test]
    fn bd_to_i64_safe_normal_value() {
        let bd = BigDecimal::from_str("1200000").unwrap();
        assert_eq!(bd_to_i64_safe(&bd).unwrap(), 1200000);
    }

    #[test]
    fn bd_to_i64_safe_zero() {
        let bd = BigDecimal::from_str("0").unwrap();
        assert_eq!(bd_to_i64_safe(&bd).unwrap(), 0);
    }

    #[test]
    fn bd_to_i64_safe_with_decimal_truncates() {
        let bd = BigDecimal::from_str("1200000.75").unwrap();
        assert_eq!(bd_to_i64_safe(&bd).unwrap(), 1200000);
    }

    #[test]
    fn bd_to_i64_safe_negative_value() {
        let bd = BigDecimal::from_str("-500").unwrap();
        assert_eq!(bd_to_i64_safe(&bd).unwrap(), -500);
    }

    #[test]
    fn bd_to_i64_safe_large_value() {
        let bd = BigDecimal::from_str("999999999").unwrap();
        assert_eq!(bd_to_i64_safe(&bd).unwrap(), 999999999);
    }

    #[test]
    fn team_member_sorting_is_case_insensitive() {
        let mut members = vec![
            TeamMemberResponse {
                resource_id: Uuid::new_v4(),
                name: "Zara".to_string(),
                role: "engineer".to_string(),
                department_name: "Eng".to_string(),
                daily_rate: Some(1000000),
                ctc_status: "Active".to_string(),
                total_allocation_pct: 50.0,
                current_allocation_percentage: 50.0,
                is_overallocated: false,
                active_assignments: vec![],
            },
            TeamMemberResponse {
                resource_id: Uuid::new_v4(),
                name: "alice".to_string(),
                role: "designer".to_string(),
                department_name: "Design".to_string(),
                daily_rate: Some(900000),
                ctc_status: "Active".to_string(),
                total_allocation_pct: 80.0,
                current_allocation_percentage: 80.0,
                is_overallocated: false,
                active_assignments: vec![],
            },
            TeamMemberResponse {
                resource_id: Uuid::new_v4(),
                name: "Bob".to_string(),
                role: "pm".to_string(),
                department_name: "Eng".to_string(),
                daily_rate: None,
                ctc_status: "Missing".to_string(),
                total_allocation_pct: 0.0,
                current_allocation_percentage: 0.0,
                is_overallocated: false,
                active_assignments: vec![],
            },
        ];

        members.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        assert_eq!(members[0].name, "alice");
        assert_eq!(members[1].name, "Bob");
        assert_eq!(members[2].name, "Zara");
    }
}
