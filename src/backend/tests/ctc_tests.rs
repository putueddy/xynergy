//! Integration tests for CTC (Cost to Company) management

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("ctc-test-{}@example.com", Uuid::new_v4())
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
        .expect("password hashing should succeed in tests");

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
    .expect("test user should be created")
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
    .expect("test resource should be created")
}

async fn get_auth_token(app: &axum::Router, email: &str) -> String {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": "Password123!"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("login should return response");
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    json["token"].as_str().unwrap().to_string()
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_can_create_ctc_record(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    // Create HR user
    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    // Create a resource (employee)
    let resource_id = create_test_resource(&pool, "Test Employee").await;

    // Create CTC record
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
                "working_days_per_month": 22,
                "risk_tier": 1
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("create should return response");
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "HR should be able to create CTC record"
    );

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response structure
    assert_eq!(
        json["resource_id"].as_str().unwrap(),
        resource_id.to_string()
    );
    assert_eq!(json["base_salary"].as_i64().unwrap(), 15000000);
    assert!(json["total_monthly_ctc"].as_i64().unwrap() > 0);
    assert!(json["daily_rate"].as_f64().unwrap() > 0.0);
    assert_eq!(json["working_days_per_month"].as_i64().unwrap(), 22);

    // Verify BPJS calculations are present
    assert!(json["bpjs"]["kesehatan"]["employer"].as_i64().unwrap() >= 0);
    assert!(json["bpjs"]["kesehatan"]["employee"].as_i64().unwrap() >= 0);
    assert!(
        json["bpjs"]["ketenagakerjaan"]["employer"]
            .as_i64()
            .unwrap()
            >= 0
    );
    assert!(
        json["bpjs"]["ketenagakerjaan"]["employee"]
            .as_i64()
            .unwrap()
            >= 0
    );

    // Verify record was created in database
    let record_exists = sqlx::query("SELECT resource_id FROM ctc_records WHERE resource_id = $1")
        .bind(resource_id)
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert!(
        record_exists.is_some(),
        "CTC record should exist in database"
    );

    // Verify audit log entry
    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM audit_logs 
         WHERE action = 'CREATE' 
         AND entity_type = 'ctc_record'
         AND entity_id = $1",
    )
    .bind(resource_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count, 1, "Audit log entry should be created");
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_hr_cannot_create_ctc_record(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    // Create non-HR user (e.g., department_head)
    let user_email = test_email();
    let _user_id = create_test_user_with_role(&pool, &user_email, "department_head").await;
    let user_token = get_auth_token(&app, &user_email).await;

    let resource_id = create_test_resource(&pool, "Test Employee").await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", user_token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": 15000000,
                "hra_allowance": 3000000,
                "medical_allowance": 1000000,
                "transport_allowance": 500000,
                "meal_allowance": 500000,
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("create should return response");
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "Non-HR should be denied"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_cannot_create_ctc_record(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    let resource_id = create_test_resource(&pool, "Admin Denied Employee").await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", admin_token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": 15000000,
                "hra_allowance": 3000000,
                "medical_allowance": 1000000,
                "transport_allowance": 500000,
                "meal_allowance": 500000,
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("create should return response");
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "Admin should be denied for HR-only endpoint"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_ctc_rejects_negative_values(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Test Employee").await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": -1000000,  // Negative value
                "hra_allowance": 3000000,
                "medical_allowance": 1000000,
                "transport_allowance": 500000,
                "meal_allowance": 500000,
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("create should return response");
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Negative values should be rejected"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_ctc_rejects_duplicate_resource(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Test Employee").await;

    // First creation
    let req1 = Request::builder()
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
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::OK);

    // Second creation attempt (should fail)
    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": 20000000,
                "hra_allowance": 4000000,
                "medical_allowance": 1000000,
                "transport_allowance": 500000,
                "meal_allowance": 500000,
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(
        res2.status(),
        StatusCode::BAD_REQUEST,
        "Duplicate CTC record should be rejected"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn calculate_bpjs_preview_works(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Test Employee").await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc/calculate")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": 10000000,
                "hra_allowance": 2000000,
                "medical_allowance": 1000000,
                "transport_allowance": 500000,
                "meal_allowance": 500000,
                "working_days_per_month": 22,
                "risk_tier": 1
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify calculations
    // Basis: 10M + 2M + 1M + 500K + 500K = 14M
    // Kesehatan capped at 12M: employer = 480K, employee = 120K
    assert_eq!(
        json["bpjs"]["kesehatan"]["employer"].as_i64().unwrap(),
        480000
    );
    assert_eq!(
        json["bpjs"]["kesehatan"]["employee"].as_i64().unwrap(),
        120000
    );

    // Verify total CTC is calculated
    assert!(json["total_monthly_ctc"].as_i64().unwrap() > 0);

    // Verify daily rate = total / 22
    let total = json["total_monthly_ctc"].as_i64().unwrap() as f64;
    let daily = json["daily_rate"].as_f64().unwrap();
    assert!(
        (daily - (total / 22.0)).abs() < 1.0,
        "Daily rate should equal total / working_days"
    );
}
