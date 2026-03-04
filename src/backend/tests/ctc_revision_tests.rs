//! Integration tests for CTC Revision Management

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::{Datelike, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("ctc-rev-test-{}@example.com", Uuid::new_v4())
}

fn set_ctc_crypto_env() {
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );
}

async fn create_test_user_with_role(pool: &PgPool, email: &str, role: &str) -> Uuid {
    let password_hash = xynergy_backend::routes::auth::hash_password("Password123!")
        .expect("password hashing should succeed");

    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, first_name, last_name, role)
         VALUES ($1, $2, 'Test', 'User', $3)
         RETURNING id",
    )
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn create_test_resource(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO resources (name, resource_type, capacity)
         VALUES ($1, 'human', 1.0)
         RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn get_auth_token(app: &axum::Router, email: &str) -> String {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"email": email, "password": "Password123!"}).to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    json["token"].as_str().unwrap().to_string()
}

async fn create_ctc_record(app: &axum::Router, hr_token: &str, resource_id: Uuid) -> Value {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": 15000000,
                "hra_allowance": 3000000,
                "medical_allowance": 1000000,
                "transport_allowance": 500000,
                "meal_allowance": 500000,
                "working_days_per_month": 22
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn build_valid_components_from_preview(
    app: &axum::Router,
    token: &str,
    resource_id: Uuid,
    base_salary: i64,
    hra_allowance: i64,
    medical_allowance: i64,
    transport_allowance: i64,
    meal_allowance: i64,
    working_days_per_month: i32,
    risk_tier: i32,
) -> Value {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc/calculate")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": base_salary,
                "hra_allowance": hra_allowance,
                "medical_allowance": medical_allowance,
                "transport_allowance": transport_allowance,
                "meal_allowance": meal_allowance,
                "working_days_per_month": working_days_per_month,
                "risk_tier": risk_tier
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let preview: Value = serde_json::from_slice(&body).unwrap();

    json!({
        "base_salary": preview["base_salary"],
        "hra_allowance": preview["allowances"]["hra"],
        "medical_allowance": preview["allowances"]["medical"],
        "transport_allowance": preview["allowances"]["transport"],
        "meal_allowance": preview["allowances"]["meal"],
        "bpjs_kesehatan_employer": preview["bpjs"]["kesehatan"]["employer"],
        "bpjs_ketenagakerjaan_employer": preview["bpjs"]["ketenagakerjaan"]["employer"],
        "thr_monthly_accrual": preview["thr_monthly_accrual"],
        "total_monthly_ctc": preview["total_monthly_ctc"],
        "daily_rate": format!("{:.2}", preview["daily_rate"].as_f64().unwrap_or(0.0)),
        "working_days_per_month": preview["working_days_per_month"],
        "risk_tier": risk_tier,
        "thr_eligible": true
    })
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_ctc_creates_revision(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Revision Test Employee").await;
    create_ctc_record(&app, &hr_token, resource_id).await;

    let components = build_valid_components_from_preview(
        &app,
        &hr_token,
        resource_id,
        20000000,
        3000000,
        1000000,
        500000,
        500000,
        22,
        1,
    )
    .await;

    // Update CTC Component
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/ctc/{}/components", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "components": components,
                "reason": "Promoted to Senior level",
                "effective_date_policy": "pro_rata"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "Update should succeed, got: {}",
        body_str
    );

    // Verify revision was created
    let revision_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM ctc_revisions WHERE resource_id = $1")
            .bind(resource_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    // There should be 2 revisions: the initial creation and the update.
    assert_eq!(
        revision_count, 2,
        "Revisions should be persisted for creates and updates"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_ctc_history_endpoint(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Revision Test Employee").await;
    create_ctc_record(&app, &hr_token, resource_id).await;

    // Call history endpoint
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/{}/history", resource_id))
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "History endpoint should exist and return OK"
    );

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let history: Value = serde_json::from_slice(&body).unwrap();

    let entries = history["history"]
        .as_array()
        .expect("Expected history array");
    assert_eq!(entries.len(), 1, "Should have one entry from create");
    assert_eq!(
        entries[0]["reason"].as_str().unwrap(),
        "Initial CTC record creation"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_non_hr_cannot_update_or_view_history(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let non_hr_email = test_email();
    let _non_hr_id = create_test_user_with_role(&pool, &non_hr_email, "department_head").await;
    let non_hr_token = get_auth_token(&app, &non_hr_email).await;

    let resource_id = create_test_resource(&pool, "RBAC Revision Test Employee").await;
    create_ctc_record(&app, &hr_token, resource_id).await;

    let req_update = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/ctc/{}/components", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", non_hr_token))
        .body(Body::from(
            json!({
                "components": {
                    "base_salary": 21000000,
                    "daily_rate": "840000"
                },
                "reason": "Unauthorized change"
            })
            .to_string(),
        ))
        .unwrap();

    let res_update = app.clone().oneshot(req_update).await.unwrap();
    assert_eq!(res_update.status(), StatusCode::FORBIDDEN);

    let req_history = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/{}/history", resource_id))
        .header("Authorization", format!("Bearer {}", non_hr_token))
        .body(Body::empty())
        .unwrap();

    let res_history = app.clone().oneshot(req_history).await.unwrap();
    assert_eq!(res_history.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_revision_storage_is_ciphertext_not_plaintext(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Encryption Revision Test Employee").await;
    create_ctc_record(&app, &hr_token, resource_id).await;

    let components = build_valid_components_from_preview(
        &app,
        &hr_token,
        resource_id,
        22000000,
        4000000,
        1000000,
        600000,
        600000,
        22,
        1,
    )
    .await;

    let req_update = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/ctc/{}/components", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "components": components,
                "reason": "Comp review update"
            })
            .to_string(),
        ))
        .unwrap();

    let res_update = app.clone().oneshot(req_update).await.unwrap();
    assert_eq!(res_update.status(), StatusCode::OK);

    let row = sqlx::query(
        "SELECT encrypted_components, encrypted_daily_rate
         FROM ctc_revisions
         WHERE resource_id = $1
         ORDER BY revision_number DESC
         LIMIT 1",
    )
    .bind(resource_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    use sqlx::Row;
    let encrypted_components: String = row.get("encrypted_components");
    let encrypted_daily_rate: Option<String> = row.get("encrypted_daily_rate");

    assert!(!encrypted_components.is_empty());
    assert!(!encrypted_components.contains("22000000"));
    assert!(!encrypted_components.contains("4000000"));

    let encrypted_daily_rate =
        encrypted_daily_rate.expect("encrypted_daily_rate should be present");
    assert!(!encrypted_daily_rate.is_empty());
    assert!(!encrypted_daily_rate.contains("900000"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_effective_date_policy_is_configurable_and_applied(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Policy Revision Test Employee").await;
    create_ctc_record(&app, &hr_token, resource_id).await;

    let components = build_valid_components_from_preview(
        &app,
        &hr_token,
        resource_id,
        20000000,
        3000000,
        1000000,
        500000,
        500000,
        22,
        1,
    )
    .await;

    let req_update = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/ctc/{}/components", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "components": components,
                "reason": "Policy behavior verification",
                "effective_date_policy": "effective_first_of_month"
            })
            .to_string(),
        ))
        .unwrap();

    let res_update = app.clone().oneshot(req_update).await.unwrap();
    assert_eq!(res_update.status(), StatusCode::OK);

    let body = to_bytes(res_update.into_body(), usize::MAX).await.unwrap();
    let parsed: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed["policy"], "effective_first_of_month");

    let today = Utc::now().date_naive();
    let expected_effective = if today.day() == 1 {
        today
    } else if today.month() == 12 {
        chrono::NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap()
    } else {
        chrono::NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1).unwrap()
    };

    assert_eq!(parsed["effective_date"], expected_effective.to_string());

    let stored_policy: Option<String> = sqlx::query_scalar(
        "SELECT value FROM global_settings WHERE key = 'ctc_effective_date_policy'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert_eq!(stored_policy.as_deref(), Some("effective_first_of_month"));
}
