use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("thr-test-{}@example.com", Uuid::new_v4())
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

    let res = app.clone().oneshot(req).await.expect("login should return response");
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    json["token"].as_str().unwrap().to_string()
}

async fn create_ctc_record_for_resource(app: &axum::Router, token: &str, resource_id: Uuid) {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": 12000000,
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

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("CTC create should succeed");
    assert_eq!(res.status(), StatusCode::OK, "CTC record creation failed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_can_configure_thr(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "THR Configure Employee").await;
    create_ctc_record_for_resource(&app, &hr_token, resource_id).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/thr/configure/{}", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "thr_eligible": true,
                "thr_calculation_basis": "full",
                "thr_employment_start_date": "2025-06-01"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("configure should return response");
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["thr_eligible"].as_bool(), Some(true));
    assert_eq!(json["thr_calculation_basis"].as_str(), Some("full"));
    assert_eq!(json["employment_start_date"].as_str(), Some("2025-06-01"));

    let start_date: Option<chrono::NaiveDate> = sqlx::query_scalar(
        "SELECT employment_start_date FROM resources WHERE id = $1",
    )
    .bind(resource_id)
    .fetch_one(&pool)
    .await
    .expect("resource employment_start_date should be readable");
    assert_eq!(start_date.map(|d| d.to_string()), Some("2025-06-01".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_hr_cannot_configure_thr(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let user_email = test_email();
    let _user_id = create_test_user_with_role(&pool, &user_email, "department_head").await;
    let user_token = get_auth_token(&app, &user_email).await;

    let resource_id = create_test_resource(&pool, "THR Forbidden Employee").await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/thr/configure/{}", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", user_token))
        .body(Body::from(
            json!({
                "thr_eligible": true,
                "thr_calculation_basis": "full",
                "thr_employment_start_date": "2025-06-01"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("configure should return response");
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_can_get_thr_config(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "THR Get Config Employee").await;
    create_ctc_record_for_resource(&app, &hr_token, resource_id).await;

    let configure_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/thr/configure/{}", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "thr_eligible": true,
                "thr_calculation_basis": "full",
                "thr_employment_start_date": "2025-06-01"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let configure_res = app.clone().oneshot(configure_req).await.unwrap();
    assert_eq!(configure_res.status(), StatusCode::OK);

    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/thr/config/{}", resource_id))
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .expect("request should be built");

    let get_res = app.clone().oneshot(get_req).await.unwrap();
    assert_eq!(get_res.status(), StatusCode::OK);

    let body = to_bytes(get_res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["thr_eligible"].as_bool(), Some(true));
    assert_eq!(json["thr_calculation_basis"].as_str(), Some("full"));
    assert_eq!(json["employment_start_date"].as_str(), Some("2025-06-01"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn thr_config_rejects_invalid_basis(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "THR Invalid Basis Employee").await;
    create_ctc_record_for_resource(&app, &hr_token, resource_id).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/thr/configure/{}", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "thr_eligible": true,
                "thr_calculation_basis": "invalid_basis",
                "thr_employment_start_date": "2025-06-01"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_can_run_thr_accrual(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "THR Accrual Employee").await;
    create_ctc_record_for_resource(&app, &hr_token, resource_id).await;

    let configure_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/thr/configure/{}", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "thr_eligible": true,
                "thr_calculation_basis": "full",
                "thr_employment_start_date": "2025-01-01"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let configure_res = app.clone().oneshot(configure_req).await.unwrap();
    assert_eq!(configure_res.status(), StatusCode::OK);

    let accrual_req = Request::builder()
        .method("POST")
        .uri("/api/v1/thr/accrual/run")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "accrual_period": "2026-03"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let accrual_res = app.clone().oneshot(accrual_req).await.unwrap();
    assert_eq!(accrual_res.status(), StatusCode::OK);

    let body = to_bytes(accrual_res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["processed"].as_i64().unwrap_or_default() >= 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn thr_accrual_is_idempotent(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "THR Idempotent Employee").await;
    create_ctc_record_for_resource(&app, &hr_token, resource_id).await;

    let configure_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/thr/configure/{}", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "thr_eligible": true,
                "thr_calculation_basis": "full",
                "thr_employment_start_date": "2025-01-01"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let configure_res = app.clone().oneshot(configure_req).await.unwrap();
    assert_eq!(configure_res.status(), StatusCode::OK);

    let first_run_req = Request::builder()
        .method("POST")
        .uri("/api/v1/thr/accrual/run")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "accrual_period": "2026-03"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let first_run_res = app.clone().oneshot(first_run_req).await.unwrap();
    assert_eq!(first_run_res.status(), StatusCode::OK);

    let first_body = to_bytes(first_run_res.into_body(), usize::MAX).await.unwrap();
    let first_json: Value = serde_json::from_slice(&first_body).unwrap();
    assert!(first_json["processed"].as_i64().unwrap_or_default() >= 1);

    let second_run_req = Request::builder()
        .method("POST")
        .uri("/api/v1/thr/accrual/run")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "accrual_period": "2026-03"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let second_run_res = app.clone().oneshot(second_run_req).await.unwrap();
    assert_eq!(second_run_res.status(), StatusCode::OK);

    let second_body = to_bytes(second_run_res.into_body(), usize::MAX).await.unwrap();
    let second_json: Value = serde_json::from_slice(&second_body).unwrap();
    assert_eq!(second_json["processed"].as_i64().unwrap_or_default(), 0);
    assert!(second_json["skipped"].as_i64().unwrap_or_default() >= 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_can_get_accrual_history(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "THR History Employee").await;
    create_ctc_record_for_resource(&app, &hr_token, resource_id).await;

    let configure_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/thr/configure/{}", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "thr_eligible": true,
                "thr_calculation_basis": "full",
                "thr_employment_start_date": "2025-01-01"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let configure_res = app.clone().oneshot(configure_req).await.unwrap();
    assert_eq!(configure_res.status(), StatusCode::OK);

    let accrual_req = Request::builder()
        .method("POST")
        .uri("/api/v1/thr/accrual/run")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "accrual_period": "2026-03"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let accrual_res = app.clone().oneshot(accrual_req).await.unwrap();
    assert_eq!(accrual_res.status(), StatusCode::OK);

    let history_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/thr/accrual/{}", resource_id))
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .expect("request should be built");
    let history_res = app.clone().oneshot(history_req).await.unwrap();
    assert_eq!(history_res.status(), StatusCode::OK);

    let body = to_bytes(history_res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let accruals = json["accruals"]
        .as_array()
        .expect("accruals should be an array");
    assert!(!accruals.is_empty(), "accruals should not be empty");

    let first = &accruals[0];
    assert!(first.get("period").is_some());
    assert!(first.get("service_months").is_some());
    assert!(first.get("basis").is_some());
    assert!(first.get("accrual_amount").is_some());
    assert!(first.get("annual_entitlement").is_some());
    assert_eq!(first["service_months"].as_i64(), Some(14));
    assert_eq!(first["annual_entitlement"].as_i64(), Some(16_000_000));
    assert_eq!(first["accrual_amount"].as_i64(), Some(1_333_333));
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_can_get_thr_payout_report(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "THR Report Employee").await;
    create_ctc_record_for_resource(&app, &hr_token, resource_id).await;

    let configure_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/thr/configure/{}", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "thr_eligible": true,
                "thr_calculation_basis": "full",
                "thr_employment_start_date": "2025-01-01"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let configure_res = app.clone().oneshot(configure_req).await.unwrap();
    assert_eq!(configure_res.status(), StatusCode::OK);

    let accrual_req = Request::builder()
        .method("POST")
        .uri("/api/v1/thr/accrual/run")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "accrual_period": "2026-03"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let accrual_res = app.clone().oneshot(accrual_req).await.unwrap();
    assert_eq!(accrual_res.status(), StatusCode::OK);

    let report_req = Request::builder()
        .method("GET")
        .uri("/api/v1/thr/report?month=2026-03")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .expect("request should be built");

    let report_res = app.clone().oneshot(report_req).await.unwrap();
    assert_eq!(report_res.status(), StatusCode::OK);

    let body = to_bytes(report_res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let entries = json["entries"].as_array().expect("entries should be an array");
    assert!(!entries.is_empty(), "entries should not be empty");

    let first = &entries[0];
    assert!(first.get("resource_id").is_some());
    assert!(first.get("month").is_some());
    assert!(first.get("service_months").is_some());
    assert!(first.get("calculation_basis").is_some());
    assert!(first.get("calculation_basis_explanation").is_some());
    assert!(first.get("thr_basis_amount").is_some());
    assert!(first.get("annual_entitlement").is_some());
    assert!(first.get("accrued_to_date").is_some());
    assert!(first.get("remaining_top_up").is_some());
    assert_eq!(first["service_months"].as_i64(), Some(14));
    assert_eq!(first["calculation_basis"].as_str(), Some("full"));
    assert_eq!(first["thr_basis_amount"].as_i64(), Some(16_000_000));
    assert_eq!(first["annual_entitlement"].as_i64(), Some(16_000_000));
    assert_eq!(first["accrued_to_date"].as_i64(), Some(1_333_333));
    assert_eq!(first["remaining_top_up"].as_i64(), Some(14_666_667));
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_hr_cannot_access_thr_report(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let user_email = test_email();
    let _user_id = create_test_user_with_role(&pool, &user_email, "department_head").await;
    let user_token = get_auth_token(&app, &user_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/thr/report?month=2026-03")
        .header("Authorization", format!("Bearer {}", user_token))
        .body(Body::empty())
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn thr_accrual_stores_encrypted_values(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "THR Encryption Employee").await;
    create_ctc_record_for_resource(&app, &hr_token, resource_id).await;

    let configure_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/thr/configure/{}", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "thr_eligible": true,
                "thr_calculation_basis": "full",
                "thr_employment_start_date": "2025-01-01"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let configure_res = app.clone().oneshot(configure_req).await.unwrap();
    assert_eq!(configure_res.status(), StatusCode::OK);

    let accrual_req = Request::builder()
        .method("POST")
        .uri("/api/v1/thr/accrual/run")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "accrual_period": "2026-03"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let accrual_res = app.clone().oneshot(accrual_req).await.unwrap();
    assert_eq!(accrual_res.status(), StatusCode::OK);

    let row = sqlx::query(
        "SELECT encrypted_accrual_amount, encrypted_annual_entitlement FROM thr_accruals WHERE resource_id = $1",
    )
    .bind(resource_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let encrypted_accrual: String = row.try_get("encrypted_accrual_amount").unwrap();
    let encrypted_entitlement: String = row.try_get("encrypted_annual_entitlement").unwrap();

    assert!(
        !encrypted_accrual.parse::<i64>().is_ok(),
        "Accrual amount should be encrypted, not plaintext"
    );
    assert!(
        !encrypted_entitlement.parse::<i64>().is_ok(),
        "Entitlement should be encrypted, not plaintext"
    );
    assert!(encrypted_accrual.len() > 20, "Encrypted value should be substantial length");
}
