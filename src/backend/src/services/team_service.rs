use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Row, Transaction};
use sqlx::types::BigDecimal;
use std::collections::HashMap;
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
    pub active_assignments: Vec<AssignmentSummary>,
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
            AppError::Internal(format!("Failed to parse '{}' from decrypted payload", field))
        }),
        serde_json::Value::Number(n) => n.to_string().parse::<BigDecimal>().map_err(|_| {
            AppError::Internal(format!("Failed to parse '{}' from decrypted payload", field))
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
        LEFT JOIN allocations a ON a.resource_id = r.id AND (a.end_date >= CURRENT_DATE OR a.end_date IS NULL)
        LEFT JOIN projects p ON p.id = a.project_id
        WHERE r.resource_type = 'human'
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
                active_assignments: Vec::new(),
            })
        };

        let allocation_pct_bd: Option<BigDecimal> = row
            .try_get("allocation_percentage")
            .map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(allocation_pct_bd) = allocation_pct_bd {
            let allocation_pct = allocation_pct_bd
                .to_string()
                .parse::<f64>()
                .map_err(|_| AppError::Internal("Failed to parse allocation percentage".to_string()))?;

            member.total_allocation_pct += allocation_pct;

            let project_name: Option<String> = row
                .try_get("project_name")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let start_date: Option<String> = row
                .try_get("start_date")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let end_date: Option<String> = row
                .try_get("end_date")
                .map_err(|e| AppError::Database(e.to_string()))?;

            if let (Some(project_name), Some(start_date)) =
                (project_name, start_date)
            {
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
                active_assignments: vec![],
            },
        ];

        members.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        assert_eq!(members[0].name, "alice");
        assert_eq!(members[1].name, "Bob");
        assert_eq!(members[2].name, "Zara");
    }
}
