//! Integration tests for CTC validation, completeness, and compliance (Story 2.4)

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::{Datelike, NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("ctc-val-test-{}@example.com", Uuid::new_v4())
}

fn set_ctc_crypto_env() {
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );
}

fn month_start_months_ago(months_ago: i32) -> NaiveDate {
    let today = Utc::now().date_naive();
    let total_months = today.year() * 12 + today.month0() as i32 - months_ago;
    let year = total_months.div_euclid(12);
    let month0 = total_months.rem_euclid(12);
    NaiveDate::from_ymd_opt(year, (month0 + 1) as u32, 1).expect("valid month start")
}

fn month_key(date: NaiveDate) -> String {
    format!("{:04}-{:02}", date.year(), date.month())
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

async fn create_test_department(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("INSERT INTO departments (name) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("test department should be created")
}

async fn create_test_resource(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO resources (name, resource_type, capacity)
         VALUES ($1, 'employee', 1.0)
         RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("test resource should be created")
}

async fn create_test_resource_in_department(pool: &PgPool, name: &str, dept_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO resources (name, resource_type, capacity, department_id)
         VALUES ($1, 'employee', 1.0, $2)
         RETURNING id",
    )
    .bind(name)
    .bind(dept_id)
    .fetch_one(pool)
    .await
    .expect("test resource should be created in department")
}

async fn create_test_project(pool: &PgPool, pm_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, project_manager_id, status, start_date, end_date)
         VALUES ($1, $2, 'active', '2025-01-01', '2025-12-31')
         RETURNING id",
    )
    .bind(format!("Test Project {}", Uuid::new_v4()))
    .bind(pm_id)
    .fetch_one(pool)
    .await
    .expect("test project should be created")
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

/// Helper: create a CTC record for a resource via the API
async fn create_ctc_for_resource(app: &axum::Router, token: &str, resource_id: Uuid) {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
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
        .expect("CTC creation should return response");
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "CTC record should be created successfully"
    );
}

// ============================================================================
// Task 8: CTC Validation Integration Tests
// ============================================================================

/// POST /ctc with invalid data (zero base salary) → 400 with validation error
#[sqlx::test(migrations = "../../migrations")]
async fn create_ctc_with_zero_base_salary_rejected(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Zero Salary Employee").await;

    // Zero base salary should trigger validation error
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": 0,
                "hra_allowance": 0,
                "medical_allowance": 0,
                "transport_allowance": 0,
                "meal_allowance": 0,
                "working_days_per_month": 22,
                "risk_tier": 1
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Zero base salary should be rejected by validation"
    );
}

/// POST /ctc with excessive allowances (> 200% of base salary) → 400 with validation error
#[sqlx::test(migrations = "../../migrations")]
async fn create_ctc_with_excessive_allowances_rejected(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Excessive Allowance Employee").await;

    // Total allowances = 35M > 200% of 10M = 20M → should be rejected
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": 10000000,
                "hra_allowance": 10000000,
                "medical_allowance": 10000000,
                "transport_allowance": 10000000,
                "meal_allowance": 5000000,
                "working_days_per_month": 22,
                "risk_tier": 1
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Excessive allowances (>200% of base salary) should be rejected"
    );
}

/// POST /ctc with valid data → 200 success (no regression from adding validation)
#[sqlx::test(migrations = "../../migrations")]
async fn create_ctc_with_valid_data_succeeds(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Valid CTC Employee").await;

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

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Valid CTC data should be accepted (no regression)"
    );

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["base_salary"].as_i64().unwrap(), 15000000);
    assert!(json["total_monthly_ctc"].as_i64().unwrap() > 0);
}

// ============================================================================
// Task 8: CTC Completeness Integration Tests
// ============================================================================

/// GET /ctc/completeness → correct department counts (HR user)
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_returns_department_counts(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    // Create department and resources
    let dept_id = create_test_department(&pool, "Engineering").await;
    let resource_with_ctc = create_test_resource_in_department(&pool, "Alice", dept_id).await;
    let _resource_without_ctc = create_test_resource_in_department(&pool, "Bob", dept_id).await;

    // Create HR user and get token
    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    // Create CTC record for one resource
    create_ctc_for_resource(&app, &hr_token, resource_with_ctc).await;

    // Fetch completeness
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/completeness")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // There should be at least one department in the result
    let departments = json["departments"].as_array().unwrap();
    assert!(
        !departments.is_empty(),
        "Should have at least one department"
    );

    let eng = departments
        .iter()
        .find(|d| d["department_id"].as_str() == Some(dept_id.to_string().as_str()));
    assert!(eng.is_some(), "Test department should be in results");

    let eng = eng.unwrap();
    assert_eq!(eng["total_employees"].as_i64().unwrap(), 2);
    assert_eq!(eng["with_ctc"].as_i64().unwrap(), 1);
    assert_eq!(eng["missing_ctc"].as_i64().unwrap(), 1);
}

/// GET /ctc/completeness as PM → 403 Forbidden
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_denied_for_pm(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let _pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let pm_token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/completeness")
        .header("Authorization", format!("Bearer {}", pm_token))
        .body(Body::empty())
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "Project Manager should be denied access to completeness"
    );
}

/// GET /ctc/completeness as Finance → 403 Forbidden
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_denied_for_finance(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let finance_email = test_email();
    let _finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let finance_token = get_auth_token(&app, &finance_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/completeness")
        .header("Authorization", format!("Bearer {}", finance_token))
        .body(Body::empty())
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "Finance should be denied access to completeness endpoint"
    );
}

/// GET /ctc/completeness/missing → correct employee list (HR only)
#[sqlx::test(migrations = "../../migrations")]
async fn missing_employees_returns_correct_list(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    // Setup department + resources
    let dept_id = create_test_department(&pool, "Marketing").await;
    let resource_with_ctc = create_test_resource_in_department(&pool, "Charlie", dept_id).await;
    let resource_without_ctc = create_test_resource_in_department(&pool, "Diana", dept_id).await;

    // HR user
    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    // Create CTC for one resource
    create_ctc_for_resource(&app, &hr_token, resource_with_ctc).await;

    // Fetch missing employees
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/completeness/missing")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let arr = json.as_array().unwrap();
    // Diana should be in the missing list; Charlie should NOT be
    let diana_found = arr.iter().any(|e| {
        e["id"]
            .as_str()
            .map(|s| s == resource_without_ctc.to_string())
            .unwrap_or(false)
    });
    let charlie_found = arr.iter().any(|e| {
        e["id"]
            .as_str()
            .map(|s| s == resource_with_ctc.to_string())
            .unwrap_or(false)
    });

    assert!(
        diana_found,
        "Diana (no CTC) should be in missing employees list"
    );
    assert!(
        !charlie_found,
        "Charlie (has CTC) should NOT be in missing employees list"
    );
}

// ============================================================================
// Task 8: BPJS Compliance Report Integration Tests
// ============================================================================

/// GET /ctc/compliance-report → returns report (HR user)
#[sqlx::test(migrations = "../../migrations")]
async fn compliance_report_returns_results_for_hr(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    // Create a resource and CTC record
    let resource_id = create_test_resource(&pool, "Compliance Test Employee").await;
    create_ctc_for_resource(&app, &hr_token, resource_id).await;

    // Run compliance report for a wide date range
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/compliance-report?start_date=2020-01-01&end_date=2030-12-31")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Should have summary fields
    assert!(
        json["total_validated"].is_number(),
        "total_validated should be present"
    );
    assert!(
        json["total_passed"].is_number(),
        "total_passed should be present"
    );
    assert!(
        json["total_discrepancies"].is_number(),
        "total_discrepancies should be present"
    );
    assert!(
        json["compliance_rate_pct"].is_number(),
        "compliance_rate_pct should be present"
    );

    // A freshly-created CTC should PASS compliance (BPJS calculated by same engine)
    let total = json["total_validated"].as_i64().unwrap();
    let passed = json["total_passed"].as_i64().unwrap();
    if total > 0 {
        assert_eq!(
            passed, total,
            "All freshly-created records should PASS compliance"
        );
    }
}

/// GET /ctc/compliance-report → returns report for Finance user
#[sqlx::test(migrations = "../../migrations")]
async fn compliance_report_accessible_by_finance(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    // Need HR to create CTC first
    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Finance Test Employee").await;
    create_ctc_for_resource(&app, &hr_token, resource_id).await;

    // Finance user fetches compliance report
    let finance_email = test_email();
    let _finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let finance_token = get_auth_token(&app, &finance_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/compliance-report?start_date=2020-01-01&end_date=2030-12-31")
        .header("Authorization", format!("Bearer {}", finance_token))
        .body(Body::empty())
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Finance should have access to compliance report"
    );
}

/// GET /ctc/compliance-report as PM → 403
#[sqlx::test(migrations = "../../migrations")]
async fn compliance_report_denied_for_pm(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let _pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let pm_token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/compliance-report?start_date=2020-01-01&end_date=2030-12-31")
        .header("Authorization", format!("Bearer {}", pm_token))
        .body(Body::empty())
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "PM should be denied access to compliance report"
    );
}

/// GET /ctc/compliance-report as department_head → 403
#[sqlx::test(migrations = "../../migrations")]
async fn compliance_report_denied_for_department_head(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dh_email = test_email();
    let _dh_id = create_test_user_with_role(&pool, &dh_email, "department_head").await;
    let dh_token = get_auth_token(&app, &dh_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/compliance-report?start_date=2020-01-01&end_date=2030-12-31")
        .header("Authorization", format!("Bearer {}", dh_token))
        .body(Body::empty())
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "Department Head should be denied access to compliance report"
    );
}

// ============================================================================
// Task 8: Allocation CTC Guard Integration Tests
// ============================================================================

/// POST /allocations for resource WITHOUT CTC → 400 rejection
#[sqlx::test(migrations = "../../migrations")]
async fn allocation_rejected_without_ctc(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    // Create admin user (can manage allocations)
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    // Create resource WITHOUT CTC
    let resource_id = create_test_resource(&pool, "No CTC Employee").await;

    // Create project
    let project_id = create_test_project(&pool, admin_id).await;

    // Attempt allocation
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", admin_token))
        .body(Body::from(
            json!({
                "project_id": project_id.to_string(),
                "resource_id": resource_id.to_string(),
                "start_date": "2025-03-01",
                "end_date": "2025-03-31",
                "allocation_percentage": 100.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Allocation should be rejected for resource without CTC"
    );

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let message = json["error"]["message"].as_str().unwrap_or("");
    assert!(
        message.contains("without CTC data"),
        "Error message should mention missing CTC data, got: {}",
        message
    );
}

/// POST /allocations for resource WITH CTC → 200 success
#[sqlx::test(migrations = "../../migrations")]
async fn allocation_succeeds_with_ctc(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    // Create HR user (to create CTC)
    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    // Create admin user (can manage allocations)
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    // Create resource WITH CTC
    let resource_id = create_test_resource(&pool, "Has CTC Employee").await;
    create_ctc_for_resource(&app, &hr_token, resource_id).await;

    // Create project
    let project_id = create_test_project(&pool, admin_id).await;

    // Attempt allocation — should succeed
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", admin_token))
        .body(Body::from(
            json!({
                "project_id": project_id.to_string(),
                "resource_id": resource_id.to_string(),
                "start_date": "2025-03-01",
                "end_date": "2025-03-31",
                "allocation_percentage": 100.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Allocation should succeed for resource with CTC"
    );
}

/// GET /ctc/compliance-report generates audit log
#[sqlx::test(migrations = "../../migrations")]
async fn compliance_report_creates_audit_log(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    // Run compliance report
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/compliance-report?start_date=2025-01-01&end_date=2025-12-31")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verify audit log entry was created
    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM audit_logs
         WHERE action = 'compliance_report_generated'
         AND entity_type = 'compliance_report'
         AND user_id = $1",
    )
    .bind(hr_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        audit_count, 1,
        "Compliance report should create an audit log entry"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_ctc_with_decimal_values_rejected(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource(&pool, "Decimal Test Employee").await;
    create_ctc_for_resource(&app, &hr_token, resource_id).await;

    let req = Request::builder()
        .method("PUT")
        .uri(&format!("/api/v1/ctc/{}/components", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "components": {
                    "base_salary": 15000000.50,
                    "hra_allowance": 3000000,
                    "medical_allowance": 1000000,
                    "transport_allowance": 500000,
                    "meal_allowance": 500000,
                    "bpjs_kesehatan_employer": 480000,
                    "bpjs_ketenagakerjaan_employer": 740948,
                    "thr_monthly_accrual": 833333,
                    "total_monthly_ctc": 14554281,
                    "daily_rate": "661558.22",
                    "working_days_per_month": 22,
                    "risk_tier": 1,
                    "thr_eligible": true
                },
                "reason": "Testing decimal rejection",
                "effective_date_policy": "pro_rata"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Decimal monetary values should be rejected in CTC update"
    );
}

// ============================================================================
// Story 6.5: CTC Completeness Dashboard regressions
// ============================================================================

const COMPLETENESS_SENSITIVE_FIELDS: &[&str] = &[
    "base_salary",
    "hra_allowance",
    "medical_allowance",
    "transport_allowance",
    "meal_allowance",
    "bpjs_kesehatan_employer",
    "bpjs_kesehatan_employee",
    "bpjs_ketenagakerjaan_employer",
    "bpjs_ketenagakerjaan_employee",
    "thr_monthly_accrual",
    "total_monthly_ctc",
    "daily_rate",
    "encrypted_components",
    "encrypted_daily_rate",
    "ciphertext",
    "key_version",
    "encryption_version",
    "encryption_algorithm",
    "encrypted_at",
    "components",
];

fn assert_no_sensitive_fields(value: &Value, path: &str) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key_lower = k.to_ascii_lowercase();
                for sensitive in COMPLETENESS_SENSITIVE_FIELDS {
                    assert!(
                        key_lower != *sensitive,
                        "Sensitive field '{}' leaked into completeness response at {}.{}",
                        sensitive,
                        path,
                        k
                    );
                }
                assert_no_sensitive_fields(v, &format!("{}.{}", path, k));
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                assert_no_sensitive_fields(item, &format!("{}[{}]", path, i));
            }
        }
        _ => {}
    }
}

/// GET /ctc/completeness returns the canonical Story-6.5 top-level shape:
/// `total_with_ctc`, `total_missing`, `overall_completion_pct`, and a bounded
/// `trend` array. The pre-6.5 names must not be present.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_exposes_story_6_5_top_level_fields(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept = create_test_department(&pool, "Compliance Field Department").await;
    let employee = create_test_resource_in_department(&pool, "Compliance Field Alice", dept).await;
    let _no_ctc = create_test_resource_in_department(&pool, "Compliance Field Bob", dept).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;
    create_ctc_for_resource(&app, &hr_token, employee).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/completeness")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Canonical top-level fields must exist.
    assert!(json.get("total_employees").is_some());
    assert!(json.get("total_with_ctc").is_some());
    assert!(json.get("total_missing").is_some());
    assert!(json.get("overall_completion_pct").is_some());

    // The pre-Story-6.5 top-level names are not part of the contract.
    assert!(
        json.get("with_ctc").is_none(),
        "Legacy top-level field `with_ctc` must not leak into completeness"
    );
    assert!(
        json.get("missing_ctc").is_none(),
        "Legacy top-level field `missing_ctc` must not leak into completeness"
    );
    assert!(
        json.get("completion_pct").is_none(),
        "Legacy top-level field `completion_pct` must not leak into completeness"
    );

    // Trend is present and bounded.
    let trend = json["trend"].as_array().expect("trend array present");
    assert!(
        !trend.is_empty(),
        "trend should contain at least one month bucket"
    );
    assert!(
        trend.len() <= 24,
        "trend must respect the bounded ceiling: got {}",
        trend.len()
    );
    // Months are stable YYYY-MM strings, ascending.
    let mut last_month: Option<String> = None;
    for point in trend {
        let month = point["month"]
            .as_str()
            .expect("month present and string-typed");
        assert!(
            month.len() == 7 && month.chars().nth(4) == Some('-'),
            "month must be YYYY-MM: got {}",
            month
        );
        if let Some(prev) = &last_month {
            assert!(
                month > prev.as_str(),
                "trend months must be ascending: {} after {}",
                month,
                prev
            );
        }
        last_month = Some(month.to_string());
        assert!(point.get("total_employees").is_some());
        assert!(point.get("total_with_ctc").is_some());
        assert!(point.get("completion_pct").is_some());
    }

    // No salary, daily rate, or encryption metadata in the dashboard payload.
    assert_no_sensitive_fields(&json, "$");
}

/// HR can hit `/ctc/completeness/missing`; PM, Finance, and DH cannot.
#[sqlx::test(migrations = "../../migrations")]
async fn missing_endpoint_is_hr_only(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    for role in ["project_manager", "finance", "department_head"] {
        let email = format!("missing-list-{}-{}@example.com", role, Uuid::new_v4());
        let _id = create_test_user_with_role(&pool, &email, role).await;
        let token = get_auth_token(&app, &email).await;

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/ctc/completeness/missing")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::FORBIDDEN,
            "Role '{}' must not access /ctc/completeness/missing",
            role
        );
    }
}

/// The missing-employee list exposes `id` as the resource id (Add CTC target)
/// and contains no encrypted, salary, or daily-rate fields.
#[sqlx::test(migrations = "../../migrations")]
async fn missing_employee_payload_uses_resource_id_and_has_no_sensitive_fields(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept = create_test_department(&pool, "MissingShapeDept").await;
    let with_ctc = create_test_resource_in_department(&pool, "MissingShape Eve", dept).await;
    let without_ctc = create_test_resource_in_department(&pool, "MissingShape Frank", dept).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;
    create_ctc_for_resource(&app, &hr_token, with_ctc).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/completeness/missing")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().expect("response is an array");

    let frank = arr
        .iter()
        .find(|e| e["id"].as_str() == Some(without_ctc.to_string().as_str()))
        .expect("Frank should appear");
    // The `id` is the resource id (the Add CTC target) and the name/department are surfaced.
    assert_eq!(frank["id"].as_str().unwrap(), without_ctc.to_string());
    assert_eq!(frank["name"].as_str().unwrap(), "MissingShape Frank");
    assert!(frank.get("department").is_some());

    // Sensitive fields must not leak through this dashboard payload.
    assert_no_sensitive_fields(&json, "$");
}

/// Department Head completeness response is scoped to their own department,
/// even when a different `department_id` query parameter is supplied.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_department_head_is_scoped_to_own_department(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_a = create_test_department(&pool, "DH Scope A").await;
    let dept_b = create_test_department(&pool, "DH Scope B").await;
    let _a_emp = create_test_resource_in_department(&pool, "Scope Alpha", dept_a).await;
    let _b_emp = create_test_resource_in_department(&pool, "Scope Bravo", dept_b).await;

    let dh_email = test_email();
    let dh_id = create_test_user_with_role(&pool, &dh_email, "department_head").await;
    // Bind the DH user to dept_a only.
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(dept_a)
        .bind(dh_id)
        .execute(&pool)
        .await
        .expect("DH department binding should succeed");
    // Make the DH the head of dept_a; completeness access now requires both
    // users.department_id and departments.head_id to agree.
    sqlx::query("UPDATE departments SET head_id = $1 WHERE id = $2")
        .bind(dh_id)
        .bind(dept_a)
        .execute(&pool)
        .await
        .expect("head assignment should succeed");

    let dh_token = get_auth_token(&app, &dh_email).await;

    // Even when DH explicitly requests dept_b, the response must remain scoped to dept_a.
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness?department_id={}", dept_b).as_str())
        .header("Authorization", format!("Bearer {}", dh_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let departments = json["departments"].as_array().unwrap();
    assert_eq!(
        departments.len(),
        1,
        "DH completeness must return exactly the DH department row; got {:?}",
        departments
    );
    let scoped_row = departments
        .iter()
        .find(|d| d["department_id"].as_str() == Some(dept_a.to_string().as_str()))
        .expect("dept_a row should be present for department head");
    assert_eq!(
        scoped_row["total_employees"].as_i64().unwrap(),
        1,
        "dept_a seeded employee must be counted"
    );
    assert_eq!(scoped_row["with_ctc"].as_i64().unwrap(), 0);
    assert_eq!(scoped_row["missing_ctc"].as_i64().unwrap(), 1);
    assert!(
        departments
            .iter()
            .all(|d| d["department_id"].as_str() == Some(dept_a.to_string().as_str())),
        "DH completeness must be scoped to dept_a only; got {:?}",
        departments
    );

    // No sensitive fields should leak.
    assert_no_sensitive_fields(&json, "$");
}

/// Department Head completeness must not fall through to an unfiltered report
/// when the user has no `users.department_id` assignment.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_department_head_without_department_returns_403(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept = create_test_department(&pool, "DH Orphan Leak Guard").await;
    let _emp = create_test_resource_in_department(&pool, "Orphan Scope Employee", dept).await;

    let dh_email = test_email();
    let _dh_id = create_test_user_with_role(&pool, &dh_email, "department_head").await;
    let dh_token = get_auth_token(&app, &dh_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/completeness")
        .header("Authorization", format!("Bearer {}", dh_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "department_head without department must not see global completeness"
    );
}

/// A `department_head` role plus `users.department_id` is not enough; the
/// user must also be the actual `departments.head_id` for that department.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_department_head_must_match_department_head_record(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept = create_test_department(&pool, "DH Head Relationship Guard").await;
    let _emp = create_test_resource_in_department(&pool, "Wrong Head Employee", dept).await;

    let dh_email = test_email();
    let dh_id = create_test_user_with_role(&pool, &dh_email, "department_head").await;
    let real_head_email = test_email();
    let real_head_id = create_test_user_with_role(&pool, &real_head_email, "department_head").await;

    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(dept)
        .bind(dh_id)
        .execute(&pool)
        .await
        .expect("DH department binding should succeed");
    sqlx::query("UPDATE departments SET head_id = $1 WHERE id = $2")
        .bind(real_head_id)
        .bind(dept)
        .execute(&pool)
        .await
        .expect("real head assignment should succeed");

    let dh_token = get_auth_token(&app, &dh_email).await;
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/completeness")
        .header("Authorization", format!("Bearer {}", dh_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "department_head must match departments.head_id for completeness access"
    );
}

/// With no employees in the database, totals are zero, `overall_completion_pct`
/// is `0.0`, the trend renders with bounded zero buckets (no NaN), and no
/// sensitive fields appear anywhere in the payload.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_empty_state_returns_zero_metrics_without_nan(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    // Filter by a fresh, empty department so even shared test data does not
    // contaminate the empty-state assertion.
    let empty_dept = create_test_department(&pool, "Story65Empty").await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness?department_id={}", empty_dept).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["total_employees"].as_i64().unwrap(), 0);
    assert_eq!(json["total_with_ctc"].as_i64().unwrap(), 0);
    assert_eq!(json["total_missing"].as_i64().unwrap(), 0);
    let pct = json["overall_completion_pct"].as_f64().unwrap();
    assert!(
        pct == 0.0,
        "empty-state overall completion must be 0.0 (no NaN/Inf), got {}",
        pct
    );

    // Trend is still emitted as a bounded run of zero buckets.
    let trend = json["trend"].as_array().expect("trend array");
    assert!(!trend.is_empty(), "trend should still emit zero-buckets");
    assert!(trend.len() <= 24);
    for point in trend {
        assert_eq!(point["total_employees"].as_i64().unwrap(), 0);
        assert_eq!(point["total_with_ctc"].as_i64().unwrap(), 0);
        assert_eq!(point["total_missing"].as_i64().unwrap(), 0);
        let p_pct = point["completion_pct"].as_f64().unwrap();
        assert!(
            p_pct == 0.0,
            "empty-bucket completion_pct must be 0.0, got {}",
            p_pct
        );
    }

    assert_no_sensitive_fields(&json, "$");
}

/// HR `department_id=X` must scope **every** visible completeness surface to X:
/// `departments` rows, `total_employees`, `total_with_ctc`, and the trend's
/// per-month head-counts. The story is explicit: filters affect all surfaces
/// consistently.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_hr_department_filter_scopes_all_surfaces(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let scoped = create_test_department(&pool, "Story65ScopedDept").await;
    let other = create_test_department(&pool, "Story65OtherDept").await;
    let scoped_emp = create_test_resource_in_department(&pool, "Scoped Alice", scoped).await;
    let _other_emp = create_test_resource_in_department(&pool, "Other Bob", other).await;
    let _other_emp2 = create_test_resource_in_department(&pool, "Other Cara", other).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;
    create_ctc_for_resource(&app, &hr_token, scoped_emp).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness?department_id={}", scoped).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Department rows only contain the scoped department.
    let departments = json["departments"].as_array().unwrap();
    assert!(
        departments
            .iter()
            .all(|d| d["department_id"].as_str() == Some(scoped.to_string().as_str())),
        "filtered completeness must contain only the scoped department: {:?}",
        departments
    );

    // Totals reflect only the scoped department's 1 employee.
    assert_eq!(json["total_employees"].as_i64().unwrap(), 1);
    assert_eq!(json["total_with_ctc"].as_i64().unwrap(), 1);
    assert_eq!(json["total_missing"].as_i64().unwrap(), 0);

    // Trend's latest bucket also sees only the scoped department's headcount.
    let trend = json["trend"].as_array().expect("trend present");
    let latest = trend.last().expect("at least one month bucket");
    assert_eq!(
        latest["total_employees"].as_i64().unwrap(),
        1,
        "trend headcount must be scoped to the filter department"
    );
    assert_eq!(latest["total_with_ctc"].as_i64().unwrap(), 1);
}

/// A malformed `department_id` (not a UUID) must be rejected by the typed
/// query extractor. The story requires "validate it through typed query
/// extraction"; silently ignoring the parameter is a scope-bleed risk.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_rejects_malformed_department_id(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/completeness?department_id=not-a-uuid")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Malformed department_id must be rejected, got {}",
        res.status()
    );
}

/// HR can scope `/ctc/completeness/missing` by `department_id=X` and the list
/// must only contain employees in that department.
#[sqlx::test(migrations = "../../migrations")]
async fn missing_endpoint_honors_hr_department_filter(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let scoped = create_test_department(&pool, "Story65MissingScope").await;
    let other = create_test_department(&pool, "Story65MissingOther").await;
    let _scoped_missing =
        create_test_resource_in_department(&pool, "Scoped Missing Alice", scoped).await;
    let _other_missing =
        create_test_resource_in_department(&pool, "Other Missing Bob", other).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness/missing?department_id={}", scoped).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().expect("missing array");

    for emp in arr {
        let dept = emp["department"].as_str().unwrap_or("");
        assert_eq!(
            dept, "Story65MissingScope",
            "missing list must only contain employees from the scoped department: {:?}",
            emp
        );
    }

    // The scoped employee is present.
    assert!(
        arr.iter()
            .any(|e| e["name"].as_str() == Some("Scoped Missing Alice")),
        "scoped missing employee must be included"
    );
    // The other-department employee is NOT present.
    assert!(
        !arr.iter()
            .any(|e| e["name"].as_str() == Some("Other Missing Bob")),
        "other-department missing employee must be excluded by filter"
    );
}

/// A non-Active (`Inactive`) ctc_records row must not count toward `with_ctc`
/// in either the department summary or the trend. The completeness contract
/// uses "currently-Active" CTC, not "ever had a CTC".
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_excludes_inactive_ctc_records(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept = create_test_department(&pool, "Story65InactiveDept").await;
    let employee = create_test_resource_in_department(&pool, "Inactive CTC Employee", dept).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;
    create_ctc_for_resource(&app, &hr_token, employee).await;

    // Flip the employee's CTC to Inactive — they should now be counted as
    // missing, not as having a CTC.
    sqlx::query("UPDATE ctc_records SET status = 'Inactive' WHERE resource_id = $1")
        .bind(employee)
        .execute(&pool)
        .await
        .expect("flip to inactive succeeds");

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness?department_id={}", dept).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["total_employees"].as_i64().unwrap(), 1);
    assert_eq!(
        json["total_with_ctc"].as_i64().unwrap(),
        0,
        "Inactive CTC must not count as 'with CTC'"
    );
    assert_eq!(json["total_missing"].as_i64().unwrap(), 1);

    // The latest trend bucket also reflects the Inactive state.
    let trend = json["trend"].as_array().unwrap();
    let latest = trend.last().expect("at least one bucket");
    assert_eq!(latest["total_employees"].as_i64().unwrap(), 1);
    assert_eq!(
        latest["total_with_ctc"].as_i64().unwrap(),
        0,
        "trend latest bucket must also exclude Inactive CTC"
    );

    // And the missing list contains the now-Inactive employee.
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness/missing?department_id={}", dept).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert!(
        arr.iter()
            .any(|e| e["id"].as_str() == Some(employee.to_string().as_str())),
        "Inactive-CTC employee must appear in the missing list"
    );
}

/// `resource_type != 'employee'` (e.g. contractors) must NOT contribute to
/// the completeness numerator or denominator. Project-context rule #9 is
/// explicit: completeness is employee-only.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_excludes_non_employee_resources(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept = create_test_department(&pool, "Story65NonEmpDept").await;

    // One real employee, no CTC.
    let _emp = create_test_resource_in_department(&pool, "Story65 Employee", dept).await;

    // One non-employee resource (contractor) in the same department. Must be
    // ignored entirely by the completeness service.
    sqlx::query(
        "INSERT INTO resources (name, resource_type, capacity, department_id)
         VALUES ($1, 'contractor', 1.0, $2)",
    )
    .bind("Story65 Contractor")
    .bind(dept)
    .execute(&pool)
    .await
    .expect("contractor row inserted");

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness?department_id={}", dept).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Only the employee shows up — the contractor is invisible to completeness.
    assert_eq!(
        json["total_employees"].as_i64().unwrap(),
        1,
        "non-employee resources must not contribute to total_employees: {}",
        json
    );

    // Missing list contains the employee, not the contractor.
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness/missing?department_id={}", dept).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert!(
        !arr.iter()
            .any(|e| e["name"].as_str() == Some("Story65 Contractor")),
        "contractor must not appear in missing-CTC list"
    );
}

/// Trend snapshot moves correctly when a new CTC record is created after the
/// historical window starts. The current month must include the new employee.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_trend_reflects_new_active_ctc(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept = create_test_department(&pool, "Trend Department").await;
    let alice = create_test_resource_in_department(&pool, "Trend Alice", dept).await;
    let _bob = create_test_resource_in_department(&pool, "Trend Bob", dept).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;
    create_ctc_for_resource(&app, &hr_token, alice).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness?department_id={}", dept).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let trend = json["trend"].as_array().expect("trend array present");
    assert!(!trend.is_empty(), "trend should have at least one bucket");
    let last = trend.last().unwrap();
    // The latest bucket should include both employees (created today) and one
    // active CTC (Alice). Bob has no CTC so he counts as missing.
    assert_eq!(
        last["total_employees"].as_i64().unwrap(),
        2,
        "latest trend month should see exactly Alice and Bob"
    );
    assert_eq!(
        last["total_with_ctc"].as_i64().unwrap(),
        1,
        "latest trend month should reflect exactly Alice's active CTC"
    );
    assert_eq!(
        last["total_missing"].as_i64().unwrap(),
        1,
        "Bob should be missing CTC in the latest trend month"
    );
    assert_eq!(last["completion_pct"].as_f64().unwrap(), 50.0);
}

/// Multiple employees with different effective months should produce exact
/// month buckets and percentages, not just a non-empty latest trend.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_trend_returns_exact_multi_month_percentages(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept = create_test_department(&pool, "Trend Multi Month Department").await;
    let alice = create_test_resource_in_department(&pool, "Trend Multi Alice", dept).await;
    let bob = create_test_resource_in_department(&pool, "Trend Multi Bob", dept).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;
    create_ctc_for_resource(&app, &hr_token, alice).await;
    create_ctc_for_resource(&app, &hr_token, bob).await;

    let two_months_ago = month_start_months_ago(2);
    let previous_month = month_start_months_ago(1);
    let current_month = month_start_months_ago(0);
    let previous_key = month_key(previous_month);
    let current_key = month_key(current_month);

    sqlx::query("UPDATE resources SET created_at = $1::date WHERE id IN ($2, $3)")
        .bind(two_months_ago)
        .bind(alice)
        .bind(bob)
        .execute(&pool)
        .await
        .expect("resource created_at backdated");
    sqlx::query("UPDATE ctc_records SET effective_date = $1 WHERE resource_id = $2")
        .bind(previous_month)
        .bind(alice)
        .execute(&pool)
        .await
        .expect("alice CTC effective date updated");
    sqlx::query("UPDATE ctc_records SET effective_date = $1 WHERE resource_id = $2")
        .bind(current_month)
        .bind(bob)
        .execute(&pool)
        .await
        .expect("bob CTC effective date updated");

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness?department_id={}", dept).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let trend = json["trend"].as_array().expect("trend array present");
    let previous = trend
        .iter()
        .find(|point| point["month"].as_str() == Some(previous_key.as_str()))
        .expect("previous month bucket should be present");
    assert_eq!(previous["total_employees"].as_i64().unwrap(), 2);
    assert_eq!(previous["total_with_ctc"].as_i64().unwrap(), 1);
    assert_eq!(previous["total_missing"].as_i64().unwrap(), 1);
    assert_eq!(previous["completion_pct"].as_f64().unwrap(), 50.0);

    let current = trend
        .iter()
        .find(|point| point["month"].as_str() == Some(current_key.as_str()))
        .expect("current month bucket should be present");
    assert_eq!(current["total_employees"].as_i64().unwrap(), 2);
    assert_eq!(current["total_with_ctc"].as_i64().unwrap(), 2);
    assert_eq!(current["total_missing"].as_i64().unwrap(), 0);
    assert_eq!(current["completion_pct"].as_f64().unwrap(), 100.0);
}

/// Legacy/resource-import rows can have NULL `resources.created_at` because
/// the initial schema defaulted it but did not require NOT NULL. Those rows
/// should still appear in current completeness, but must not inflate every
/// historical trend bucket.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_trend_null_resource_created_at_counts_current_month_only(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept = create_test_department(&pool, "Trend Null Created Department").await;
    let employee = create_test_resource_in_department(&pool, "Trend Null Created", dept).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;
    create_ctc_for_resource(&app, &hr_token, employee).await;

    let previous_month = month_start_months_ago(1);
    let previous_key = month_key(previous_month);
    let current_key = month_key(month_start_months_ago(0));

    sqlx::query("UPDATE resources SET created_at = NULL WHERE id = $1")
        .bind(employee)
        .execute(&pool)
        .await
        .expect("resource created_at nullable in legacy schema");
    sqlx::query("UPDATE ctc_records SET effective_date = $1 WHERE resource_id = $2")
        .bind(previous_month)
        .bind(employee)
        .execute(&pool)
        .await
        .expect("CTC effective date backdated");

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness?department_id={}", dept).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["total_employees"].as_i64().unwrap(), 1);
    assert_eq!(json["total_with_ctc"].as_i64().unwrap(), 1);

    let trend = json["trend"].as_array().expect("trend array present");
    let previous = trend
        .iter()
        .find(|point| point["month"].as_str() == Some(previous_key.as_str()))
        .expect("previous month bucket should be present");
    assert_eq!(previous["total_employees"].as_i64().unwrap(), 0);
    assert_eq!(previous["total_with_ctc"].as_i64().unwrap(), 0);

    let current = trend
        .iter()
        .find(|point| point["month"].as_str() == Some(current_key.as_str()))
        .expect("current month bucket should be present");
    assert_eq!(current["total_employees"].as_i64().unwrap(), 1);
    assert_eq!(current["total_with_ctc"].as_i64().unwrap(), 1);
}

/// The current-month trend bucket is a live as-of-today snapshot, not a
/// projection through the future end of the current month.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_trend_current_month_excludes_future_effective_ctc(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept = create_test_department(&pool, "Trend Future Effective Department").await;
    let employee =
        create_test_resource_in_department(&pool, "Trend Future Effective Employee", dept).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;
    create_ctc_for_resource(&app, &hr_token, employee).await;

    let tomorrow = Utc::now()
        .date_naive()
        .checked_add_signed(chrono::Duration::days(1))
        .expect("tomorrow date");
    sqlx::query("UPDATE ctc_records SET effective_date = $1 WHERE resource_id = $2")
        .bind(tomorrow)
        .bind(employee)
        .execute(&pool)
        .await
        .expect("future-date CTC effective date");

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness?department_id={}", dept).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Current rollup keeps existing Active-only semantics.
    assert_eq!(json["total_employees"].as_i64().unwrap(), 1);
    assert_eq!(json["total_with_ctc"].as_i64().unwrap(), 1);

    // The latest trend bucket is as-of today, so tomorrow's effective CTC is
    // not counted yet.
    let trend = json["trend"].as_array().expect("trend array present");
    let latest = trend.last().expect("latest trend bucket");
    assert_eq!(latest["total_employees"].as_i64().unwrap(), 1);
    assert_eq!(latest["total_with_ctc"].as_i64().unwrap(), 0);
    assert_eq!(latest["total_missing"].as_i64().unwrap(), 1);
    assert_eq!(latest["completion_pct"].as_f64().unwrap(), 0.0);
}

/// `/ctc/completeness` is a polled dashboard read — it must not write an
/// audit-log row each tick. The story is explicit: "do not add noisy audit
/// rows for ordinary dashboard reads unless a current route already audits
/// that read." A regression here would multiply audit volume by the polling
/// frequency for every HR user.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_dashboard_read_does_not_emit_audit_log(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    // Baseline: count audit rows attributed to this HR user before the read.
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE user_id = $1")
        .bind(hr_id)
        .fetch_one(&pool)
        .await
        .expect("baseline audit count");

    // Poll the dashboard read three times to simulate the 30s polling loop.
    for _ in 0..3 {
        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/ctc/completeness")
            .header("Authorization", format!("Bearer {}", hr_token))
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE user_id = $1")
        .bind(hr_id)
        .fetch_one(&pool)
        .await
        .expect("post-read audit count");
    assert_eq!(
        after, before,
        "completeness dashboard reads must not emit audit rows: before={}, after={}",
        before, after
    );
}

/// Cross-row invariant: the per-department `total_employees` / `with_ctc` /
/// `missing_ctc` columns must sum to the top-level totals. Catches potential
/// LEFT JOIN row multiplication, double-counting, or aggregation drift —
/// failure modes the per-row tests cannot see by themselves.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_department_rows_sum_matches_top_level_totals(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    // Two departments with mixed CTC coverage.
    let dept_a = create_test_department(&pool, "Story65SumDeptA").await;
    let dept_b = create_test_department(&pool, "Story65SumDeptB").await;
    let a_with = create_test_resource_in_department(&pool, "Sum Alice", dept_a).await;
    let _a_missing = create_test_resource_in_department(&pool, "Sum Aaron", dept_a).await;
    let b_with = create_test_resource_in_department(&pool, "Sum Beth", dept_b).await;
    let _b_missing1 = create_test_resource_in_department(&pool, "Sum Boris", dept_b).await;
    let _b_missing2 = create_test_resource_in_department(&pool, "Sum Bo", dept_b).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;
    create_ctc_for_resource(&app, &hr_token, a_with).await;
    create_ctc_for_resource(&app, &hr_token, b_with).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ctc/completeness")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let top_total: i64 = json["total_employees"].as_i64().unwrap();
    let top_with: i64 = json["total_with_ctc"].as_i64().unwrap();
    let top_missing: i64 = json["total_missing"].as_i64().unwrap();
    let departments = json["departments"].as_array().unwrap();

    let sum_total: i64 = departments
        .iter()
        .map(|d| d["total_employees"].as_i64().unwrap_or(0))
        .sum();
    let sum_with: i64 = departments
        .iter()
        .map(|d| d["with_ctc"].as_i64().unwrap_or(0))
        .sum();
    let sum_missing: i64 = departments
        .iter()
        .map(|d| d["missing_ctc"].as_i64().unwrap_or(0))
        .sum();

    assert_eq!(
        sum_total, top_total,
        "Σ department.total_employees ({}) must equal top-level total_employees ({}): {:?}",
        sum_total, top_total, departments
    );
    assert_eq!(
        sum_with, top_with,
        "Σ department.with_ctc ({}) must equal top-level total_with_ctc ({})",
        sum_with, top_with
    );
    assert_eq!(
        sum_missing, top_missing,
        "Σ department.missing_ctc ({}) must equal top-level total_missing ({})",
        sum_missing, top_missing
    );

    // And the per-row invariant must hold for every row.
    for d in departments {
        let t = d["total_employees"].as_i64().unwrap();
        let w = d["with_ctc"].as_i64().unwrap();
        let m = d["missing_ctc"].as_i64().unwrap();
        assert_eq!(
            t,
            w + m,
            "per-row invariant total = with + missing failed for {:?}",
            d
        );
        assert!(
            w <= t,
            "with_ctc must never exceed total_employees: {:?}",
            d
        );
    }
}

/// A well-formed but unknown `department_id` UUID must produce an empty,
/// bounded payload (no 404 / 500). Mirrors the empty-state invariant but
/// drives it through the typed query path instead of an empty department —
/// guards against the SQL filter accidentally short-circuiting to "all rows"
/// or the handler treating "no match" as a not-found error.
#[sqlx::test(migrations = "../../migrations")]
async fn completeness_unknown_department_id_returns_empty_payload(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    set_ctc_crypto_env();
    let app = xynergy_backend::create_app(pool.clone());

    // Seed shared data the test must NOT accidentally see leak through the filter.
    let real = create_test_department(&pool, "Story65UnknownReal").await;
    let _real_emp =
        create_test_resource_in_department(&pool, "Unknown Filter Real Employee", real).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    // A random, well-formed UUID that is not in the departments table.
    let unknown = Uuid::new_v4();

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/completeness?department_id={}", unknown).as_str())
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Unknown-but-well-formed department_id must succeed with an empty payload, not 404/500"
    );
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let departments = json["departments"].as_array().unwrap();
    assert!(
        departments.is_empty(),
        "unknown department_id must yield no department rows, got {:?}",
        departments
    );
    assert_eq!(json["total_employees"].as_i64().unwrap(), 0);
    assert_eq!(json["total_with_ctc"].as_i64().unwrap(), 0);
    assert_eq!(json["total_missing"].as_i64().unwrap(), 0);
    assert_eq!(json["overall_completion_pct"].as_f64().unwrap(), 0.0);

    // Trend is still bounded and emits zero buckets — no NaN, no overflow.
    let trend = json["trend"].as_array().expect("trend array");
    assert!(
        !trend.is_empty(),
        "trend must still emit bounded zero buckets"
    );
    assert!(trend.len() <= 24);
    for point in trend {
        assert_eq!(point["total_employees"].as_i64().unwrap(), 0);
        assert_eq!(point["total_with_ctc"].as_i64().unwrap(), 0);
        assert_eq!(point["completion_pct"].as_f64().unwrap(), 0.0);
    }

    assert_no_sensitive_fields(&json, "$");
}
