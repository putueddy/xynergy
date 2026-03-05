use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use chrono::{Datelike, Utc};
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
    format!("projpl-{}@example.com", Uuid::new_v4())
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
    body["token"]
        .as_str()
        .expect("token present")
        .to_string()
}

async fn create_test_project_with_pm(pool: &PgPool, name: &str, pm_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, status, start_date, end_date, project_manager_id, target_margin_pct, margin_alert_threshold_pct)
         VALUES ($1, 'Active', CURRENT_DATE, CURRENT_DATE + INTERVAL '90 days', $2, 40.00, 5.00)
         RETURNING id",
    )
    .bind(name)
    .bind(pm_id)
    .fetch_one(pool)
    .await
    .expect("project created")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn pm_can_get_pl_dashboard_for_own_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "P&L Test Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let current_year = Utc::now().year();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/pl?year={}", project_id, current_year))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.expect("readable body");
    let body: Value = serde_json::from_slice(&body).expect("valid JSON");
    
    assert_eq!(body["project_id"].as_str().unwrap(), project_id.to_string());
    assert_eq!(body["year"].as_i64().unwrap(), current_year as i64);
    assert!(body["months"].as_array().unwrap().len() == 12);
    assert_eq!(body["target_margin_pct"].as_f64().unwrap(), 40.0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_get_pl_dashboard_for_any_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Admin P&L Project", pm_id).await;

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    let current_year = Utc::now().year();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/pl?year={}", project_id, current_year))
        .header("Authorization", format!("Bearer {}", admin_token))
        .body(Body::empty())
        .expect("request should be built");
    
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_denied_pl_dashboard_on_non_owned_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_a_email = test_email();
    let pm_a_id = create_test_user_with_role(&pool, &pm_a_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "PM A Project", pm_a_id).await;

    let pm_b_email = test_email();
    let _pm_b_id = create_test_user_with_role(&pool, &pm_b_email, "project_manager").await;
    let pm_b_token = get_auth_token(&app, &pm_b_email).await;

    let current_year = Utc::now().year();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/pl?year={}", project_id, current_year))
        .header("Authorization", format!("Bearer {}", pm_b_token))
        .body(Body::empty())
        .expect("request should be built");
    
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Verify audit log
    let audit_entry = sqlx::query!(
        "SELECT action, entity_type FROM audit_logs 
         WHERE action = 'ACCESS_DENIED' AND entity_type = 'project_pl_dashboard' 
         AND entity_id = $1 
         ORDER BY created_at DESC LIMIT 1",
        project_id
    )
    .fetch_optional(&pool)
    .await
    .expect("audit query should succeed");

    assert!(audit_entry.is_some(), "access denied audit should be logged");
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_pm_non_admin_denied_pl_dashboard(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "HR Test Project", pm_id).await;

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let current_year = Utc::now().year();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/pl?year={}", project_id, current_year))
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .expect("request should be built");
    
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn pl_dashboard_shows_zero_values_for_empty_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Empty Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let current_year = Utc::now().year();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/pl?year={}", project_id, current_year))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.expect("readable body");
    let body: Value = serde_json::from_slice(&body).expect("valid JSON");
    
    // All months should have zero values
    assert_eq!(body["total_revenue_idr"].as_i64().unwrap(), 0);
    assert_eq!(body["total_cost_idr"].as_i64().unwrap(), 0);
    assert_eq!(body["gross_profit_idr"].as_i64().unwrap(), 0);
    assert_eq!(body["margin_pct"].as_f64().unwrap(), 0.0);
    
    // No margin alert for zero revenue
    assert!(body["margin_alert"].is_null());
}

#[sqlx::test(migrations = "../../migrations")]
async fn pl_dashboard_with_revenue_shows_positive_profit(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // Create revenue entry
    let current_year = Utc::now().year();
    let revenue_month = format!("{}-01", current_year);
    let create_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/revenue", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "revenue_month": revenue_month,
                "amount_idr": 100_000_000,
                "override_erp": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    
    let resp = app.clone().oneshot(create_req).await.expect("should create revenue");
    assert_eq!(resp.status(), StatusCode::OK);

    // Get P&L dashboard
    let pl_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/pl?year={}", project_id, current_year))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    
    let resp = app.clone().oneshot(pl_req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.expect("readable body");
    let body: Value = serde_json::from_slice(&body).expect("valid JSON");
    
    assert_eq!(body["total_revenue_idr"].as_i64().unwrap(), 100_000_000);
    // First month should have revenue
    let jan = &body["months"][0];
    assert_eq!(jan["revenue_idr"].as_i64().unwrap(), 100_000_000);
}

// ── New tests: margin alert, expense impact, settings validation, 404 ────────

#[sqlx::test(migrations = "../../migrations")]
async fn pl_dashboard_triggers_margin_alert_when_below_target(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Alert Test Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let current_year = Utc::now().year();

    // Add revenue: 100M IDR in January
    let revenue_month = format!("{}-01", current_year);
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/revenue", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "revenue_month": revenue_month,
                "amount_idr": 100_000_000i64,
                "override_erp": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should create revenue");
    assert_eq!(resp.status(), StatusCode::OK);

    // Add expense: 70M IDR in January
    // margin = (100M - 70M) / 100M = 30%
    // target=40%, threshold=5%, deviation = 40 - 30 = 10% > 5% → alert triggers
    let expense_date = format!("{}-01-15", current_year);
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/expenses", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "category": "software",
                "description": "Large software license",
                "amount_idr": 70_000_000i64,
                "expense_date": expense_date,
                "vendor": "Test Vendor"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should create expense");
    assert!(resp.status().is_success(), "expense creation should succeed");

    // Get P&L dashboard
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/projects/{}/pl?year={}",
            project_id, current_year
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("readable body");
    let body: Value = serde_json::from_slice(&body).expect("valid JSON");

    // Verify margin alert is triggered
    assert!(
        body["margin_alert"].is_string(),
        "margin alert should be present when margin is below target"
    );
    let alert = body["margin_alert"].as_str().unwrap();
    assert!(
        alert.contains("Margin below target"),
        "alert should say 'Margin below target', got: {alert}"
    );
    assert!(
        alert.contains("40"),
        "alert should reference 40% target, got: {alert}"
    );

    // Verify the calculated values
    assert_eq!(body["total_revenue_idr"].as_i64().unwrap(), 100_000_000);
    assert_eq!(body["total_cost_idr"].as_i64().unwrap(), 70_000_000);
    assert_eq!(body["gross_profit_idr"].as_i64().unwrap(), 30_000_000);
    let margin = body["margin_pct"].as_f64().unwrap();
    assert!(
        (margin - 30.0).abs() < 0.01,
        "margin should be ~30%, got: {margin}"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn pl_dashboard_includes_expense_impact(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Expense Impact Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let current_year = Utc::now().year();

    // Add expense in March: 25M IDR
    let expense_date = format!("{}-03-10", current_year);
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/expenses", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "category": "hardware",
                "description": "Server purchase",
                "amount_idr": 25_000_000i64,
                "expense_date": expense_date,
                "vendor": "Hardware Vendor"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should create expense");
    assert!(resp.status().is_success(), "expense creation should succeed");

    // Get P&L dashboard
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/projects/{}/pl?year={}",
            project_id, current_year
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("readable body");
    let body: Value = serde_json::from_slice(&body).expect("valid JSON");

    // March (index 2) should show the expense as non-resource cost
    let march = &body["months"][2];
    assert_eq!(march["month"].as_u64().unwrap(), 3);
    assert_eq!(march["non_resource_cost_idr"].as_i64().unwrap(), 25_000_000);
    assert_eq!(march["total_cost_idr"].as_i64().unwrap(), 25_000_000);
    assert_eq!(march["gross_profit_idr"].as_i64().unwrap(), -25_000_000);

    // Summary should reflect the expense
    assert_eq!(body["total_cost_idr"].as_i64().unwrap(), 25_000_000);
    assert_eq!(body["gross_profit_idr"].as_i64().unwrap(), -25_000_000);

    // No margin alert when revenue is zero (division by zero guard)
    assert!(body["margin_alert"].is_null());
}

#[sqlx::test(migrations = "../../migrations")]
async fn pl_settings_validation_rejects_invalid_percentages(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id =
        create_test_project_with_pm(&pool, "Settings Validation Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // Test: target_margin_pct > 100 → BAD_REQUEST
    let req = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/v1/projects/{}/pl/settings",
            project_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "target_margin_pct": 150.0 }).to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Test: target_margin_pct < 0 → BAD_REQUEST
    let req = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/v1/projects/{}/pl/settings",
            project_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "target_margin_pct": -10.0 }).to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Test: valid values → success
    let req = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/v1/projects/{}/pl/settings",
            project_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "target_margin_pct": 35.0,
                "margin_alert_threshold_pct": 3.0
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("readable body");
    let body: Value = serde_json::from_slice(&body).expect("valid JSON");
    assert_eq!(body["target_margin_pct"].as_f64().unwrap(), 35.0);
    assert_eq!(body["margin_alert_threshold_pct"].as_f64().unwrap(), 3.0);

    // Verify settings persisted by checking P&L dashboard
    let current_year = Utc::now().year();
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/projects/{}/pl?year={}",
            project_id, current_year
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("readable body");
    let body: Value = serde_json::from_slice(&body).expect("valid JSON");
    assert_eq!(body["target_margin_pct"].as_f64().unwrap(), 35.0);
    assert_eq!(body["margin_alert_threshold_pct"].as_f64().unwrap(), 3.0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn pl_dashboard_404_for_nonexistent_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    let fake_id = Uuid::new_v4();
    let current_year = Utc::now().year();
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/projects/{}/pl?year={}",
            fake_id, current_year
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("should return response");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
