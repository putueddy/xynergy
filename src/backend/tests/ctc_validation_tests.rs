//! Integration tests for CTC validation, completeness, and compliance (Story 2.4)

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
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
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO departments (name) VALUES ($1) RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("test department should be created")
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

async fn create_test_resource_in_department(pool: &PgPool, name: &str, dept_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO resources (name, resource_type, capacity, department_id, status)
         VALUES ($1, 'human', 1.0, $2, 'Active')
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
    assert!(!departments.is_empty(), "Should have at least one department");

    // Find our Engineering department
    let eng = departments
        .iter()
        .find(|d| d["department"].as_str() == Some("Engineering"));
    assert!(eng.is_some(), "Engineering department should be in results");

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
        e["id"].as_str().map(|s| s == resource_without_ctc.to_string()).unwrap_or(false)
    });
    let charlie_found = arr.iter().any(|e| {
        e["id"].as_str().map(|s| s == resource_with_ctc.to_string()).unwrap_or(false)
    });

    assert!(diana_found, "Diana (no CTC) should be in missing employees list");
    assert!(!charlie_found, "Charlie (has CTC) should NOT be in missing employees list");
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
    assert!(json["total_validated"].is_number(), "total_validated should be present");
    assert!(json["total_passed"].is_number(), "total_passed should be present");
    assert!(json["total_discrepancies"].is_number(), "total_discrepancies should be present");
    assert!(json["compliance_rate_pct"].is_number(), "compliance_rate_pct should be present");

    // A freshly-created CTC should PASS compliance (BPJS calculated by same engine)
    let total = json["total_validated"].as_i64().unwrap();
    let passed = json["total_passed"].as_i64().unwrap();
    if total > 0 {
        assert_eq!(passed, total, "All freshly-created records should PASS compliance");
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
    let message = json["message"].as_str().unwrap_or("");
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
