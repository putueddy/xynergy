//! CTC completeness reporting service.

use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{AppError, Result};

#[derive(Debug, Serialize)]
pub struct DepartmentCompleteness {
    pub department_id: Uuid,
    pub department: String,
    pub total_employees: i64,
    pub with_ctc: i64,
    pub missing_ctc: i64,
    pub completion_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct CompletenessReport {
    pub departments: Vec<DepartmentCompleteness>,
    pub total_employees: i64,
    pub total_with_ctc: i64,
    pub total_missing: i64,
    pub overall_completion_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct MissingCtcEmployee {
    pub id: Uuid,
    pub name: String,
    pub department: String,
}

fn percent(numerator: i64, denominator: i64) -> f64 {
    if denominator <= 0 {
        return 0.0;
    }
    (numerator as f64 / denominator as f64) * 100.0
}

pub async fn get_completeness_summary(
    pool: &PgPool,
    department_id: Option<Uuid>,
) -> Result<CompletenessReport> {
    let rows = sqlx::query(
        r#"
        SELECT
            d.id AS department_id,
            d.name AS department,
            COUNT(r.id) AS total_employees,
            COUNT(c.resource_id) AS with_ctc
        FROM departments d
        LEFT JOIN resources r
            ON r.department_id = d.id
           AND r.resource_type = 'employee'
        LEFT JOIN ctc_records c
            ON c.resource_id = r.id
           AND c.status = 'Active'
        WHERE ($1::uuid IS NULL OR d.id = $1)
        GROUP BY d.id, d.name
        ORDER BY d.name
        "#,
    )
    .bind(department_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut departments = Vec::with_capacity(rows.len());
    let mut total_employees = 0i64;
    let mut total_with_ctc = 0i64;

    for row in rows {
        let total_department: i64 = row
            .try_get::<i64, _>("total_employees")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let with_ctc: i64 = row
            .try_get::<i64, _>("with_ctc")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let missing_ctc = total_department.saturating_sub(with_ctc);

        total_employees += total_department;
        total_with_ctc += with_ctc;

        departments.push(DepartmentCompleteness {
            department_id: row
                .try_get::<Uuid, _>("department_id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            department: row
                .try_get::<String, _>("department")
                .map_err(|e| AppError::Database(e.to_string()))?,
            total_employees: total_department,
            with_ctc,
            missing_ctc,
            completion_pct: percent(with_ctc, total_department),
        });
    }

    let total_missing = total_employees.saturating_sub(total_with_ctc);

    Ok(CompletenessReport {
        departments,
        total_employees,
        total_with_ctc,
        total_missing,
        overall_completion_pct: percent(total_with_ctc, total_employees),
    })
}

pub async fn get_missing_employees(
    pool: &PgPool,
    department_id: Option<Uuid>,
) -> Result<Vec<MissingCtcEmployee>> {
    let rows = sqlx::query(
        r#"
        SELECT
            r.id,
            r.name,
            d.name AS department
        FROM resources r
        JOIN departments d ON d.id = r.department_id
        LEFT JOIN ctc_records c
            ON c.resource_id = r.id
           AND c.status = 'Active'
        WHERE r.resource_type = 'employee'
          AND c.resource_id IS NULL
          AND ($1::uuid IS NULL OR d.id = $1)
        ORDER BY d.name, r.name
        "#,
    )
    .bind(department_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut employees = Vec::with_capacity(rows.len());
    for row in rows {
        employees.push(MissingCtcEmployee {
            id: row
                .try_get::<Uuid, _>("id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            name: row
                .try_get::<String, _>("name")
                .map_err(|e| AppError::Database(e.to_string()))?,
            department: row
                .try_get::<String, _>("department")
                .map_err(|e| AppError::Database(e.to_string()))?,
        });
    }

    Ok(employees)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_with_zero_denominator() {
        assert_eq!(percent(5, 0), 0.0);
    }

    #[test]
    fn test_percent_with_values() {
        assert_eq!(percent(5, 10), 50.0);
    }
}
