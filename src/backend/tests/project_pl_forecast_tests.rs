use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

// ── Helper functions ─────────────────────────────────────────────────────

fn set_test_env() {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );
}

fn test_email() -> String {
    format!("forecast-{}@example.com", Uuid::new_v4())
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

async fn get_auth_token(app: &Router, email: &str) -> String {
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
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("readable body");
    let body: Value = serde_json::from_slice(&body).expect("valid JSON");
    body["token"].as_str().expect("token present").to_string()
}

/// Create a project with explicit start_date and end_date for deterministic forecasting.
async fn create_forecast_test_project(
    pool: &PgPool,
    name: &str,
    pm_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
    budget_hr: i64,
    budget_software: i64,
    budget_hardware: i64,
    budget_overhead: i64,
) -> Uuid {
    let total_budget = budget_hr + budget_software + budget_hardware + budget_overhead;
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, status, start_date, end_date, project_manager_id,
                               total_budget_idr, budget_hr_idr, budget_software_idr,
                               budget_hardware_idr, budget_overhead_idr,
                               target_margin_pct, margin_alert_threshold_pct)
         VALUES ($1, 'Active', $2, $3, $4, $5, $6, $7, $8, $9, 40.00, 5.00)
         RETURNING id",
    )
    .bind(name)
    .bind(start_date)
    .bind(end_date)
    .bind(pm_id)
    .bind(total_budget)
    .bind(budget_hr)
    .bind(budget_software)
    .bind(budget_hardware)
    .bind(budget_overhead)
    .fetch_one(pool)
    .await
    .expect("project created")
}

async fn create_test_department(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("INSERT INTO departments (name) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("department created")
}

async fn create_test_resource(pool: &PgPool, name: &str, dept_id: Uuid) -> Uuid {
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

async fn create_ctc_for_resource(
    pool: &PgPool,
    resource_id: Uuid,
    user_id: Uuid,
    daily_rate: i64,
    effective_date: &str,
) {
    use xynergy_backend::services::ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService};
    use xynergy_backend::services::key_provider::EnvKeyProvider;

    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let encrypted_daily_rate = crypto_svc
        .encrypt_components(&json!({"daily_rate": daily_rate.to_string()}))
        .await
        .expect("daily rate encryption should work");
    let encrypted_components = crypto_svc
        .encrypt_components(&json!({"base_salary": 10000000, "daily_rate": daily_rate.to_string()}))
        .await
        .expect("components encryption should work");

    sqlx::query(
        "INSERT INTO ctc_records (resource_id, components, encrypted_components, encrypted_daily_rate, key_version, encryption_version, encryption_algorithm, encrypted_at, daily_rate, working_days_per_month, effective_date, status, created_by, created_at, updated_by, reason)
         VALUES ($1, '{}'::jsonb, $2, $3, $4, $5, $6, $7, $8, 22, $9::date, 'Active', $10, CURRENT_TIMESTAMP, $10, 'Test CTC')",
    )
    .bind(resource_id)
    .bind(&encrypted_components.ciphertext)
    .bind(&encrypted_daily_rate.ciphertext)
    .bind(&encrypted_daily_rate.key_version)
    .bind(&encrypted_daily_rate.encryption_version)
    .bind(&encrypted_daily_rate.algorithm)
    .bind(encrypted_daily_rate.encrypted_at)
    .bind(sqlx::types::BigDecimal::from(daily_rate))
    .bind(effective_date)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("CTC record created");
}

async fn create_allocation(
    pool: &PgPool,
    resource_id: Uuid,
    project_id: Uuid,
    pct: f64,
    start_date: &str,
    end_date: &str,
    include_weekend: bool,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO allocations (resource_id, project_id, allocation_percentage, start_date, end_date, include_weekend)
         VALUES ($1, $2, $3, $4::date, $5::date, $6)
         RETURNING id",
    )
    .bind(resource_id)
    .bind(project_id)
    .bind(sqlx::types::BigDecimal::try_from(pct).expect("pct should convert"))
    .bind(start_date)
    .bind(end_date)
    .bind(include_weekend)
    .fetch_one(pool)
    .await
    .expect("allocation created")
}

async fn update_allocation_percentage(pool: &PgPool, allocation_id: Uuid, pct: f64) {
    sqlx::query("UPDATE allocations SET allocation_percentage = $1 WHERE id = $2")
        .bind(sqlx::types::BigDecimal::try_from(pct).expect("pct should convert"))
        .bind(allocation_id)
        .execute(pool)
        .await
        .expect("allocation updated");
}

async fn add_expense(
    app: &Router,
    project_id: Uuid,
    token: &str,
    category: &str,
    amount: i64,
    date: &str,
) {
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/expenses", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "category": category,
                "description": format!("Test {} expense", category),
                "amount_idr": amount,
                "expense_date": date,
                "vendor": "Test Vendor"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should create expense");
    assert!(
        resp.status().is_success(),
        "expense creation should succeed"
    );
}

async fn add_revenue(app: &Router, project_id: Uuid, token: &str, month: &str, amount: i64) {
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/revenue", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "revenue_month": month,
                "amount_idr": amount,
                "override_erp": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should create revenue");
    assert!(
        resp.status().is_success(),
        "revenue creation should succeed"
    );
}

async fn fetch_forecast(
    app: &Router,
    project_id: Uuid,
    token: &str,
    year: i32,
    as_of: Option<&str>,
) -> (StatusCode, Value) {
    let uri = match as_of {
        Some(date) => format!(
            "/api/v1/projects/{}/pl/forecast?year={}&as_of={}",
            project_id, year, date
        ),
        None => format!("/api/v1/projects/{}/pl/forecast?year={}", project_id, year),
    };
    let req = Request::builder()
        .method("GET")
        .uri(&uri)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("readable body");
    let body: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    (status, body)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn pm_can_fetch_forecast_for_owned_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_forecast_test_project(
        &pool,
        "Forecast PM Test",
        pm_id,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        500_000_000,
        100_000_000,
        50_000_000,
        50_000_000,
    )
    .await;
    let token = get_auth_token(&app, &pm_email).await;

    let (status, body) = fetch_forecast(&app, project_id, &token, 2026, Some("2026-06-30")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["project_id"].as_str().unwrap(), project_id.to_string());
    assert_eq!(body["year"].as_i64().unwrap(), 2026);
    assert_eq!(body["as_of_date"].as_str().unwrap(), "2026-06-30");
    assert!(body["categories"].as_array().unwrap().len() == 4);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_fetch_forecast_for_any_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_forecast_test_project(
        &pool,
        "Admin Forecast Test",
        pm_id,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        400_000_000,
        100_000_000,
        50_000_000,
        50_000_000,
    )
    .await;

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    let (status, _) =
        fetch_forecast(&app, project_id, &admin_token, 2026, Some("2026-06-30")).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_denied_forecast_on_non_owned_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_a_email = test_email();
    let pm_a_id = create_test_user_with_role(&pool, &pm_a_email, "project_manager").await;
    let project_id = create_forecast_test_project(
        &pool,
        "PM A Forecast Project",
        pm_a_id,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        400_000_000,
        100_000_000,
        50_000_000,
        50_000_000,
    )
    .await;

    let pm_b_email = test_email();
    let _pm_b_id = create_test_user_with_role(&pool, &pm_b_email, "project_manager").await;
    let pm_b_token = get_auth_token(&app, &pm_b_email).await;

    let (status, _) = fetch_forecast(&app, project_id, &pm_b_token, 2026, Some("2026-06-30")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Verify audit log
    let audit_entry = sqlx::query!(
        "SELECT action, entity_type FROM audit_logs
         WHERE action = 'ACCESS_DENIED' AND entity_type = 'project_pl_forecast'
         AND entity_id = $1
         ORDER BY created_at DESC LIMIT 1",
        project_id
    )
    .fetch_optional(&pool)
    .await
    .expect("audit query should succeed");

    assert!(
        audit_entry.is_some(),
        "access denied audit should be logged"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn forecast_returns_404_for_non_existent_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    let fake_id = Uuid::new_v4();
    let (status, _) = fetch_forecast(&app, fake_id, &admin_token, 2026, Some("2026-06-30")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn forecast_burn_rate_math_correctness(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;

    // Project: 2026-01-01 to 2026-12-31 (365 days)
    // Budget: HR=500M, Software=100M, Hardware=50M, Overhead=50M (total=700M)
    let project_id = create_forecast_test_project(
        &pool,
        "Burn Rate Test",
        pm_id,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        500_000_000,
        100_000_000,
        50_000_000,
        50_000_000,
    )
    .await;
    let token = get_auth_token(&app, &pm_email).await;

    // Add 60M in software expenses in Jan+Feb
    add_expense(
        &app,
        project_id,
        &token,
        "software",
        30_000_000,
        "2026-01-15",
    )
    .await;
    add_expense(
        &app,
        project_id,
        &token,
        "software",
        30_000_000,
        "2026-02-15",
    )
    .await;
    // Add 10M overhead expense
    add_expense(
        &app,
        project_id,
        &token,
        "overhead",
        10_000_000,
        "2026-01-20",
    )
    .await;

    // Add 200M revenue in Jan
    add_revenue(&app, project_id, &token, "2026-01", 200_000_000).await;

    // Forecast as of 2026-03-31
    // elapsed_days = (2026-03-31 - 2026-01-01) + 1 = 90 days
    // total_project_days = (2026-12-31 - 2026-01-01) + 1 = 365 days
    // current_spend = 60M + 10M = 70M (no resource costs in this test)
    // burn_rate = 70M / 90 = ~777777.78/day
    // projected_total = round(777777.78 * 365) = ~283,888,889
    // remaining = 283,888,889 - 70,000,000 = 213,888,889
    // forecast_margin = (200M - 283,888,889) / 200M * 100 = -41.94%
    // variance = -41.94 - 40.0 = -81.94%
    let (status, body) = fetch_forecast(&app, project_id, &token, 2026, Some("2026-03-31")).await;
    assert_eq!(status, StatusCode::OK);

    // Verify timeline
    assert_eq!(body["elapsed_days"].as_i64().unwrap(), 90);
    assert_eq!(body["total_project_days"].as_i64().unwrap(), 365);

    // Verify current spend
    assert_eq!(body["current_spend_idr"].as_i64().unwrap(), 70_000_000);

    // Verify burn rate
    let burn_rate = body["burn_rate_idr_per_day"].as_f64().unwrap();
    let expected_burn_rate = 70_000_000.0 / 90.0;
    assert!(
        (burn_rate - expected_burn_rate).abs() < 1.0,
        "burn rate should be ~{:.2}, got {:.2}",
        expected_burn_rate,
        burn_rate
    );

    // Verify projected total cost
    let projected = body["projected_total_cost_idr"].as_i64().unwrap();
    let expected_projected = (expected_burn_rate * 365.0).round() as i64;
    assert_eq!(projected, expected_projected);

    // Verify remaining cost
    let remaining = body["remaining_cost_projection_idr"].as_i64().unwrap();
    assert_eq!(remaining, projected - 70_000_000);
}

#[sqlx::test(migrations = "../../migrations")]
async fn forecast_margin_and_variance_formulas(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;

    let project_id = create_forecast_test_project(
        &pool,
        "Margin Variance Test",
        pm_id,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        200_000_000,
        50_000_000,
        25_000_000,
        25_000_000,
    )
    .await;
    let token = get_auth_token(&app, &pm_email).await;

    // Add 30M expenses, 100M revenue
    add_expense(&app, project_id, &token, "hr", 30_000_000, "2026-01-15").await;
    add_revenue(&app, project_id, &token, "2026-01", 100_000_000).await;

    // as_of = 2026-03-31, elapsed = 90 days, total = 181 days
    // spend = 30M, burn = 30M/90 = 333333.33/day
    // projected = round(333333.33 * 181) = 60,333,333
    // forecast_margin = (100M - 60,333,333) / 100M * 100 = 39.67%
    // target = 40%, variance = 39.67 - 40.0 = -0.33%
    let (status, body) = fetch_forecast(&app, project_id, &token, 2026, Some("2026-03-31")).await;
    assert_eq!(status, StatusCode::OK);

    let forecast_margin = body["forecast_margin_pct"].as_f64().unwrap();
    let target_margin = body["target_margin_pct"].as_f64().unwrap();
    let variance = body["variance_from_target_pct"].as_f64().unwrap();

    assert_eq!(target_margin, 40.0);
    // variance should equal forecast_margin - target_margin
    assert!(
        (variance - (forecast_margin - target_margin)).abs() < 0.01,
        "variance should be forecast_margin - target, got: {variance} vs {forecast_margin} - {target_margin}"
    );
    // Forecast margin should be positive since revenue > projected cost
    assert!(
        forecast_margin > 0.0,
        "forecast margin should be positive, got: {forecast_margin}"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn forecast_overrun_categories_detected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;

    // Budget: software=10M (small), overhead=10M, hr=200M, hardware=10M
    let project_id = create_forecast_test_project(
        &pool,
        "Overrun Test",
        pm_id,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        200_000_000,
        10_000_000,
        10_000_000,
        10_000_000,
    )
    .await;
    let token = get_auth_token(&app, &pm_email).await;

    // Spend 8M on software in first month → projected ~97M >> budget 10M → overrun
    add_expense(
        &app,
        project_id,
        &token,
        "software",
        8_000_000,
        "2026-01-15",
    )
    .await;
    // Small overhead expense
    add_expense(
        &app,
        project_id,
        &token,
        "overhead",
        1_000_000,
        "2026-01-10",
    )
    .await;

    let (status, body) = fetch_forecast(&app, project_id, &token, 2026, Some("2026-01-31")).await;
    assert_eq!(status, StatusCode::OK);

    let categories = body["categories"].as_array().unwrap();
    assert_eq!(categories.len(), 4);

    // Find software category — should have overrun
    let software = categories
        .iter()
        .find(|c| c["category"].as_str().unwrap() == "software")
        .expect("software category should exist");
    assert!(
        software["overrun_idr"].as_i64().unwrap() > 0,
        "software should be overrunning with 8M spend in first month against 10M budget"
    );
    assert_eq!(software["budget_idr"].as_i64().unwrap(), 10_000_000);

    // HR category should have 0 overrun (no HR expenses, no resource allocations)
    let hr = categories
        .iter()
        .find(|c| c["category"].as_str().unwrap() == "hr")
        .expect("hr category should exist");
    assert_eq!(hr["overrun_idr"].as_i64().unwrap(), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn forecast_resource_drivers_sorted_descending(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_forecast_test_project(
        &pool,
        "Resource Driver Test",
        pm_id,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        500_000_000,
        100_000_000,
        50_000_000,
        50_000_000,
    )
    .await;
    let token = get_auth_token(&app, &pm_email).await;

    let dept_id = create_test_department(&pool, "Engineering").await;
    let lead = create_test_resource(&pool, "Lead Engineer", dept_id).await;
    let senior = create_test_resource(&pool, "Senior Engineer", dept_id).await;
    let junior = create_test_resource(&pool, "Junior Engineer", dept_id).await;

    create_ctc_for_resource(&pool, lead, pm_id, 2_000_000, "2026-01-01").await;
    create_ctc_for_resource(&pool, senior, pm_id, 1_500_000, "2026-01-01").await;
    create_ctc_for_resource(&pool, junior, pm_id, 800_000, "2026-01-01").await;

    create_allocation(
        &pool,
        lead,
        project_id,
        1.0,
        "2026-01-01",
        "2026-01-31",
        false,
    )
    .await;
    create_allocation(
        &pool,
        senior,
        project_id,
        1.0,
        "2026-01-01",
        "2026-01-31",
        false,
    )
    .await;
    create_allocation(
        &pool,
        junior,
        project_id,
        1.0,
        "2026-01-01",
        "2026-01-31",
        false,
    )
    .await;

    let (status, body) = fetch_forecast(&app, project_id, &token, 2026, Some("2026-06-30")).await;
    assert_eq!(status, StatusCode::OK);

    let drivers = body["resource_drivers"].as_array().unwrap();
    assert_eq!(drivers.len(), 3);

    let first = drivers[0]["total_cost_idr"].as_i64().unwrap();
    let second = drivers[1]["total_cost_idr"].as_i64().unwrap();
    let third = drivers[2]["total_cost_idr"].as_i64().unwrap();
    assert!(first >= second, "first driver should be >= second");
    assert!(second >= third, "second driver should be >= third");
}

#[sqlx::test(migrations = "../../migrations")]
async fn forecast_updates_after_expense_change(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;

    let project_id = create_forecast_test_project(
        &pool,
        "Update Forecast Test",
        pm_id,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        500_000_000,
        100_000_000,
        50_000_000,
        50_000_000,
    )
    .await;
    let token = get_auth_token(&app, &pm_email).await;

    add_revenue(&app, project_id, &token, "2026-01", 500_000_000).await;

    // First forecast with 10M spend
    add_expense(
        &app,
        project_id,
        &token,
        "software",
        10_000_000,
        "2026-01-15",
    )
    .await;
    let (_, body1) = fetch_forecast(&app, project_id, &token, 2026, Some("2026-03-31")).await;
    let projected1 = body1["projected_total_cost_idr"].as_i64().unwrap();

    // Add more expenses → projected cost should increase
    add_expense(
        &app,
        project_id,
        &token,
        "software",
        40_000_000,
        "2026-02-15",
    )
    .await;
    let (_, body2) = fetch_forecast(&app, project_id, &token, 2026, Some("2026-03-31")).await;
    let projected2 = body2["projected_total_cost_idr"].as_i64().unwrap();

    assert!(
        projected2 > projected1,
        "projected cost should increase after adding expenses: {} > {} (AC#4)",
        projected2,
        projected1
    );

    // Verify margin decreased
    let margin1 = body1["forecast_margin_pct"].as_f64().unwrap();
    let margin2 = body2["forecast_margin_pct"].as_f64().unwrap();
    assert!(
        margin2 < margin1,
        "forecast margin should decrease after adding expenses: {} < {}",
        margin2,
        margin1
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn forecast_excludes_expenses_after_as_of(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;

    let project_id = create_forecast_test_project(
        &pool,
        "As Of Expense Window Test",
        pm_id,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        500_000_000,
        100_000_000,
        50_000_000,
        50_000_000,
    )
    .await;
    let token = get_auth_token(&app, &pm_email).await;

    add_expense(
        &app,
        project_id,
        &token,
        "software",
        10_000_000,
        "2026-01-15",
    )
    .await;
    add_expense(
        &app,
        project_id,
        &token,
        "software",
        50_000_000,
        "2026-11-15",
    )
    .await;

    let (status_march, body_march) =
        fetch_forecast(&app, project_id, &token, 2026, Some("2026-03-31")).await;
    assert_eq!(status_march, StatusCode::OK);
    assert_eq!(
        body_march["current_spend_idr"].as_i64().unwrap(),
        10_000_000
    );

    let (status_dec, body_dec) =
        fetch_forecast(&app, project_id, &token, 2026, Some("2026-12-31")).await;
    assert_eq!(status_dec, StatusCode::OK);
    assert_eq!(body_dec["current_spend_idr"].as_i64().unwrap(), 60_000_000);
}

#[sqlx::test(migrations = "../../migrations")]
async fn forecast_updates_after_allocation_change(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;

    let project_id = create_forecast_test_project(
        &pool,
        "Allocation Update Forecast Test",
        pm_id,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        500_000_000,
        100_000_000,
        50_000_000,
        50_000_000,
    )
    .await;
    let token = get_auth_token(&app, &pm_email).await;

    add_revenue(&app, project_id, &token, "2026-01", 500_000_000).await;

    let dept_id = create_test_department(&pool, "Engineering").await;
    let resource_id = create_test_resource(&pool, "Allocation Driver", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, pm_id, 1_200_000, "2026-01-01").await;

    let allocation_id = create_allocation(
        &pool,
        resource_id,
        project_id,
        0.5,
        "2026-01-01",
        "2026-03-31",
        false,
    )
    .await;

    let (_, body_before) = fetch_forecast(&app, project_id, &token, 2026, Some("2026-03-31")).await;
    let spend_before = body_before["current_spend_idr"].as_i64().unwrap();
    let projected_before = body_before["projected_total_cost_idr"].as_i64().unwrap();

    update_allocation_percentage(&pool, allocation_id, 1.0).await;

    let (_, body_after) = fetch_forecast(&app, project_id, &token, 2026, Some("2026-03-31")).await;
    let spend_after = body_after["current_spend_idr"].as_i64().unwrap();
    let projected_after = body_after["projected_total_cost_idr"].as_i64().unwrap();

    assert!(
        spend_after > spend_before,
        "current spend should increase after allocation increase: {} > {}",
        spend_after,
        spend_before
    );
    assert!(
        projected_after > projected_before,
        "projected total should increase after allocation increase: {} > {}",
        projected_after,
        projected_before
    );
}
