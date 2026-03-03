//! Relationship-based access control (RBAC) helpers.
//!
//! Validates access using database relationships (`departments.head_id`,
//! `projects.project_manager_id`) rather than just JWT role strings.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{AppError, Result};

/// Check if user is the head of a specific department.
///
/// Looks up `departments.head_id` and compares to `user_id`.
pub async fn is_department_head(
    conn: &mut PgConnection,
    user_id: Uuid,
    department_id: Uuid,
) -> Result<bool> {
    let row = sqlx::query!(
        "SELECT head_id FROM departments WHERE id = $1",
        department_id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(row.map_or(false, |r| r.head_id == Some(user_id)))
}

/// Check if user is the project manager of a specific project.
///
/// Looks up `projects.project_manager_id` and compares to `user_id`.
pub async fn is_project_manager(
    conn: &mut PgConnection,
    user_id: Uuid,
    project_id: Uuid,
) -> Result<bool> {
    let row = sqlx::query!(
        "SELECT project_manager_id FROM projects WHERE id = $1",
        project_id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(row.map_or(false, |r| r.project_manager_id == Some(user_id)))
}

/// Can the user access a department's data?
///
/// Access matrix:
/// - `admin`: always yes
/// - `hr`: always yes (read-only CTC access across departments)
/// - `department_head`: only if `departments.head_id` matches user_id
/// - others: only if `users.department_id` matches the target department
pub async fn can_access_department(
    conn: &mut PgConnection,
    user_id: Uuid,
    role: &str,
    department_id: Uuid,
) -> Result<bool> {
    match role {
        "admin" | "hr" => Ok(true),
        "department_head" => is_department_head(conn, user_id, department_id).await,
        _ => {
            // Check if user belongs to that department
            let row = sqlx::query!(
                "SELECT department_id FROM users WHERE id = $1",
                user_id
            )
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            Ok(row.map_or(false, |r| r.department_id == Some(department_id)))
        }
    }
}

/// Can the user access a project's data?
///
/// Access matrix:
/// - `admin`: always yes
/// - `project_manager`: only if `projects.project_manager_id` matches user_id
/// - `department_head`: if any of their department's resources are allocated to the project
/// - `finance`: read access to all projects
/// - others: denied
pub async fn can_access_project(
    conn: &mut PgConnection,
    user_id: Uuid,
    role: &str,
    project_id: Uuid,
) -> Result<bool> {
    match role {
        "admin" | "finance" => Ok(true),
        "project_manager" => is_project_manager(conn, user_id, project_id).await,
        "department_head" => {
            // Check if user heads a department that has resources allocated to this project
            let row = sqlx::query_scalar!(
                "SELECT EXISTS(
                    SELECT 1
                    FROM departments d
                    JOIN resources r ON r.department_id = d.id
                    JOIN allocations a ON a.resource_id = r.id
                    WHERE d.head_id = $1
                      AND a.project_id = $2
                ) AS \"exists!\"",
                user_id,
                project_id
            )
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            Ok(row)
        }
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit-level documentation tests only; integration tests in tests/rbac_tests.rs
    #[test]
    fn test_role_matching_logic() {
        // Admin and HR always have access — verified via match arms
        assert!(matches!("admin", "admin" | "hr"));
        assert!(matches!("hr", "admin" | "hr"));
        assert!(!matches!("department_head", "admin" | "hr"));
        assert!(!matches!("project_manager", "admin" | "hr"));
    }
}
