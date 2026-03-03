use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::{audit_payload, log_audit, user_id_from_headers};

/// Resource response structure using f64 for capacity
#[derive(Debug, Serialize)]
pub struct ResourceResponse {
    pub id: Uuid,
    pub name: String,
    pub resource_type: String,
    pub capacity: Option<f64>,
    pub department_id: Option<Uuid>,
    pub department_name: Option<String>,
    pub skills: Option<serde_json::Value>,
    pub employment_start_date: Option<NaiveDate>,
}

/// Create resource request
#[derive(Debug, Deserialize)]
pub struct CreateResourceRequest {
    pub name: String,
    pub resource_type: String,
    pub capacity: Option<f64>,
    pub department_id: Option<Uuid>,
    pub skills: Option<serde_json::Value>,
    pub employment_start_date: Option<NaiveDate>,
}

/// Update resource request
#[derive(Debug, Deserialize)]
pub struct UpdateResourceRequest {
    pub name: Option<String>,
    pub resource_type: Option<String>,
    pub capacity: Option<f64>,
    pub department_id: Option<Uuid>,
    pub skills: Option<serde_json::Value>,
    pub employment_start_date: Option<NaiveDate>,
}

/// Convert BigDecimal to f64
fn bigdecimal_to_f64(bd: Option<sqlx::types::BigDecimal>) -> Option<f64> {
    bd.and_then(|d| d.to_string().parse::<f64>().ok())
}

/// Convert f64 to BigDecimal
fn f64_to_bigdecimal(f: Option<f64>) -> Option<sqlx::types::BigDecimal> {
    f.and_then(|v| sqlx::types::BigDecimal::try_from(v).ok())
}

/// Get all resources
async fn get_resources(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<Vec<ResourceResponse>>> {
    let claims = crate::services::audit_log::user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    let mut tx = crate::services::begin_rls_transaction(&pool, &headers).await?;

    let resources = sqlx::query!(
        r#"SELECT r.id, r.name, r.resource_type, r.capacity, r.department_id, r.skills, r.employment_start_date,
                d.name AS "department_name?"
         FROM resources r
         LEFT JOIN departments d ON d.id = r.department_id
         ORDER BY r.name"#
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Application level defense-in-depth filtering
    let mut user_dept_id = None;
    if claims.role == "department_head" {
        if let Ok(uid) = Uuid::parse_str(&claims.sub) {
            let row = sqlx::query!("SELECT department_id FROM users WHERE id = $1", uid)
                .fetch_optional(&pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
            if let Some(rec) = row {
                user_dept_id = rec.department_id;
            }
        }
    }

    let response: Vec<ResourceResponse> = resources
        .into_iter()
        .filter(|r| {
            if claims.role == "department_head" {
                r.department_id == user_dept_id && user_dept_id.is_some()
            } else {
                true
            }
        })
        .map(|r| ResourceResponse {
            id: r.id,
            name: r.name,
            resource_type: r.resource_type,
            capacity: bigdecimal_to_f64(r.capacity),
            department_id: r.department_id,
            department_name: r.department_name,
            skills: r.skills,
            employment_start_date: r.employment_start_date,
        })
        .collect();

    Ok(Json(response))
}

/// Get resource by ID
async fn get_resource(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ResourceResponse>> {
    let claims = crate::services::audit_log::user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    let mut tx = crate::services::begin_rls_transaction(&pool, &headers).await?;

    let resource = sqlx::query!(
        r#"SELECT r.id, r.name, r.resource_type, r.capacity, r.department_id, r.skills, r.employment_start_date,
                d.name AS "department_name?"
         FROM resources r
         LEFT JOIN departments d ON d.id = r.department_id
         WHERE r.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let resource =
        resource.ok_or_else(|| AppError::NotFound(format!("Resource {} not found", id)))?;

    // Application level defense-in-depth filtering
    if claims.role == "department_head" {
        if let Ok(uid) = Uuid::parse_str(&claims.sub) {
            let row = sqlx::query!("SELECT department_id FROM users WHERE id = $1", uid)
                .fetch_optional(&pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
            if let Some(rec) = row {
                if rec.department_id != resource.department_id || rec.department_id.is_none() {
                    return Err(AppError::Forbidden(
                        "Insufficient permissions (Department Isolation)".to_string(),
                    ));
                }
            } else {
                return Err(AppError::Forbidden("Insufficient permissions".to_string()));
            }
        }
    }

    Ok(Json(ResourceResponse {
        id: resource.id,
        name: resource.name,
        resource_type: resource.resource_type,
        capacity: bigdecimal_to_f64(resource.capacity),
        department_id: resource.department_id,
        department_name: resource.department_name,
        skills: resource.skills,
        employment_start_date: resource.employment_start_date,
    }))
}

/// Create a new resource
async fn create_resource(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(req): Json<CreateResourceRequest>,
) -> Result<Json<ResourceResponse>> {
    let audit_changes = audit_payload(
        None,
        Some(json!({
            "name": req.name.clone(),
            "resource_type": req.resource_type.clone(),
            "capacity": req.capacity,
            "department_id": req.department_id,
            "skills": req.skills.clone(),
            "employment_start_date": req.employment_start_date,
        })),
    );
    let user_id = user_id_from_headers(&headers)?;

    let capacity_decimal = f64_to_bigdecimal(req.capacity);

    let resource = sqlx::query!(
        "INSERT INTO resources (name, resource_type, capacity, department_id, skills, employment_start_date)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, name, resource_type, capacity, department_id, skills, employment_start_date",
        req.name,
        req.resource_type,
        capacity_decimal,
        req.department_id,
        req.skills,
        req.employment_start_date
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        user_id,
        "create",
        "resource",
        resource.id,
        audit_changes,
    )
    .await?;

    Ok(Json(ResourceResponse {
        id: resource.id,
        name: resource.name,
        resource_type: resource.resource_type,
        capacity: bigdecimal_to_f64(resource.capacity),
        department_id: resource.department_id,
        department_name: None,
        skills: resource.skills,
        employment_start_date: resource.employment_start_date,
    }))
}

/// Update a resource
async fn update_resource(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateResourceRequest>,
) -> Result<Json<ResourceResponse>> {
    // Check if resource exists
    let existing = sqlx::query!(
        "SELECT id, name, resource_type, capacity, department_id, skills, employment_start_date FROM resources WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Resource {} not found", id)))?;
    let existing = existing;

    // Convert capacity
    let capacity_decimal = f64_to_bigdecimal(req.capacity);
    let before_capacity = bigdecimal_to_f64(existing.capacity);
    let before_name = existing.name.clone();
    let before_type = existing.resource_type.clone();
    let before_department_id = existing.department_id;
    let before_skills = existing.skills.clone();
    let before_employment_start_date = existing.employment_start_date;
    let after_name_default = existing.name;
    let after_type_default = existing.resource_type;
    let after_capacity_default = before_capacity;
    let after_department_default = before_department_id;
    let after_skills_default = existing.skills;
    let after_employment_start_date_default = existing.employment_start_date;
    let audit_changes = audit_payload(
        Some(json!({
            "name": before_name,
            "resource_type": before_type,
            "capacity": before_capacity,
            "department_id": before_department_id,
            "skills": before_skills,
            "employment_start_date": before_employment_start_date,
        })),
        Some(json!({
            "name": req.name.clone().unwrap_or_else(|| after_name_default),
            "resource_type": req.resource_type.clone().unwrap_or_else(|| after_type_default),
            "capacity": req.capacity.or(after_capacity_default),
            "department_id": req.department_id.or(after_department_default),
            "skills": req.skills.clone().or(after_skills_default),
            "employment_start_date": req.employment_start_date.or(after_employment_start_date_default),
        })),
    );
    let user_id = user_id_from_headers(&headers)?;

    // Update with new values or keep existing
    let resource = sqlx::query!(
        "UPDATE resources 
         SET name = COALESCE($1, name),
             resource_type = COALESCE($2, resource_type),
             capacity = COALESCE($3, capacity),
             department_id = COALESCE($4, department_id),
             skills = COALESCE($5, skills),
             employment_start_date = COALESCE($6, employment_start_date)
         WHERE id = $7
         RETURNING id, name, resource_type, capacity, department_id, skills, employment_start_date",
        req.name.as_ref(),
        req.resource_type.as_ref(),
        capacity_decimal,
        req.department_id,
        req.skills,
        req.employment_start_date,
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    log_audit(
        &pool,
        user_id,
        "update",
        "resource",
        resource.id,
        audit_changes,
    )
    .await?;

    Ok(Json(ResourceResponse {
        id: resource.id,
        name: resource.name,
        resource_type: resource.resource_type,
        capacity: bigdecimal_to_f64(resource.capacity),
        department_id: resource.department_id,
        department_name: None,
        skills: resource.skills,
        employment_start_date: resource.employment_start_date,
    }))
}

/// Delete a resource
async fn delete_resource(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check if resource exists
    let existing = sqlx::query!(
        "SELECT id, name, resource_type, capacity, department_id, skills, employment_start_date FROM resources WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Resource {} not found", id)))?;

    // Delete the resource
    sqlx::query!("DELETE FROM resources WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let user_id = user_id_from_headers(&headers)?;
    let audit_changes = audit_payload(
        Some(json!({
            "name": existing.name,
            "resource_type": existing.resource_type,
            "capacity": bigdecimal_to_f64(existing.capacity),
            "department_id": existing.department_id,
            "skills": existing.skills,
            "employment_start_date": existing.employment_start_date,
        })),
        None,
    );
    log_audit(&pool, user_id, "delete", "resource", id, audit_changes).await?;

    Ok(Json(json!({"message": "Resource deleted successfully"})))
}

/// Create resource routes
pub fn resource_routes() -> Router<PgPool> {
    Router::new()
        .route("/resources", get(get_resources).post(create_resource))
        .route(
            "/resources/:id",
            get(get_resource)
                .put(update_resource)
                .delete(delete_resource),
        )
}
