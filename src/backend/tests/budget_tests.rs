use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("budget-{}@example.com", Uuid::new_v4())
}

fn set_test_env() {
    std::env::set_var("JWT_SECRET", "test-secret");
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
    .expect("test user created")
}

async fn create_test_department(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("INSERT INTO departments (name) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("department created")
}

async fn create_test_resource_in_dept(pool: &PgPool, name: &str, dept_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO resources (name, resource_type, capacity, department_id)
         VALUES ($1, 'human', 1.0, $2)
         RETURNING id",
    )
    .bind(name)
    .bind(dept_id)
    .fetch_one(pool)
    .await
    .expect("resource created")
}

async fn assign_user_to_department(pool: &PgPool, user_id: Uuid, dept_id: Uuid) {
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(dept_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("user dept assigned");
}

async fn create_ctc_for_resource(pool: &PgPool, resource_id: Uuid, user_id: Uuid) {
    use xynergy_backend::services::ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService};
    use xynergy_backend::services::key_provider::EnvKeyProvider;

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let encrypted_daily_rate = crypto_svc
        .encrypt_components(&json!({"daily_rate": "1200000"}))
        .await
        .expect("daily rate encryption should work");
    let encrypted_components = crypto_svc
        .encrypt_components(&json!({"base_salary": 10000000}))
        .await
        .expect("components encryption should work");

    sqlx::query(
        "INSERT INTO ctc_records (resource_id, components, encrypted_components, encrypted_daily_rate, key_version, encryption_version, encryption_algorithm, encrypted_at, daily_rate, working_days_per_month, effective_date, status, created_by, created_at, updated_by, reason)
         VALUES ($1, '{}'::jsonb, $2, $3, $4, $5, $6, $7, 1200000, 22, CURRENT_DATE, 'Active', $8, CURRENT_TIMESTAMP, $8, 'Test CTC')",
    )
    .bind(resource_id)
    .bind(&encrypted_components.ciphertext)
    .bind(&encrypted_daily_rate.ciphertext)
    .bind(&encrypted_daily_rate.key_version)
    .bind(&encrypted_daily_rate.encryption_version)
    .bind(&encrypted_daily_rate.algorithm)
    .bind(encrypted_daily_rate.encrypted_at)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("CTC record created");
}

async fn create_test_project(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, status, start_date, end_date)
         VALUES ($1, 'Active', CURRENT_DATE, CURRENT_DATE + INTERVAL '90 days')
         RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("project created")
}

async fn create_allocation(pool: &PgPool, resource_id: Uuid, project_id: Uuid, pct: f64) {
    sqlx::query(
        "INSERT INTO allocations (resource_id, project_id, allocation_percentage, start_date, end_date, include_weekend)
         VALUES ($1, $2, $3, CURRENT_DATE, CURRENT_DATE + INTERVAL '60 days', false)",
    )
    .bind(resource_id)
    .bind(project_id)
    .bind(sqlx::types::BigDecimal::try_from(pct).expect("allocation percentage should convert"))
    .execute(pool)
    .await
    .expect("allocation created");
}

async fn get_auth_token(app: &axum::Router, email: &str) -> String {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"email": email, "password": "Password123!"}).to_string(),
        ))
        .expect("login request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("login should return response");
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("login response body should be readable");
    let body: Value =
        serde_json::from_slice(&bytes).expect("login response should be valid JSON payload");
    body["token"]
        .as_str()
        .expect("login response should include token")
        .to_string()
}

fn current_period() -> String {
    let now = chrono::Local::now().date_naive();
    format!("{:04}-{:02}", now.year(), now.month())
}

use chrono::Datelike;

// --- Budget CRUD Tests ---

#[sqlx::test(migrations = "../../migrations")]
async fn create_budget_for_department(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let token = get_auth_token(&app, &email).await;

    let period = current_period();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": 10_000_000,
                "alert_threshold_pct": 80
            })
            .to_string(),
        ))
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert!(body["id"].as_str().is_some(), "response should have id");
    assert_eq!(body["department_id"].as_str().unwrap(), dept_id.to_string());
    assert_eq!(body["budget_period"].as_str().unwrap(), period);
    assert_eq!(body["total_budget_idr"].as_i64().unwrap(), 10_000_000);
    assert_eq!(body["alert_threshold_pct"].as_i64().unwrap(), 80);
}

#[sqlx::test(migrations = "../../migrations")]
async fn upsert_budget_updates_existing(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // First POST
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": 10_000_000,
                "alert_threshold_pct": 80
            })
            .to_string(),
        ))
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let first: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    let first_id = first["id"].as_str().unwrap().to_string();

    // Second POST — update same period
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": 20_000_000,
                "alert_threshold_pct": 70
            })
            .to_string(),
        ))
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let second: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert_eq!(second["id"].as_str().unwrap(), first_id, "same budget row should be updated");
    assert_eq!(second["total_budget_idr"].as_i64().unwrap(), 20_000_000);
    assert_eq!(second["alert_threshold_pct"].as_i64().unwrap(), 70);
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_budget_invalid_period_format(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let token = get_auth_token(&app, &email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": "2026-1",
                "total_budget_idr": 10_000_000
            })
            .to_string(),
        ))
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_budget_negative_amount(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let token = get_auth_token(&app, &email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": current_period(),
                "total_budget_idr": -100
            })
            .to_string(),
        ))
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_budget_invalid_threshold(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // threshold too low (30)
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": 10_000_000,
                "alert_threshold_pct": 30
            })
            .to_string(),
        ))
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // threshold too high (101)
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": 10_000_000,
                "alert_threshold_pct": 101
            })
            .to_string(),
        ))
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// --- Budget Summary Tests ---

#[sqlx::test(migrations = "../../migrations")]
async fn get_budget_summary_with_allocations(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "Dev1", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;

    let project_id = create_test_project(&pool, "Project Alpha").await;
    create_allocation(&pool, resource_id, project_id, 100.0).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // Set budget
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": 100_000_000
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    // GET summary
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert_eq!(body["department_id"].as_str().unwrap(), dept_id.to_string());
    assert_eq!(body["budget_period"].as_str().unwrap(), period);
    assert_eq!(body["total_budget_idr"].as_i64().unwrap(), 100_000_000);
    assert!(body["total_committed_idr"].as_i64().unwrap() > 0, "committed should be > 0 with active allocation");
    assert_eq!(body["spent_actual_source"].as_str().unwrap(), "committed_proxy");
    assert_eq!(body["spent_actual_idr"].as_i64().unwrap(), body["total_committed_idr"].as_i64().unwrap());
    assert!(body["budget_configured"].as_bool().unwrap());
    assert!(body["utilization_percentage"].as_f64().is_some());
    assert!(body["budget_health"].as_str().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_budget_summary_no_budget_configured(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // GET summary without creating budget
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert!(!body["budget_configured"].as_bool().unwrap(), "budget_configured should be false");
    assert_eq!(body["total_budget_idr"].as_i64().unwrap(), 0);
    assert_eq!(body["utilization_percentage"].as_f64().unwrap(), 0.0);
}

// --- Breakdown Tests ---

#[sqlx::test(migrations = "../../migrations")]
async fn get_budget_breakdown_single_period(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "Dev1", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;

    let project_id = create_test_project(&pool, "Project Alpha").await;
    create_allocation(&pool, resource_id, project_id, 100.0).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget/breakdown?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert_eq!(body["department_id"].as_str().unwrap(), dept_id.to_string());
    let by_employee = body["by_employee"].as_array().expect("by_employee should be array");
    assert!(!by_employee.is_empty(), "should have employee entries");
    let by_project = body["by_project"].as_array().expect("by_project should be array");
    assert!(!by_project.is_empty(), "should have project entries");
    let by_period = body["by_period"].as_array().expect("by_period should be array");
    assert!(!by_period.is_empty(), "should have period entries");
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_budget_breakdown_invalid_mixed_mode(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // Both period AND start_period — invalid
    let req = Request::builder()
        .method("GET")
        .uri(&format!(
            "/api/v1/team/budget/breakdown?period={}&start_period={}&end_period={}",
            period, period, period
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// --- Budget Health Tests ---

#[sqlx::test(migrations = "../../migrations")]
async fn budget_health_healthy(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "Dev1", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;

    let project_id = create_test_project(&pool, "Project Alpha").await;
    create_allocation(&pool, resource_id, project_id, 100.0).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // Set a very large budget so utilization is <50% => healthy
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": 1_000_000_000 // 1 billion — committed will be far less than 50%
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert_eq!(body["budget_health"].as_str().unwrap(), "healthy");
    let utilization = body["utilization_percentage"].as_f64().unwrap();
    assert!(utilization < 50.0, "utilization {:.1}% should be < 50% for healthy", utilization);
}

#[sqlx::test(migrations = "../../migrations")]
async fn budget_health_warning(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "Dev1", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;

    let project_id = create_test_project(&pool, "Project Alpha").await;
    create_allocation(&pool, resource_id, project_id, 100.0).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // First GET summary to see committed amount
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let pre: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    let committed = pre["total_committed_idr"].as_i64().unwrap();

    // Set budget so utilization is ~65% (between 50-80%) => warning
    // budget = committed * 100 / 65
    let budget = (committed as f64 * 100.0 / 65.0) as i64;
    let budget = budget.max(1); // avoid zero

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": budget
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    // GET summary
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert_eq!(body["budget_health"].as_str().unwrap(), "warning");
    let utilization = body["utilization_percentage"].as_f64().unwrap();
    assert!(utilization >= 50.0 && utilization < 80.0, "utilization {:.1}% should be 50-80% for warning", utilization);
}

#[sqlx::test(migrations = "../../migrations")]
async fn budget_health_critical(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "Dev1", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;

    let project_id = create_test_project(&pool, "Project Alpha").await;
    create_allocation(&pool, resource_id, project_id, 100.0).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // First GET summary to see committed amount
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let pre: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    let committed = pre["total_committed_idr"].as_i64().unwrap();

    // Set budget so utilization is ~90% (>=80%) => critical
    let budget = (committed as f64 * 100.0 / 90.0) as i64;
    let budget = budget.max(1);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": budget
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    // GET summary
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert_eq!(body["budget_health"].as_str().unwrap(), "critical");
    let utilization = body["utilization_percentage"].as_f64().unwrap();
    assert!(utilization >= 80.0, "utilization {:.1}% should be >=80% for critical", utilization);
}

// --- Custom Threshold Test ---

#[sqlx::test(migrations = "../../migrations")]
async fn custom_threshold_alert(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "Dev1", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;

    let project_id = create_test_project(&pool, "Project Alpha").await;
    create_allocation(&pool, resource_id, project_id, 100.0).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // Get committed amount first
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let pre: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    let committed = pre["total_committed_idr"].as_i64().unwrap();

    // Set budget so utilization is ~65%, threshold=60
    // This means utilization (65%) >= threshold (60%) so alert should trigger
    // But budget_health is "warning" (50-80%) based on fixed bands
    let budget = (committed as f64 * 100.0 / 65.0) as i64;
    let budget = budget.max(1);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": budget,
                "alert_threshold_pct": 60
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    // GET summary
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    let utilization = body["utilization_percentage"].as_f64().unwrap();
    let threshold = body["alert_threshold_pct"].as_i64().unwrap();

    assert!(utilization >= threshold as f64, "utilization {:.1}% should be >= threshold {}%", utilization, threshold);
    assert_eq!(body["budget_health"].as_str().unwrap(), "warning", "health based on fixed bands, not threshold");
}

// --- Auth Tests ---

#[sqlx::test(migrations = "../../migrations")]
async fn dept_head_sees_own_department_budget(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // POST budget
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": 10_000_000
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    // GET budget
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(body["department_id"].as_str().unwrap(), dept_id.to_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_can_target_other_department(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_dept_id = create_test_department(&pool, "HR").await;
    let eng_dept_id = create_test_department(&pool, "Engineering").await;

    let hr_email = test_email();
    let hr_user_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    assign_user_to_department(&pool, hr_user_id, hr_dept_id).await;

    let token = get_auth_token(&app, &hr_email).await;
    let period = current_period();

    // POST budget targeting Engineering department
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": 50_000_000,
                "department_id": eng_dept_id
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(body["department_id"].as_str().unwrap(), eng_dept_id.to_string());

    // GET budget for Engineering
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}&department_id={}", period, eng_dept_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(body["department_id"].as_str().unwrap(), eng_dept_id.to_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_gets_403_on_budget(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "project_manager").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // POST budget — should 403
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": 10_000_000
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // GET budget — should 403
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// --- No-Budget Fallback Test ---

#[sqlx::test(migrations = "../../migrations")]
async fn no_budget_returns_fallback_contract(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "Dev1", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;

    let project_id = create_test_project(&pool, "Project Alpha").await;
    create_allocation(&pool, resource_id, project_id, 100.0).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // GET without setting budget first
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert!(!body["budget_configured"].as_bool().unwrap());
    assert_eq!(body["total_budget_idr"].as_i64().unwrap(), 0);
    assert_eq!(body["utilization_percentage"].as_f64().unwrap(), 0.0);
    assert_eq!(body["spent_actual_source"].as_str().unwrap(), "committed_proxy");
    let committed = body["total_committed_idr"].as_i64().unwrap();
    assert_eq!(body["spent_actual_idr"].as_i64().unwrap(), committed);
}

// --- Spent Proxy Test ---

#[sqlx::test(migrations = "../../migrations")]
async fn spent_actual_equals_committed(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "Dev1", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;

    let project_id = create_test_project(&pool, "Project Alpha").await;
    create_allocation(&pool, resource_id, project_id, 100.0).await;

    let token = get_auth_token(&app, &email).await;
    let period = current_period();

    // Set budget
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/team/budget")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "budget_period": period,
                "total_budget_idr": 100_000_000
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    // GET summary
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/team/budget?period={}", period))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("readable");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert_eq!(body["spent_actual_source"].as_str().unwrap(), "committed_proxy");
    assert_eq!(
        body["spent_actual_idr"].as_i64().unwrap(),
        body["total_committed_idr"].as_i64().unwrap(),
        "spent_actual_idr must equal total_committed_idr"
    );
}
