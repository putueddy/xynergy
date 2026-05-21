//! Integration tests for CTC Validation Reports (Story 5.3)
//!
//! Covers the new finance/admin-facing reconciliation endpoint
//! `/api/v1/ctc/validation-report`. Verifies role access, date
//! validation, payroll staging precondition, deterministic ordering,
//! exclusion accounting, BPJS regulation validation, and audit logging.
//! Also re-asserts that the existing `/api/v1/ctc/compliance-report`
//! endpoint and its `compliance_report_generated` audit action remain
//! intact (no regression from Story 5.3).

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Datelike;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("ctc-val-report-{}@example.com", Uuid::new_v4())
}

fn set_test_env() {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );
}

async fn create_user(pool: &PgPool, email: &str, role: &str) -> Uuid {
    let password_hash = xynergy_backend::routes::auth::hash_password("Password123!")
        .expect("password hashing should succeed");
    let department_id = test_department(pool).await;

    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, first_name, last_name, role, department_id)
         VALUES ($1, $2, 'Test', 'User', $3, $4)
         RETURNING id",
    )
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .bind(department_id)
    .fetch_one(pool)
    .await
    .expect("test user should be created")
}

async fn test_department(pool: &PgPool) -> Uuid {
    if let Some(id) = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM departments WHERE name = 'CTC Validation Test Department' LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .expect("department lookup should succeed")
    {
        return id;
    }

    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO departments (name) VALUES ('CTC Validation Test Department') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("test department should be created")
}

async fn create_resource(pool: &PgPool, name: &str) -> Uuid {
    let department_id = test_department(pool).await;
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO resources (name, resource_type, capacity, department_id)
         VALUES ($1, 'employee', 1.0, $2)
         RETURNING id",
    )
    .bind(name)
    .bind(department_id)
    .fetch_one(pool)
    .await
    .expect("test resource should be created")
}

async fn get_token(app: &axum::Router, email: &str) -> String {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"email": email, "password": "Password123!"}).to_string(),
        ))
        .expect("login request should build");

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

/// Create a CTC record for a resource via the API (HR endpoint).
async fn create_ctc_record(app: &axum::Router, hr_token: &str, resource_id: Uuid) {
    create_ctc_record_with_risk_tier(app, hr_token, resource_id, 1).await;
}

async fn create_ctc_record_with_risk_tier(
    app: &axum::Router,
    hr_token: &str,
    resource_id: Uuid,
    risk_tier: i32,
) {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": 15_000_000,
                "hra_allowance": 3_000_000,
                "medical_allowance": 1_000_000,
                "transport_allowance": 500_000,
                "meal_allowance": 500_000,
                "working_days_per_month": 22,
                "risk_tier": risk_tier
            })
            .to_string(),
        ))
        .expect("ctc request should build");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "CTC record creation should succeed"
    );
}

/// Insert a payroll staging row matching the values produced by `create_ctc_record`.
async fn insert_payroll_matching(pool: &PgPool, resource_id: Uuid, effective_date: &str) {
    let batch_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO payroll_validation_staging (
            import_batch_id, resource_id, effective_date,
            base_salary, hra_allowance, medical_allowance, transport_allowance, meal_allowance,
            bpjs_kesehatan_employer, bpjs_ketenagakerjaan_employer
         ) VALUES ($1, $2, $3::date,
            15000000, 3000000, 1000000, 500000, 500000,
            480000, 1058948)",
    )
    .bind(batch_id)
    .bind(resource_id)
    .bind(effective_date)
    .execute(pool)
    .await
    .expect("payroll matching insert should succeed");
}

/// Insert a payroll staging row with intentional discrepancy in base_salary.
/// BPJS values stay regulation-correct for the payroll-side basis, so this
/// produces a cross-system BPJS mismatch without a BPJS regulation error.
async fn insert_payroll_with_discrepancy(pool: &PgPool, resource_id: Uuid, effective_date: &str) {
    let batch_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO payroll_validation_staging (
            import_batch_id, resource_id, effective_date,
            base_salary, hra_allowance, medical_allowance, transport_allowance, meal_allowance,
            bpjs_kesehatan_employer, bpjs_ketenagakerjaan_employer
         ) VALUES ($1, $2, $3::date,
            14000000, 3000000, 1000000, 500000, 500000,
            480000, 1016548)",
    )
    .bind(batch_id)
    .bind(resource_id)
    .bind(effective_date)
    .execute(pool)
    .await
    .expect("payroll discrepancy insert should succeed");
}

/// Insert a payroll staging row where the Xynergy base_salary is *less than*
/// the payroll value (opposite direction from `insert_payroll_with_discrepancy`).
/// Used to assert variance is reported as an absolute value.
async fn insert_payroll_with_higher_base(pool: &PgPool, resource_id: Uuid, effective_date: &str) {
    let batch_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO payroll_validation_staging (
            import_batch_id, resource_id, effective_date,
            base_salary, hra_allowance, medical_allowance, transport_allowance, meal_allowance,
            bpjs_kesehatan_employer, bpjs_ketenagakerjaan_employer
         ) VALUES ($1, $2, $3::date,
            17000000, 3000000, 1000000, 500000, 500000,
            480000, 1143748)",
    )
    .bind(batch_id)
    .bind(resource_id)
    .bind(effective_date)
    .execute(pool)
    .await
    .expect("payroll higher-base insert should succeed");
}

async fn insert_payroll_with_bpjs_ketenagakerjaan(
    pool: &PgPool,
    resource_id: Uuid,
    effective_date: &str,
    bpjs_ketenagakerjaan_employer: i64,
) {
    let batch_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO payroll_validation_staging (
            import_batch_id, resource_id, effective_date,
            base_salary, hra_allowance, medical_allowance, transport_allowance, meal_allowance,
            bpjs_kesehatan_employer, bpjs_ketenagakerjaan_employer
         ) VALUES ($1, $2, $3::date,
            15000000, 3000000, 1000000, 500000, 500000,
            480000, $4)",
    )
    .bind(batch_id)
    .bind(resource_id)
    .bind(effective_date)
    .bind(bpjs_ketenagakerjaan_employer)
    .execute(pool)
    .await
    .expect("payroll custom BPJS insert should succeed");
}

async fn fetch_validation_report_paged(
    app: &axum::Router,
    token: &str,
    start: &str,
    end: &str,
    limit: i64,
    offset: i64,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/ctc/validation-report?start_date={}&end_date={}&limit={}&offset={}",
            start, end, limit, offset
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("paged request should build");

    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body: Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({"raw_body_unparseable": true}));
    (status, body)
}

async fn fetch_validation_report(
    app: &axum::Router,
    token: &str,
    start: &str,
    end: &str,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/ctc/validation-report?start_date={}&end_date={}",
            start, end
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should build");

    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body: Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({"raw_body_unparseable": true}));
    (status, body)
}

async fn fetch_validation_report_sampled(
    app: &axum::Router,
    token: &str,
    start: &str,
    end: &str,
    employee_ids: &str,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/ctc/validation-report?start_date={}&end_date={}&employee_ids={}",
            start, end, employee_ids
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("sampled request should build");

    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body: Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({"raw_body_unparseable": true}));
    (status, body)
}

async fn insert_payroll_matching_with_imported_at(
    pool: &PgPool,
    resource_id: Uuid,
    effective_date: &str,
    imported_at: chrono::DateTime<chrono::Utc>,
) {
    let batch_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO payroll_validation_staging (
            import_batch_id, resource_id, effective_date,
            base_salary, hra_allowance, medical_allowance, transport_allowance, meal_allowance,
            bpjs_kesehatan_employer, bpjs_ketenagakerjaan_employer, imported_at
         ) VALUES ($1, $2, $3::date,
            15000000, 3000000, 1000000, 500000, 500000,
            480000, 1058948, $4)",
    )
    .bind(batch_id)
    .bind(resource_id)
    .bind(effective_date)
    .bind(imported_at)
    .execute(pool)
    .await
    .expect("payroll matching with imported_at insert should succeed");
}

// ── Role Access ───────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn finance_can_fetch_validation_report(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let resource_id = create_resource(&pool, "Validation Employee A").await;
    create_ctc_record(&app, &hr_token, resource_id).await;

    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, resource_id, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["total_compared"].is_number());
    assert!(body["match_rate_pct"].is_number());
    assert!(body["mismatches"].is_array());
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_fetch_validation_report(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let resource_id = create_resource(&pool, "Validation Employee Admin").await;
    create_ctc_record(&app, &hr_token, resource_id).await;
    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, resource_id, &today).await;

    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin").await;
    let admin_token = get_token(&app, &admin_email).await;

    let (status, _body) =
        fetch_validation_report(&app, &admin_token, "2020-01-01", "2030-12-31").await;

    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_denied_validation_report(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let (status, body) = fetch_validation_report(&app, &hr_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "FORBIDDEN_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn department_head_denied_validation_report(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _ = create_user(&pool, &email, "department_head").await;
    let token = get_token(&app, &email).await;

    let (status, _) = fetch_validation_report(&app, &token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn project_manager_denied_validation_report(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _ = create_user(&pool, &email, "project_manager").await;
    let token = get_token(&app, &email).await;

    let (status, _) = fetch_validation_report(&app, &token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ── Validation ────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn inverted_date_range_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _ = create_user(&pool, &email, "finance").await;
    let token = get_token(&app, &email).await;

    let (status, body) = fetch_validation_report(&app, &token, "2030-01-01", "2020-12-31").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn missing_payroll_staging_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _ = create_user(&pool, &email, "finance").await;
    let token = get_token(&app, &email).await;

    let (status, body) = fetch_validation_report(&app, &token, "2020-01-01", "2020-12-31").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("payroll staging"),
        "Expected payroll staging error message, got: {}",
        msg
    );
}

// ── Summary + Deterministic Ordering ──────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn report_returns_summary_counts_and_deterministic_rows(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    // Create two employees with CTC records, names chosen to test sort order.
    let bob_id = create_resource(&pool, "Bob Employee").await;
    let alice_id = create_resource(&pool, "Alice Employee").await;
    create_ctc_record(&app, &hr_token, bob_id).await;
    create_ctc_record(&app, &hr_token, alice_id).await;

    let today = chrono::Local::now().date_naive().to_string();
    // Bob matches, Alice has base_salary and derived BPJS discrepancies.
    insert_payroll_matching(&pool, bob_id, &today).await;
    insert_payroll_with_discrepancy(&pool, alice_id, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["total_compared"].as_i64().unwrap(), 2);
    assert_eq!(body["total_matches"].as_i64().unwrap(), 1);
    assert_eq!(body["total_discrepancies"].as_i64().unwrap(), 2);

    let mismatches = body["mismatches"].as_array().unwrap();
    assert!(
        !mismatches.is_empty(),
        "Expected at least one mismatch for Alice"
    );

    // Deterministic ordering: Alice should appear first (employee_name ASC).
    let first_name = mismatches[0]["employee_name"].as_str().unwrap();
    assert_eq!(first_name, "Alice Employee");

    // Run twice and confirm identical mismatch ordering.
    let (status2, body2) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(body["mismatches"], body2["mismatches"]);
}

// ── Excluded Records ──────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn excluded_records_are_counted_when_missing_payroll(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let with_payroll = create_resource(&pool, "Has Payroll").await;
    let without_payroll = create_resource(&pool, "Missing Payroll").await;
    create_ctc_record(&app, &hr_token, with_payroll).await;
    create_ctc_record(&app, &hr_token, without_payroll).await;

    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, with_payroll, &today).await;
    // No payroll row for `without_payroll` - should land in excluded list.

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["total_compared"].as_i64().unwrap(), 1);
    assert_eq!(body["excluded_count"].as_i64().unwrap(), 1);

    let excluded = body["excluded"].as_array().unwrap();
    assert_eq!(excluded.len(), 1);
    assert_eq!(
        excluded[0]["employee_name"].as_str().unwrap(),
        "Missing Payroll"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn payroll_only_records_are_reported_as_excluded(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let payroll_only = create_resource(&pool, "Payroll Only").await;
    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, payroll_only, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["total_compared"].as_i64().unwrap(), 0);
    assert_eq!(body["excluded_count"].as_i64().unwrap(), 1);
    let excluded = body["excluded"].as_array().unwrap();
    assert_eq!(
        excluded[0]["employee_name"].as_str().unwrap(),
        "Payroll Only"
    );
    assert_eq!(
        excluded[0]["reason"].as_str().unwrap(),
        "No active Xynergy CTC record in range"
    );
}

// ── BPJS Regulation Validation ────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn bpjs_validation_uses_current_regulation_formula(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let resource_id = create_resource(&pool, "BPJS Test Employee").await;
    create_ctc_record(&app, &hr_token, resource_id).await;

    let today = chrono::Local::now().date_naive().to_string();
    // Insert payroll row with intentionally wrong BPJS values.
    let batch_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO payroll_validation_staging (
            import_batch_id, resource_id, effective_date,
            base_salary, hra_allowance, medical_allowance, transport_allowance, meal_allowance,
            bpjs_kesehatan_employer, bpjs_ketenagakerjaan_employer
         ) VALUES ($1, $2, $3::date,
            15000000, 3000000, 1000000, 500000, 500000,
            100000, 200000)",
    )
    .bind(batch_id)
    .bind(resource_id)
    .bind(&today)
    .execute(&pool)
    .await
    .expect("payroll insert should succeed");

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);

    // BPJS error count should be > 0 since payroll values diverge from regulation-recalculated ones.
    assert!(
        body["bpjs_error_count"].as_i64().unwrap() >= 1,
        "Expected at least 1 BPJS regulation error, body: {}",
        body
    );

    let mismatches = body["mismatches"].as_array().unwrap();
    assert_eq!(
        body["total_discrepancies"].as_i64().unwrap(),
        mismatches.len() as i64,
        "total_discrepancies should count field-level mismatch rows"
    );
    let bpjs_mismatch = mismatches
        .iter()
        .find(|m| m["field_name"].as_str().unwrap() == "bpjs_kesehatan_employer");
    assert!(
        bpjs_mismatch.is_some(),
        "Should report bpjs_kesehatan_employer mismatch"
    );

    // BPJS metadata should be attached for BPJS field mismatches.
    let bpjs_meta = &bpjs_mismatch.unwrap()["bpjs_metadata"];
    assert!(bpjs_meta.is_object());
    assert!(bpjs_meta["risk_tier"].is_number());
    assert!(bpjs_meta["recalculated_value"].is_number());
}

#[sqlx::test(migrations = "../../migrations")]
async fn bpjs_validation_respects_created_risk_tier(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let resource_id = create_resource(&pool, "BPJS Tier Four Employee").await;
    create_ctc_record_with_risk_tier(&app, &hr_token, resource_id, 4).await;

    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_with_bpjs_ketenagakerjaan(&pool, resource_id, &today, 1_358_948).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_compared"].as_i64().unwrap(), 1);
    assert_eq!(body["total_matches"].as_i64().unwrap(), 1);
    assert_eq!(body["bpjs_error_count"].as_i64().unwrap(), 0);

    let mismatches = body["mismatches"].as_array().unwrap();
    assert!(
        !mismatches
            .iter()
            .any(|m| m["field_name"].as_str() == Some("bpjs_ketenagakerjaan_employer")),
        "tier 4 BPJS values should not be recalculated with tier 1 defaults: {}",
        body
    );
}

// ── Audit Logging ─────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn report_generation_creates_audit_log(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let resource_id = create_resource(&pool, "Audit Test Employee").await;
    create_ctc_record(&app, &hr_token, resource_id).await;
    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, resource_id, &today).await;

    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, _) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs
         WHERE action = 'ctc_validation_report_generated'
         AND entity_type = 'ctc_validation_report'
         AND user_id = $1",
    )
    .bind(finance_id)
    .fetch_one(&pool)
    .await
    .expect("audit log query should succeed");

    assert_eq!(count, 1, "Should create exactly one audit log entry");
}

// ── Regression: existing compliance-report behavior intact ────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn existing_compliance_report_endpoint_unchanged(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let hr_id = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    // Verify the existing endpoint still serves HR and emits its original audit action.
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/compliance-report?start_date=2025-01-01&end_date=2025-12-31")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .expect("request should build");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs
         WHERE action = 'compliance_report_generated'
         AND entity_type = 'compliance_report'
         AND user_id = $1",
    )
    .bind(hr_id)
    .fetch_one(&pool)
    .await
    .expect("audit log query should succeed");

    assert_eq!(
        count, 1,
        "Original compliance_report_generated audit action should still fire"
    );
}

// ── Pagination (G1, G2) ───────────────────────────────────────────────────

/// [P0] Pagination must return only the requested slice and never exceed `limit`.
/// The summary metrics must still reflect the full mismatch population.
#[sqlx::test(migrations = "../../migrations")]
async fn pagination_limit_offset_returns_subset(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    // Three employees, all with base_salary and derived BPJS discrepancies.
    let alice = create_resource(&pool, "Alice Pagination").await;
    let bob = create_resource(&pool, "Bob Pagination").await;
    let carol = create_resource(&pool, "Carol Pagination").await;
    create_ctc_record(&app, &hr_token, alice).await;
    create_ctc_record(&app, &hr_token, bob).await;
    create_ctc_record(&app, &hr_token, carol).await;

    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_with_discrepancy(&pool, alice, &today).await;
    insert_payroll_with_discrepancy(&pool, bob, &today).await;
    insert_payroll_with_discrepancy(&pool, carol, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report_paged(&app, &finance_token, "2020-01-01", "2030-12-31", 1, 2).await;
    assert_eq!(status, StatusCode::OK);

    // Summary reflects ALL six field-level discrepancies, not just the page.
    assert_eq!(body["total_compared"].as_i64().unwrap(), 3);
    assert_eq!(body["total_discrepancies"].as_i64().unwrap(), 6);

    // But the mismatch detail page returns only one row (the limit).
    let mismatches = body["mismatches"].as_array().unwrap();
    assert_eq!(mismatches.len(), 1, "limit=1 must yield one mismatch row");

    // offset=2 with deterministic alphabetical ordering → "Bob Pagination" first.
    assert_eq!(
        mismatches[0]["employee_name"].as_str().unwrap(),
        "Bob Pagination"
    );
}

/// [P1] Summary metrics must be identical for paginated vs unpaginated requests
/// over the same range — pagination only narrows the mismatch detail region.
#[sqlx::test(migrations = "../../migrations")]
async fn summary_metrics_stable_under_pagination(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let a = create_resource(&pool, "Stable Summary A").await;
    let b = create_resource(&pool, "Stable Summary B").await;
    create_ctc_record(&app, &hr_token, a).await;
    create_ctc_record(&app, &hr_token, b).await;

    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, a, &today).await;
    insert_payroll_with_discrepancy(&pool, b, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (s1, full) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    let (s2, paged) =
        fetch_validation_report_paged(&app, &finance_token, "2020-01-01", "2030-12-31", 1, 0).await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::OK);

    for field in [
        "total_compared",
        "total_matches",
        "total_discrepancies",
        "excluded_count",
        "bpjs_error_count",
        "match_rate_pct",
    ] {
        assert_eq!(
            full[field], paged[field],
            "summary field `{}` must be stable under pagination",
            field
        );
    }
}

// ── Mismatch Row Contract (G3, G4) ────────────────────────────────────────

/// [P0] AC #3: drilling into a mismatch must expose the canonical fields.
#[sqlx::test(migrations = "../../migrations")]
async fn mismatch_row_contract_includes_canonical_fields(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let res_id = create_resource(&pool, "Contract Employee").await;
    create_ctc_record(&app, &hr_token, res_id).await;
    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_with_discrepancy(&pool, res_id, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);

    let mismatches = body["mismatches"].as_array().unwrap();
    let base_salary_row = mismatches
        .iter()
        .find(|m| m["field_name"].as_str() == Some("base_salary"))
        .expect("expected a base_salary mismatch row");

    // employee_id must be a parseable UUID string.
    let emp_id_str = base_salary_row["employee_id"].as_str().unwrap();
    assert!(
        Uuid::parse_str(emp_id_str).is_ok(),
        "employee_id should be a UUID string"
    );
    assert_eq!(emp_id_str, res_id.to_string());

    assert_eq!(
        base_salary_row["employee_name"].as_str().unwrap(),
        "Contract Employee"
    );
    assert_eq!(
        base_salary_row["xynergy_value"].as_i64().unwrap(),
        15_000_000
    );
    assert_eq!(
        base_salary_row["payroll_value"].as_i64().unwrap(),
        14_000_000
    );
    assert_eq!(
        base_salary_row["variance_amount"].as_i64().unwrap(),
        1_000_000
    );
    assert_eq!(
        base_salary_row["status"].as_str().unwrap(),
        "DISCREPANCY",
        "plain field mismatch should be DISCREPANCY, not BPJS_REGULATION_ERROR"
    );
    // bpjs_metadata is omitted (serde `Option<T>` → `null`) for plain-field rows.
    assert!(
        base_salary_row["bpjs_metadata"].is_null(),
        "bpjs_metadata must be null for non-BPJS fields"
    );
}

/// [P1] `variance_amount` must always be the absolute difference, regardless of
/// which side is larger.
#[sqlx::test(migrations = "../../migrations")]
async fn variance_amount_is_absolute(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let lower = create_resource(&pool, "Lower Payroll").await;
    let higher = create_resource(&pool, "Higher Payroll").await;
    create_ctc_record(&app, &hr_token, lower).await;
    create_ctc_record(&app, &hr_token, higher).await;

    let today = chrono::Local::now().date_naive().to_string();
    // Lower Payroll: payroll < xynergy (14M vs 15M).
    insert_payroll_with_discrepancy(&pool, lower, &today).await;
    // Higher Payroll: payroll > xynergy (17M vs 15M).
    insert_payroll_with_higher_base(&pool, higher, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);

    let mismatches = body["mismatches"].as_array().unwrap();
    for row in mismatches {
        if row["field_name"].as_str() == Some("base_salary") {
            let variance = row["variance_amount"].as_i64().unwrap();
            assert!(
                variance > 0,
                "variance_amount must be positive (absolute) for row {:?}",
                row
            );
        }
    }

    let lower_var = mismatches
        .iter()
        .find(|m| m["employee_name"].as_str() == Some("Lower Payroll"))
        .and_then(|m| m["variance_amount"].as_i64())
        .unwrap();
    let higher_var = mismatches
        .iter()
        .find(|m| m["employee_name"].as_str() == Some("Higher Payroll"))
        .and_then(|m| m["variance_amount"].as_i64())
        .unwrap();

    assert_eq!(lower_var, 1_000_000);
    assert_eq!(higher_var, 2_000_000);
}

// ── BPJS Regulation Error Status (G5) ─────────────────────────────────────

/// [P0] AC #4: BPJS regulation calculation errors must be flagged with a
/// distinct status, not lumped into generic DISCREPANCY.
#[sqlx::test(migrations = "../../migrations")]
async fn bpjs_regulation_error_status_distinct_from_discrepancy(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let res_id = create_resource(&pool, "BPJS Regulation Employee").await;
    create_ctc_record(&app, &hr_token, res_id).await;

    let today = chrono::Local::now().date_naive().to_string();
    // BPJS values diverging from both Xynergy stored value AND the regulation formula.
    let batch_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO payroll_validation_staging (
            import_batch_id, resource_id, effective_date,
            base_salary, hra_allowance, medical_allowance, transport_allowance, meal_allowance,
            bpjs_kesehatan_employer, bpjs_ketenagakerjaan_employer
         ) VALUES ($1, $2, $3::date,
            15000000, 3000000, 1000000, 500000, 500000,
            100000, 200000)",
    )
    .bind(batch_id)
    .bind(res_id)
    .bind(&today)
    .execute(&pool)
    .await
    .expect("payroll insert should succeed");

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);

    let mismatches = body["mismatches"].as_array().unwrap();
    let bpjs_regulation_rows: Vec<&Value> = mismatches
        .iter()
        .filter(|m| m["status"].as_str() == Some("BPJS_REGULATION_ERROR"))
        .collect();

    assert!(
        !bpjs_regulation_rows.is_empty(),
        "Expected at least one BPJS_REGULATION_ERROR row when payroll BPJS diverges from regulation"
    );
    for row in bpjs_regulation_rows {
        assert!(
            row["bpjs_metadata"].is_object(),
            "BPJS_REGULATION_ERROR rows must carry bpjs_metadata"
        );
        assert!(row["bpjs_metadata"]["risk_tier"].is_number());
        assert!(row["bpjs_metadata"]["recalculated_value"].is_number());
    }
}

// ── Audit Payload Shape (G6) ──────────────────────────────────────────────

/// [P0] Story rule: audit payload stays lean (date range + counts only). No
/// mismatch rows, no decrypted values, no employee identifiers in the changes blob.
#[sqlx::test(migrations = "../../migrations")]
async fn audit_log_payload_contains_only_lean_metadata(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let res_id = create_resource(&pool, "Audit Payload Employee").await;
    create_ctc_record(&app, &hr_token, res_id).await;
    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_with_discrepancy(&pool, res_id, &today).await;
    let start = "2020-01-01";
    let end = "2030-12-31";

    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, _) = fetch_validation_report(&app, &finance_token, start, end).await;
    assert_eq!(status, StatusCode::OK);

    let payload: Value = sqlx::query_scalar(
        "SELECT changes FROM audit_logs
         WHERE action = 'ctc_validation_report_generated'
           AND user_id = $1
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(finance_id)
    .fetch_one(&pool)
    .await
    .expect("audit log row should exist");

    // Allowed keys per story Task 3 (extended in Decision A/B follow-ups for
    // sampling state and payroll coverage — both are summary metadata, not
    // sensitive comparison values).
    let allowed: std::collections::HashSet<&str> = [
        "start_date",
        "end_date",
        "total_compared",
        "total_matches",
        "total_discrepancies",
        "excluded_count",
        "bpjs_error_count",
        "match_rate_pct",
        "sampled",
        "sampled_count",
        "payroll_coverage_pct",
    ]
    .into_iter()
    .collect();

    let obj = payload
        .as_object()
        .expect("audit changes must be a JSON object");

    for key in obj.keys() {
        assert!(
            allowed.contains(key.as_str()),
            "audit payload contains disallowed key `{}` (would leak detail beyond story Task 3 contract)",
            key
        );
    }

    // Required metadata fields are present.
    assert_eq!(obj.get("start_date").and_then(|v| v.as_str()), Some(start));
    assert_eq!(obj.get("end_date").and_then(|v| v.as_str()), Some(end));
    assert!(obj.contains_key("total_compared"));
    assert!(obj.contains_key("match_rate_pct"));

    // Mismatch rows must NOT leak into audit.
    assert!(
        !obj.contains_key("mismatches") && !obj.contains_key("excluded"),
        "audit payload must not include mismatches or excluded rows"
    );
}

// ── Missing Auth (G7) ─────────────────────────────────────────────────────

/// [P1] No Authorization header → 401, not 403 / 500.
#[sqlx::test(migrations = "../../migrations")]
async fn missing_auth_token_returns_401(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/validation-report?start_date=2020-01-01&end_date=2030-12-31")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// ── Date-Range Semantics (G8, G9) ─────────────────────────────────────────

/// [P1] Records on exactly `start_date` and `end_date` must be included
/// (BETWEEN is inclusive — regression guard).
#[sqlx::test(migrations = "../../migrations")]
async fn date_boundary_records_are_included(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let start_emp = create_resource(&pool, "Start Boundary").await;
    let end_emp = create_resource(&pool, "End Boundary").await;
    create_ctc_record(&app, &hr_token, start_emp).await;
    create_ctc_record(&app, &hr_token, end_emp).await;

    let current_year = chrono::Local::now().date_naive().year();
    let start = chrono::NaiveDate::from_ymd_opt(current_year, 1, 1)
        .unwrap()
        .to_string();
    let end = chrono::NaiveDate::from_ymd_opt(current_year, 12, 31)
        .unwrap()
        .to_string();

    insert_payroll_matching(&pool, start_emp, &start).await;
    insert_payroll_matching(&pool, end_emp, &end).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) = fetch_validation_report(&app, &finance_token, &start, &end).await;
    assert_eq!(status, StatusCode::OK);

    assert!(
        body["total_compared"].as_i64().unwrap() + body["excluded_count"].as_i64().unwrap() >= 2,
        "both boundary employees should be visible to the report, body={}",
        body
    );
}

/// [P1] When a resource has multiple payroll rows within the range, the most
/// recent `effective_date` is used as the comparison source.
#[sqlx::test(migrations = "../../migrations")]
async fn latest_payroll_row_per_resource_wins(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let res_id = create_resource(&pool, "Multi-Payroll Employee").await;
    create_ctc_record(&app, &hr_token, res_id).await;

    let today = chrono::Local::now().date_naive();
    let older = (today - chrono::Duration::days(60)).to_string();
    let newer = today.to_string();

    // Older row diverges, newer row matches → if "latest wins", report shows 0 discrepancies.
    insert_payroll_with_discrepancy(&pool, res_id, &older).await;
    insert_payroll_matching(&pool, res_id, &newer).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["total_compared"].as_i64().unwrap(), 1);
    assert_eq!(
        body["total_matches"].as_i64().unwrap(),
        1,
        "newer payroll row should be the comparison source (matching), body={}",
        body
    );
    assert_eq!(body["total_discrepancies"].as_i64().unwrap(), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn latest_import_wins_when_payroll_effective_date_ties(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let res_id = create_resource(&pool, "Same-Day Payroll Employee").await;
    create_ctc_record(&app, &hr_token, res_id).await;

    let today = chrono::Local::now().date_naive().to_string();
    let old_batch = Uuid::new_v4();
    let new_batch = Uuid::new_v4();
    // Use recent imported_at values so the same-day tie-break scenario
    // remains within the payroll freshness window (7 days).
    let now = chrono::Utc::now();
    let older_import = now - chrono::Duration::hours(2);
    let newer_import = now - chrono::Duration::hours(1);

    sqlx::query(
        "INSERT INTO payroll_validation_staging (
            import_batch_id, resource_id, effective_date,
            base_salary, hra_allowance, medical_allowance, transport_allowance, meal_allowance,
            bpjs_kesehatan_employer, bpjs_ketenagakerjaan_employer, imported_at
         ) VALUES ($1, $2, $3::date,
            14000000, 3000000, 1000000, 500000, 500000,
            480000, 1058948, $4)",
    )
    .bind(old_batch)
    .bind(res_id)
    .bind(&today)
    .bind(older_import)
    .execute(&pool)
    .await
    .expect("old payroll row should insert");

    sqlx::query(
        "INSERT INTO payroll_validation_staging (
            import_batch_id, resource_id, effective_date,
            base_salary, hra_allowance, medical_allowance, transport_allowance, meal_allowance,
            bpjs_kesehatan_employer, bpjs_ketenagakerjaan_employer, imported_at
         ) VALUES ($1, $2, $3::date,
            15000000, 3000000, 1000000, 500000, 500000,
            480000, 1058948, $4)",
    )
    .bind(new_batch)
    .bind(res_id)
    .bind(&today)
    .bind(newer_import)
    .execute(&pool)
    .await
    .expect("new payroll row should insert");

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_compared"].as_i64().unwrap(), 1);
    assert_eq!(
        body["total_matches"].as_i64().unwrap(),
        1,
        "same-date payroll rows should use the latest imported_at row, body={}",
        body
    );
}

// ── Match Rate Numeric Correctness (G10) ──────────────────────────────────

/// [P1] match_rate_pct must be the actual computed percentage, not a stub.
/// 3 matches out of 4 compared records → 75.0%.
#[sqlx::test(migrations = "../../migrations")]
async fn match_rate_percentage_is_accurate(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let today = chrono::Local::now().date_naive().to_string();
    let matches = ["MR Match One", "MR Match Two", "MR Match Three"];
    for name in matches {
        let id = create_resource(&pool, name).await;
        create_ctc_record(&app, &hr_token, id).await;
        insert_payroll_matching(&pool, id, &today).await;
    }
    let mismatch_id = create_resource(&pool, "MR Mismatch").await;
    create_ctc_record(&app, &hr_token, mismatch_id).await;
    insert_payroll_with_discrepancy(&pool, mismatch_id, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["total_compared"].as_i64().unwrap(), 4);
    assert_eq!(body["total_matches"].as_i64().unwrap(), 3);
    assert_eq!(body["total_discrepancies"].as_i64().unwrap(), 2);

    let rate = body["match_rate_pct"].as_f64().unwrap();
    assert!(
        (rate - 75.0).abs() < 0.0001,
        "expected match_rate_pct = 75.0, got {}",
        rate
    );
}

// ── Full-Summary Idempotency (G11) ────────────────────────────────────────

/// [P2] Two consecutive runs over the same range return byte-identical summary
/// and mismatches. Builds on the existing deterministic-ordering test by also
/// asserting on the summary metrics themselves.
#[sqlx::test(migrations = "../../migrations")]
async fn full_report_is_idempotent_across_runs(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let a = create_resource(&pool, "Idem Alpha").await;
    let b = create_resource(&pool, "Idem Beta").await;
    create_ctc_record(&app, &hr_token, a).await;
    create_ctc_record(&app, &hr_token, b).await;

    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, a, &today).await;
    insert_payroll_with_discrepancy(&pool, b, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (s1, run1) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    let (s2, run2) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::OK);

    for field in [
        "total_compared",
        "total_matches",
        "total_discrepancies",
        "excluded_count",
        "bpjs_error_count",
        "match_rate_pct",
        "mismatches",
        "excluded",
    ] {
        assert_eq!(
            run1[field], run2[field],
            "field `{}` must be identical across idempotent runs",
            field
        );
    }
}

// ── Sampling (Decision A) ─────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn sampled_run_returns_only_picked_employees(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let alice = create_resource(&pool, "Sample Alice").await;
    let bob = create_resource(&pool, "Sample Bob").await;
    let carol = create_resource(&pool, "Sample Carol").await;
    create_ctc_record(&app, &hr_token, alice).await;
    create_ctc_record(&app, &hr_token, bob).await;
    create_ctc_record(&app, &hr_token, carol).await;

    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, alice, &today).await;
    insert_payroll_with_discrepancy(&pool, bob, &today).await;
    insert_payroll_matching(&pool, carol, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let sample = format!("{},{}", alice, bob);
    let (status, body) =
        fetch_validation_report_sampled(&app, &finance_token, "2020-01-01", "2030-12-31", &sample)
            .await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["sampled"].as_bool().unwrap(), true);
    assert_eq!(
        body["total_compared"].as_i64().unwrap(),
        2,
        "sampled run should compare only the picked employees, body={}",
        body
    );

    let names: Vec<&str> = body["mismatches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["employee_name"].as_str().unwrap())
        .collect();
    assert!(
        names.iter().all(|n| *n == "Sample Bob"),
        "only the sampled mismatched employee should appear in mismatches: {:?}",
        names
    );
    assert!(
        !names.contains(&"Sample Carol"),
        "non-sampled employees must never leak into a sampled report"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn sampled_run_with_invalid_id_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) = fetch_validation_report_sampled(
        &app,
        &finance_token,
        "2020-01-01",
        "2030-12-31",
        "not-a-uuid",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn audit_log_records_sampled_metadata(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let res_id = create_resource(&pool, "Audit Sampled Employee").await;
    create_ctc_record(&app, &hr_token, res_id).await;
    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, res_id, &today).await;

    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, _) = fetch_validation_report_sampled(
        &app,
        &finance_token,
        "2020-01-01",
        "2030-12-31",
        &res_id.to_string(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let payload: Value = sqlx::query_scalar(
        "SELECT changes FROM audit_logs
         WHERE action = 'ctc_validation_report_generated'
           AND user_id = $1
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(finance_id)
    .fetch_one(&pool)
    .await
    .expect("audit log row should exist");

    assert_eq!(payload["sampled"].as_bool(), Some(true));
    assert_eq!(payload["sampled_count"].as_i64(), Some(1));
    assert!(
        payload["bpjs_error_count"].is_number(),
        "audit payload should expose bpjs_error_count"
    );
    assert!(
        payload["payroll_coverage_pct"].is_number(),
        "audit payload should expose payroll_coverage_pct"
    );

    // Unsampled run should record sampled=false / sampled_count=0.
    let (status2, _) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status2, StatusCode::OK);

    let payload2: Value = sqlx::query_scalar(
        "SELECT changes FROM audit_logs
         WHERE action = 'ctc_validation_report_generated'
           AND user_id = $1
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(finance_id)
    .fetch_one(&pool)
    .await
    .expect("audit log row should exist");

    assert_eq!(payload2["sampled"].as_bool(), Some(false));
    assert_eq!(payload2["sampled_count"].as_i64(), Some(0));
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_sampled_ids_are_deduped_in_audit(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let res_id = create_resource(&pool, "Duplicate Sample Employee").await;
    create_ctc_record(&app, &hr_token, res_id).await;
    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, res_id, &today).await;

    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let duplicate_sample = format!("{},{},{}", res_id, res_id, res_id);
    let (status, _) = fetch_validation_report_sampled(
        &app,
        &finance_token,
        "2020-01-01",
        "2030-12-31",
        &duplicate_sample,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let payload: Value = sqlx::query_scalar(
        "SELECT changes FROM audit_logs
         WHERE action = 'ctc_validation_report_generated'
           AND user_id = $1
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(finance_id)
    .fetch_one(&pool)
    .await
    .expect("audit log row should exist");

    assert_eq!(payload["sampled"].as_bool(), Some(true));
    assert_eq!(
        payload["sampled_count"].as_i64(),
        Some(1),
        "sampled_count should count distinct employee ids only"
    );
}

// ── Payroll Freshness (Decision B) ────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn stale_payroll_data_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let res_id = create_resource(&pool, "Stale Payroll Employee").await;
    create_ctc_record(&app, &hr_token, res_id).await;

    let today = chrono::Local::now().date_naive();
    let stale_imported_at = chrono::Utc::now() - chrono::Duration::days(30);
    insert_payroll_matching_with_imported_at(&pool, res_id, &today.to_string(), stale_imported_at)
        .await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", &today.to_string()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.to_lowercase().contains("stale"),
        "expected freshness validation error, got: {}",
        msg
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn one_recent_payroll_row_does_not_hide_stale_selected_baseline(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let stale_res = create_resource(&pool, "Stale Selected Payroll").await;
    let fresh_res = create_resource(&pool, "Fresh Selected Payroll").await;
    create_ctc_record(&app, &hr_token, stale_res).await;
    create_ctc_record(&app, &hr_token, fresh_res).await;

    let today = chrono::Local::now().date_naive();
    insert_payroll_matching_with_imported_at(
        &pool,
        stale_res,
        &today.to_string(),
        chrono::Utc::now() - chrono::Duration::days(30),
    )
    .await;
    insert_payroll_matching_with_imported_at(
        &pool,
        fresh_res,
        &today.to_string(),
        chrono::Utc::now(),
    )
    .await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", &today.to_string()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.to_lowercase().contains("stale"),
        "expected stale selected baseline error, got: {}",
        msg
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn sampled_run_requires_payroll_for_sampled_ids(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let unrelated = create_resource(&pool, "Unrelated Payroll Employee").await;
    create_ctc_record(&app, &hr_token, unrelated).await;
    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, unrelated, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let missing_sample_id = Uuid::new_v4();
    let (status, body) = fetch_validation_report_sampled(
        &app,
        &finance_token,
        "2020-01-01",
        "2030-12-31",
        &missing_sample_id.to_string(),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("No payroll staging data"),
        "expected sample-scoped payroll validation error, got: {}",
        msg
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn sampled_run_rejects_mixed_sample_with_missing_payroll(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let with_payroll = create_resource(&pool, "Sample Has Payroll").await;
    let missing_payroll = create_resource(&pool, "Sample Missing Payroll").await;
    create_ctc_record(&app, &hr_token, with_payroll).await;
    create_ctc_record(&app, &hr_token, missing_payroll).await;

    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, with_payroll, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let sample = format!("{},{}", with_payroll, missing_payroll);
    let (status, body) =
        fetch_validation_report_sampled(&app, &finance_token, "2020-01-01", "2030-12-31", &sample)
            .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("without payroll staging data"),
        "expected mixed sample payroll validation error, got: {}",
        msg
    );
}

// ── Payroll Coverage (Decision B) ─────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn payroll_coverage_pct_surfaces_missing_employees(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr").await;
    let hr_token = get_token(&app, &hr_email).await;

    let with_payroll = create_resource(&pool, "Coverage Present").await;
    let without_payroll = create_resource(&pool, "Coverage Missing").await;
    let payroll_only = create_resource(&pool, "Coverage Payroll Only").await;
    create_ctc_record(&app, &hr_token, with_payroll).await;
    create_ctc_record(&app, &hr_token, without_payroll).await;

    let today = chrono::Local::now().date_naive().to_string();
    insert_payroll_matching(&pool, with_payroll, &today).await;
    insert_payroll_matching(&pool, payroll_only, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);

    let coverage = body["payroll_coverage_pct"]
        .as_f64()
        .expect("payroll_coverage_pct must be present in the report response");
    assert!(
        (coverage - 50.0).abs() < 0.01,
        "1 of 2 CTC employees has payroll → coverage = 50.00, got {}",
        coverage
    );

    let excluded_names: Vec<&str> = body["excluded"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["employee_name"].as_str().unwrap())
        .collect();
    assert!(
        excluded_names.contains(&"Coverage Missing"),
        "missing-from-payroll employee should still surface in excluded list"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn legacy_ctc_rows_missing_encryption_metadata_are_excluded(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let resource_id = create_resource(&pool, "Legacy Missing Encryption").await;
    let today = chrono::Local::now().date_naive().to_string();

    sqlx::query(
        "INSERT INTO ctc_records (resource_id, components, reason, effective_date, status)
         VALUES ($1, '{}'::jsonb, 'Legacy plaintext CTC row', $2::date, 'Active')",
    )
    .bind(resource_id)
    .bind(&today)
    .execute(&pool)
    .await
    .expect("legacy CTC row should insert");

    insert_payroll_matching(&pool, resource_id, &today).await;

    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance").await;
    let finance_token = get_token(&app, &finance_email).await;

    let (status, body) =
        fetch_validation_report(&app, &finance_token, "2020-01-01", "2030-12-31").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_compared"].as_i64().unwrap(), 0);
    assert_eq!(body["excluded_count"].as_i64().unwrap(), 1);
    assert_eq!(
        body["excluded"][0]["reason"].as_str().unwrap(),
        "CTC encryption metadata missing"
    );
}
