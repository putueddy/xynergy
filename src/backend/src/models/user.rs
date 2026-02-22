use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User model for database queries
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub department_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Auth security fields
    pub login_attempts: Option<i32>,
    pub locked_until: Option<DateTime<Utc>>,
    #[serde(skip_serializing)]
    pub refresh_token_hash: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
}

/// User response model (without sensitive data)
#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub department_id: Option<Uuid>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            first_name: user.first_name,
            last_name: user.last_name,
            role: user.role,
            department_id: user.department_id,
        }
    }
}
