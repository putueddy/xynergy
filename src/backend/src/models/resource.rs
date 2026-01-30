use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

/// Resource model
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Resource {
    pub id: Uuid,
    pub name: String,
    pub resource_type: String,
    pub capacity: BigDecimal,
    pub department_id: Option<Uuid>,
    pub skills: Value,
}
