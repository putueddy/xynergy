use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::{Datelike, Local, NaiveDate, Weekday};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("cost-preview-{}@example.com", Uuid::new_v4())
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

async fn assign_user_to_department(pool: &PgPool, user_id: Uuid, dept_id: Uuid) {
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(dept_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("user dept assigned");
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

async fn create_ctc_for_resource(pool: &PgPool, resource_id: Uuid, user_id: Uuid) {
    use xynergy_backend::services::ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService};
    use xynergy_backend::services::key_provider::EnvKeyProvider;

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let encrypted_daily_rate = crypto_svc
        .encrypt_components(&json!({"daily_rate": "1200000"}))
        .await
        .expect("daily rate encryption should work");

    sqlx::query(
        "INSERT INTO ctc_records (resource_id, components, encrypted_components, encrypted_daily_rate, key_version, encryption_version, encryption_algorithm, encrypted_at, daily_rate, working_days_per_month, effective_date, status, created_by, created_at, updated_by, reason)
         VALUES ($1, '{}'::jsonb, $2, $3, $4, $5, $6, $7, 1200000, 22, CURRENT_DATE, 'Active', $8, CURRENT_TIMESTAMP, $8, 'Test CTC')",
    )
    .bind(resource_id)
    .bind(&encrypted_daily_rate.ciphertext)
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
         VALUES ($1, 'Active', CURRENT_DATE, CURRENT_DATE + INTERVAL '120 days')
         RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("project created")
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

fn next_weekday(mut date: NaiveDate, weekday: Weekday) -> NaiveDate {
    while date.weekday() != weekday {
        date = date.succ_opt().expect("next date should exist");
    }
    date
}

fn preview_uri(
    resource_id: Uuid,
    project_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
    allocation_percentage: f64,
    include_weekend: bool,
) -> String {
    format!(
        "/api/v1/allocations/cost-preview?resource_id={}&project_id={}&start_date={}&end_date={}&allocation_percentage={}&include_weekend={}",
        resource_id,
        project_id,
        start_date.format("%Y-%m-%d"),
        end_date.format("%Y-%m-%d"),
        allocation_percentage,
        include_weekend
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn cost_preview_happy_path_returns_expected_formula(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;
    let resource_id = create_test_resource_in_dept(&pool, "Preview Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;
    let project_id = create_test_project(&pool, "Preview Project").await;

    let token = get_auth_token(&app, &email).await;
    let monday = next_weekday(Local::now().date_naive() + chrono::Duration::days(7), Weekday::Mon);
    let friday = monday + chrono::Duration::days(4);

    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, monday, friday, 50.0, false))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["daily_rate_idr"], 1_200_000);
    assert_eq!(body["working_days"], 5);
    assert_eq!(body["allocation_percentage"], 50.0);
    assert_eq!(body["total_cost_idr"], 3_000_000);
    assert!(body["budget_impact"].is_null());
}

#[sqlx::test(migrations = "../../migrations")]
async fn cost_preview_validates_create_constraints(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "admin").await;
    let resource_id = create_test_resource_in_dept(&pool, "Validation Resource", dept_id).await;
    let project_id = create_test_project(&pool, "Validation Project").await;
    let token = get_auth_token(&app, &email).await;

    let today = Local::now().date_naive();
    let start = today + chrono::Duration::days(10);
    let end = today + chrono::Duration::days(5);

    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, start, end, 50.0, false))
        .header("Authorization", format!("Bearer {}", token.clone()))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, end, end, 0.0, false))
        .header("Authorization", format!("Bearer {}", token.clone()))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, end, end + chrono::Duration::days(3), 50.0, false))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("Cannot assign resource without CTC data"));

    let _ = user_id;
}

#[sqlx::test(migrations = "../../migrations")]
async fn cost_preview_monthly_split_across_boundaries(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "admin").await;
    let resource_id = create_test_resource_in_dept(&pool, "Split Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;
    let project_id = create_test_project(&pool, "Split Project").await;
    let token = get_auth_token(&app, &email).await;

    let today = Local::now().date_naive();
    let first_next_month = if today.month() == 12 {
        NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1).unwrap()
    };
    let start = first_next_month - chrono::Duration::days(2);
    let end = first_next_month + chrono::Duration::days(3);

    sqlx::query("INSERT INTO holidays (name, date) VALUES ('Preview Holiday', $1)")
        .bind(first_next_month)
        .execute(&pool)
        .await
        .unwrap();

    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, start, end, 50.0, false))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let breakdown = body["monthly_breakdown"].as_array().unwrap();
    assert!(breakdown.len() >= 2, "expected at least two month buckets");

    let total_from_buckets = breakdown
        .iter()
        .map(|bucket| bucket["cost_idr"].as_i64().unwrap())
        .sum::<i64>();
    assert_eq!(total_from_buckets, body["total_cost_idr"].as_i64().unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
async fn cost_preview_budget_impact_null_without_config(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "admin").await;
    let resource_id = create_test_resource_in_dept(&pool, "Budgetless Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;
    let project_id = create_test_project(&pool, "Budgetless Project").await;
    let token = get_auth_token(&app, &email).await;

    let start = Local::now().date_naive() + chrono::Duration::days(7);
    let end = start + chrono::Duration::days(5);

    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, start, end, 75.0, false))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["budget_impact"].is_null());
    assert!(body["warning"].is_null());
    assert_eq!(body["requires_approval"], false);
}

#[sqlx::test(migrations = "../../migrations")]
async fn cost_preview_unauthorized_role_forbidden(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "hr").await;
    let resource_id = create_test_resource_in_dept(&pool, "Forbidden Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;
    let project_id = create_test_project(&pool, "Forbidden Project").await;
    let token = get_auth_token(&app, &email).await;

    let start = Local::now().date_naive() + chrono::Duration::days(7);
    let end = start + chrono::Duration::days(5);

    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, start, end, 50.0, false))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

async fn insert_department_budget(pool: &PgPool, dept_id: Uuid, period: &str, total_budget_idr: i64) {
    sqlx::query(
        "INSERT INTO department_budgets (id, department_id, budget_period, total_budget_idr, created_at, updated_at)
         VALUES ($1, $2, $3, $4, NOW(), NOW())"
    )
    .bind(Uuid::new_v4())
    .bind(dept_id)
    .bind(period)
    .bind(total_budget_idr)
    .execute(pool)
    .await
    .expect("department budget inserted");
}

#[sqlx::test(migrations = "../../migrations")]
async fn cost_preview_budget_health_thresholds(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Budget Health Dept").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;
    let resource_id = create_test_resource_in_dept(&pool, "Budget Health Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;
    let project_id = create_test_project(&pool, "Budget Health Project").await;
    let token = get_auth_token(&app, &email).await;

    // daily_rate = 1,200,000. 5 weekdays at 100% = 6,000,000
    let monday = next_weekday(Local::now().date_naive() + chrono::Duration::days(7), Weekday::Mon);
    let friday = monday + chrono::Duration::days(4);
    let period = format!("{:04}-{:02}", monday.year(), monday.month());

    // Scenario 1: healthy — budget 100M, assignment cost 6M => ~6% utilization
    insert_department_budget(&pool, dept_id, &period, 100_000_000).await;
    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, monday, friday, 100.0, false))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["budget_impact"]["budget_health"], "healthy");
    assert!(body["warning"].is_null());
    assert_eq!(body["requires_approval"], false);

    // Scenario 2: warning — budget 10M, assignment cost 6M => 60% utilization
    sqlx::query("DELETE FROM department_budgets WHERE department_id = $1")
        .bind(dept_id)
        .execute(&pool)
        .await
        .unwrap();
    insert_department_budget(&pool, dept_id, &period, 10_000_000).await;
    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, monday, friday, 100.0, false))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["budget_impact"]["budget_health"], "warning");
    assert_eq!(body["requires_approval"], false);

    // Scenario 3: critical — budget 7M, assignment cost 6M => ~86% utilization
    sqlx::query("DELETE FROM department_budgets WHERE department_id = $1")
        .bind(dept_id)
        .execute(&pool)
        .await
        .unwrap();
    insert_department_budget(&pool, dept_id, &period, 7_000_000).await;
    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, monday, friday, 100.0, false))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["budget_impact"]["budget_health"], "critical");
    assert!(!body["warning"].is_null(), "warning should be present for critical budget");
}

#[sqlx::test(migrations = "../../migrations")]
async fn cost_preview_committed_increases_after_allocation(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Committed Dept").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;
    let resource_id = create_test_resource_in_dept(&pool, "Committed Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;
    let project_id = create_test_project(&pool, "Committed Project").await;
    let project_id_2 = create_test_project(&pool, "Committed Project 2").await;
    let token = get_auth_token(&app, &email).await;

    let monday = next_weekday(Local::now().date_naive() + chrono::Duration::days(7), Weekday::Mon);
    let friday = monday + chrono::Duration::days(4);
    let period = format!("{:04}-{:02}", monday.year(), monday.month());
    insert_department_budget(&pool, dept_id, &period, 100_000_000).await;

    // Query 1: before any allocation — committed should be 0
    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, monday, friday, 50.0, false))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_before: Value = serde_json::from_slice(&bytes).unwrap();
    let committed_before = body_before["budget_impact"]["current_committed_idr"].as_i64().unwrap();
    assert_eq!(committed_before, 0);

    // Create an allocation via POST
    let allocation_payload = serde_json::json!({
        "project_id": project_id.to_string(),
        "resource_id": resource_id.to_string(),
        "start_date": monday.format("%Y-%m-%d").to_string(),
        "end_date": friday.format("%Y-%m-%d").to_string(),
        "allocation_percentage": 50.0,
        "include_weekend": false
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(allocation_payload.to_string()))
        .expect("request");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK, "allocation creation should succeed");

    // Query 2: after allocation — committed should have increased
    // Use project_id_2 so this is a different assignment preview
    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id_2, monday, friday, 50.0, false))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_after: Value = serde_json::from_slice(&bytes).unwrap();
    let committed_after = body_after["budget_impact"]["current_committed_idr"].as_i64().unwrap();
    assert!(committed_after > committed_before, "committed should increase after allocation; before={committed_before}, after={committed_after}");
}

#[sqlx::test(migrations = "../../migrations")]
async fn cost_preview_budget_overrun_block_requires_approval(pool: PgPool) {
    set_test_env();
    // Set BUDGET_OVERRUN_POLICY=block for this test
    std::env::set_var("BUDGET_OVERRUN_POLICY", "block");
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Block Dept").await;
    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    assign_user_to_department(&pool, user_id, dept_id).await;
    let resource_id = create_test_resource_in_dept(&pool, "Block Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, user_id).await;
    let project_id = create_test_project(&pool, "Block Project").await;
    let token = get_auth_token(&app, &email).await;

    let monday = next_weekday(Local::now().date_naive() + chrono::Duration::days(7), Weekday::Mon);
    let friday = monday + chrono::Duration::days(4);
    let period = format!("{:04}-{:02}", monday.year(), monday.month());

    // Budget 7M, assignment 6M at 100% => ~86% => critical
    insert_department_budget(&pool, dept_id, &period, 7_000_000).await;

    let req = Request::builder()
        .method("GET")
        .uri(preview_uri(resource_id, project_id, monday, friday, 100.0, false))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body["budget_impact"]["budget_health"], "critical");
    assert_eq!(body["requires_approval"], true);
    assert!(!body["warning"].is_null(), "warning should be present for critical budget with block policy");

    // Clean up env var
    std::env::set_var("BUDGET_OVERRUN_POLICY", "warn");
}
