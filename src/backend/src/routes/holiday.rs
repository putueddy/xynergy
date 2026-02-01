use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

/// Holiday response structure
#[derive(Debug, Serialize)]
pub struct Holiday {
    pub id: Uuid,
    pub name: String,
    pub date: String,
    pub description: Option<String>,
}

/// Create holiday request
#[derive(Debug, Deserialize)]
pub struct CreateHolidayRequest {
    pub name: String,
    pub date: String,
    pub description: Option<String>,
}

/// Update holiday request
#[derive(Debug, Deserialize)]
pub struct UpdateHolidayRequest {
    pub name: Option<String>,
    pub date: Option<String>,
    pub description: Option<String>,
}

/// Get all holidays
pub async fn get_holidays(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Holiday>>, AppError> {
    let holidays = sqlx::query_as!(
        Holiday,
        r#"
        SELECT 
            id,
            name,
            date::TEXT as "date!",
            description
        FROM holidays
        ORDER BY date
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(holidays))
}

/// Get holiday by ID
pub async fn get_holiday(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Holiday>, AppError> {
    let holiday = sqlx::query_as!(
        Holiday,
        r#"
        SELECT 
            id,
            name,
            date::TEXT as "date!",
            description
        FROM holidays
        WHERE id = $1
        "#,
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(holiday))
}

/// Create a new holiday
pub async fn create_holiday(
    State(pool): State<PgPool>,
    Json(req): Json<CreateHolidayRequest>,
) -> Result<(StatusCode, Json<Holiday>), AppError> {
    let date = chrono::NaiveDate::parse_from_str(&req.date, "%Y-%m-%d")
        .map_err(|e| AppError::Validation(format!("Invalid date format: {}", e)))?;
    
    let holiday = sqlx::query_as!(
        Holiday,
        r#"
        INSERT INTO holidays (name, date, description)
        VALUES ($1, $2, $3)
        RETURNING 
            id,
            name,
            date::TEXT as "date!",
            description
        "#,
        req.name,
        date,
        req.description
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(holiday)))
}

/// Update a holiday
pub async fn update_holiday(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateHolidayRequest>,
) -> Result<Json<Holiday>, AppError> {
    let date = req.date.map(|d| {
        chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d")
            .map_err(|e| AppError::Validation(format!("Invalid date format: {}", e)))
    }).transpose()?;
    
    let holiday = sqlx::query_as!(
        Holiday,
        r#"
        UPDATE holidays
        SET 
            name = COALESCE($2, name),
            date = COALESCE($3, date),
            description = COALESCE($4, description),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $1
        RETURNING 
            id,
            name,
            date::TEXT as "date!",
            description
        "#,
        id,
        req.name,
        date,
        req.description
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(holiday))
}

/// Delete a holiday
pub async fn delete_holiday(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    sqlx::query!(
        "DELETE FROM holidays WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

use axum::routing::{get, post, put, delete};
use axum::Router;

/// Holiday routes
pub fn holiday_routes() -> Router<PgPool> {
    Router::new()
        .route("/holidays", get(get_holidays).post(create_holiday))
        .route("/holidays/:id", get(get_holiday).put(update_holiday).delete(delete_holiday))
}