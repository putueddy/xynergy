use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Department model
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Department {
    pub id: Uuid,
    pub name: String,
}
