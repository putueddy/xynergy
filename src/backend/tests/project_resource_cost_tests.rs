use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("projrcost-{}@example.com", Uuid::new_v4())
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

async fn create_test_project_with_pm(pool: &PgPool, name: &str, pm_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, status, start_date, end_date, project_manager_id)
         VALUES ($1, 'Active', '2026-03-01', '2026-05-31', $2)
         RETURNING id",
    )
    .bind(name)
    .bind(pm_id)
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

async fn create_ctc_for_resource(pool: &PgPool, resource_id: Uuid, user_id: Uuid, daily_rate: i64) {
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
         VALUES ($1, '{}'::jsonb, $2, $3, $4, $5, $6, $7, $8, 22, '2026-01-01', 'Active', $9, CURRENT_TIMESTAMP, $9, 'Test CTC')",
    )
    .bind(resource_id)
    .bind(&encrypted_components.ciphertext)
    .bind(&encrypted_daily_rate.ciphertext)
    .bind(&encrypted_daily_rate.key_version)
    .bind(&encrypted_daily_rate.encryption_version)
    .bind(&encrypted_daily_rate.algorithm)
    .bind(encrypted_daily_rate.encrypted_at)
    .bind(sqlx::types::BigDecimal::try_from(daily_rate as f64).unwrap())
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
) {
    sqlx::query(
        "INSERT INTO allocations (resource_id, project_id, allocation_percentage, start_date, end_date, include_weekend)
         VALUES ($1, $2, $3, $4::date, $5::date, $6)",
    )
    .bind(resource_id)
    .bind(project_id)
    .bind(sqlx::types::BigDecimal::try_from(pct).expect("pct should convert"))
    .bind(start_date)
    .bind(end_date)
    .bind(include_weekend)
    .execute(pool)
    .await
    .expect("allocation created");
}

async fn create_ctc_revision(
    pool: &PgPool,
    resource_id: Uuid,
    user_id: Uuid,
    revision_number: i32,
    effective_date: &str,
    daily_rate: i64,
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
        "INSERT INTO ctc_revisions (
            resource_id, revision_number,
            key_version, encryption_version, encryption_algorithm, encrypted_at,
            encrypted_components, encrypted_daily_rate,
            effective_date_policy, effective_date, working_days_per_month, status,
            changed_by, reason
         ) VALUES (
            $1, $2,
            $3, $4, $5, $6,
            $7, $8,
            'pro_rata', $9::date, 22, 'Active',
            $10, 'Test revision'
         )",
    )
    .bind(resource_id)
    .bind(revision_number)
    .bind(&encrypted_daily_rate.key_version)
    .bind(&encrypted_daily_rate.encryption_version)
    .bind(&encrypted_daily_rate.algorithm)
    .bind(encrypted_daily_rate.encrypted_at)
    .bind(&encrypted_components.ciphertext)
    .bind(&encrypted_daily_rate.ciphertext)
    .bind(effective_date)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("ctc revision created");
}

/// Helper: GET resource costs and return (status, body)
async fn get_resource_costs(
    app: &axum::Router,
    token: &str,
    project_id: Uuid,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/resource-costs", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("readable response body");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON response");
    (status, body)
}

// ── Test 1: PM can view resource costs on own project (empty allocations) ───

#[sqlx::test(migrations = "../../migrations")]
async fn pm_can_view_resource_costs_empty_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Empty Cost Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let (status, body) = get_resource_costs(&app, &token, project_id).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["project_id"].as_str().unwrap(),
        project_id.to_string()
    );
    assert_eq!(body["total_resource_cost_idr"].as_i64().unwrap(), 0);
    assert!(body["employees"].as_array().unwrap().is_empty());
    assert!(body["monthly_breakdown"].as_array().unwrap().is_empty());
}

// ── Test 2: Resource cost with 100% allocation ──────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn resource_cost_100pct_allocation(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Full Alloc Project", pm_id).await;
    let dept_id = create_test_department(&pool, "Dept-FullAlloc").await;
    let resource_id = create_test_resource(&pool, "Dev Full", dept_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // Daily rate: 1,000,000 IDR
    create_ctc_for_resource(&pool, resource_id, pm_id, 1_000_000).await;

    // Allocate: 5 working days (Mon-Fri), 100%
    // 2026-03-02 (Mon) to 2026-03-06 (Fri) = 5 working days
    create_allocation(&pool, resource_id, project_id, 100.0, "2026-03-02", "2026-03-06", false).await;

    let (status, body) = get_resource_costs(&app, &token, project_id).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_resource_cost_idr"].as_i64().unwrap(), 5_000_000);

    let employees = body["employees"].as_array().unwrap();
    assert_eq!(employees.len(), 1);
    assert_eq!(employees[0]["resource_name"].as_str().unwrap(), "Dev Full");
    assert_eq!(employees[0]["daily_rate_idr"].as_i64().unwrap(), 1_000_000);
    assert_eq!(employees[0]["days_allocated"].as_i64().unwrap(), 5);
    assert_eq!(employees[0]["total_cost_idr"].as_i64().unwrap(), 5_000_000);
    assert_eq!(employees[0]["missing_rate"].as_bool().unwrap(), false);
    assert_eq!(employees[0]["has_rate_change"].as_bool().unwrap(), false);
}

// ── Test 3: Resource cost with <100% allocation ─────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn resource_cost_partial_allocation(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Partial Alloc Project", pm_id).await;
    let dept_id = create_test_department(&pool, "Dept-PartialAlloc").await;
    let resource_id = create_test_resource(&pool, "Dev Partial", dept_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // Daily rate: 1,200,000 IDR
    create_ctc_for_resource(&pool, resource_id, pm_id, 1_200_000).await;

    // 50% allocation over 5 working days
    // Expected: 1,200,000 * 5 * 50% = 3,000,000
    create_allocation(&pool, resource_id, project_id, 50.0, "2026-03-02", "2026-03-06", false).await;

    let (status, body) = get_resource_costs(&app, &token, project_id).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_resource_cost_idr"].as_i64().unwrap(), 3_000_000);

    let employees = body["employees"].as_array().unwrap();
    assert_eq!(employees[0]["total_cost_idr"].as_i64().unwrap(), 3_000_000);
}

// ── Test 4: Cross-month allocation prorating ────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn resource_cost_cross_month_prorating(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Cross Month Project", pm_id).await;
    let dept_id = create_test_department(&pool, "Dept-CrossMonth").await;
    let resource_id = create_test_resource(&pool, "Dev CrossMonth", dept_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    create_ctc_for_resource(&pool, resource_id, pm_id, 1_000_000).await;

    // Spans Feb 27 (Fri) to Mar 3 (Tue) at 50% allocation
    // Feb: 27th (Fri) = 1 working day
    // Mar: 2nd (Mon), 3rd (Tue) = 2 working days
    // Total: 3 working days => 1,000,000 * 3 * 50% = 1,500,000
    create_allocation(&pool, resource_id, project_id, 50.0, "2026-02-27", "2026-03-03", false).await;

    let (status, body) = get_resource_costs(&app, &token, project_id).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_resource_cost_idr"].as_i64().unwrap(), 1_500_000);

    let monthly = body["monthly_breakdown"].as_array().unwrap();
    assert_eq!(monthly.len(), 2, "should have 2 monthly buckets");

    // Find Feb and Mar entries
    let feb = monthly.iter().find(|m| m["month"].as_str().unwrap() == "2026-02").expect("Feb bucket");
    let mar = monthly.iter().find(|m| m["month"].as_str().unwrap() == "2026-03").expect("Mar bucket");

    assert_eq!(feb["working_days"].as_i64().unwrap(), 1);
    assert_eq!(feb["cost_idr"].as_i64().unwrap(), 500_000);
    assert_eq!(mar["working_days"].as_i64().unwrap(), 2);
    assert_eq!(mar["cost_idr"].as_i64().unwrap(), 1_000_000);
}

// ── Test 5: include_weekend allocations count weekend days ──────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn resource_cost_include_weekend(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Weekend Project", pm_id).await;
    let dept_id = create_test_department(&pool, "Dept-Weekend").await;
    let resource_id = create_test_resource(&pool, "Dev Weekend", dept_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    create_ctc_for_resource(&pool, resource_id, pm_id, 1_000_000).await;

    // Mon Mar 2 to Sun Mar 8 = 7 calendar days, all working with include_weekend=true
    create_allocation(&pool, resource_id, project_id, 100.0, "2026-03-02", "2026-03-08", true).await;

    let (status, body) = get_resource_costs(&app, &token, project_id).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_resource_cost_idr"].as_i64().unwrap(), 7_000_000);

    let employees = body["employees"].as_array().unwrap();
    assert_eq!(employees[0]["days_allocated"].as_i64().unwrap(), 7);
}

// ── Test 6: Missing CTC returns missing_rate=true, cost=0 ──────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn resource_without_ctc_returns_missing_rate(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Missing CTC Project", pm_id).await;
    let dept_id = create_test_department(&pool, "Dept-MissingCTC").await;
    let resource_id = create_test_resource(&pool, "Dev NoCTC", dept_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // No CTC record created — resource has no rate data
    create_allocation(&pool, resource_id, project_id, 100.0, "2026-03-02", "2026-03-06", false).await;

    let (status, body) = get_resource_costs(&app, &token, project_id).await;

    assert_eq!(status, StatusCode::OK);
    // Total cost should be 0 because rate is unavailable
    assert_eq!(body["total_resource_cost_idr"].as_i64().unwrap(), 0);

    let employees = body["employees"].as_array().unwrap();
    assert_eq!(employees.len(), 1);
    assert_eq!(employees[0]["missing_rate"].as_bool().unwrap(), true);
    assert!(employees[0]["daily_rate_idr"].is_null());
    assert_eq!(employees[0]["total_cost_idr"].as_i64().unwrap(), 0);
    // Still counts working days even without rate
    assert!(employees[0]["days_allocated"].as_i64().unwrap() > 0);
}

// ── Test 7: PM denied on non-owned project ──────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn pm_denied_resource_costs_on_non_owned_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_a_email = test_email();
    let pm_b_email = test_email();
    let pm_a_id = create_test_user_with_role(&pool, &pm_a_email, "project_manager").await;
    let _pm_b_id = create_test_user_with_role(&pool, &pm_b_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "PM A Project", pm_a_id).await;
    let pm_b_token = get_auth_token(&app, &pm_b_email).await;

    let (status, body) = get_resource_costs(&app, &pm_b_token, project_id).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        body["error"]["code"].as_str().unwrap(),
        "FORBIDDEN_ERROR"
    );
}

// ── Test 8: Admin can view resource costs on any project ────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_view_resource_costs_on_any_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let admin_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let project_id = create_test_project_with_pm(&pool, "Admin View Project", pm_id).await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    let (status, body) = get_resource_costs(&app, &admin_token, project_id).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["project_id"].as_str().unwrap(),
        project_id.to_string()
    );
}

// ── Test 9: Non-PM/non-admin role denied ────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn non_pm_non_admin_denied_resource_costs(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let hr_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let project_id = create_test_project_with_pm(&pool, "HR Denied Project", pm_id).await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let (status, body) = get_resource_costs(&app, &hr_token, project_id).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        body["error"]["code"].as_str().unwrap(),
        "FORBIDDEN_ERROR"
    );
}

// ── Test 10: spent_to_date_idr = expense sum + resource cost sum ────────────

#[sqlx::test(migrations = "../../migrations")]
async fn budget_spent_includes_resource_costs(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Spent Total Project", pm_id).await;
    let dept_id = create_test_department(&pool, "Dept-SpentTotal").await;
    let resource_id = create_test_resource(&pool, "Dev Spent", dept_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // Set budget
    let budget_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 50_000_000_i64,
                "budget_hr_idr": 20_000_000_i64,
                "budget_software_idr": 15_000_000_i64,
                "budget_hardware_idr": 10_000_000_i64,
                "budget_overhead_idr": 5_000_000_i64,
            })
            .to_string(),
        ))
        .expect("request should be built");
    let budget_resp = app.clone().oneshot(budget_req).await.expect("budget set");
    assert_eq!(budget_resp.status(), StatusCode::OK);

    // Create a manual expense: 500,000
    let expense_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/expenses", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "category": "software",
                "description": "IDE License",
                "amount_idr": 500_000_i64,
                "expense_date": "2026-03-01",
                "vendor": null,
            })
            .to_string(),
        ))
        .expect("expense request should be built");
    let expense_resp = app.clone().oneshot(expense_req).await.expect("expense created");
    assert_eq!(expense_resp.status(), StatusCode::OK);

    // Create allocation with CTC
    create_ctc_for_resource(&pool, resource_id, pm_id, 1_000_000).await;
    // 5 working days at 100% = 5,000,000 resource cost
    create_allocation(&pool, resource_id, project_id, 100.0, "2026-03-02", "2026-03-06", false).await;

    // Fetch budget — spent_to_date_idr should be expense (500,000) + resource cost (5,000,000) = 5,500,000
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("get budget request should be built");
    let get_resp = app.clone().oneshot(get_req).await.expect("budget fetched");
    assert_eq!(get_resp.status(), StatusCode::OK);

    let bytes = to_bytes(get_resp.into_body(), usize::MAX)
        .await
        .expect("readable body");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert_eq!(
        body["spent_to_date_idr"].as_i64().unwrap(),
        5_500_000,
        "spent_to_date_idr should be expenses (500k) + resource costs (5M)"
    );
    assert_eq!(
        body["remaining_idr"].as_i64().unwrap(),
        50_000_000 - 5_500_000,
        "remaining_idr should be total minus spent"
    );
}

// ── Test 11: Multiple employees aggregated correctly ────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn multiple_employees_aggregated(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Multi Employee Project", pm_id).await;
    let dept_id = create_test_department(&pool, "Dept-MultiEmp").await;
    let res_a = create_test_resource(&pool, "Dev Alpha", dept_id).await;
    let res_b = create_test_resource(&pool, "Dev Beta", dept_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    create_ctc_for_resource(&pool, res_a, pm_id, 1_000_000).await;
    create_ctc_for_resource(&pool, res_b, pm_id, 800_000).await;

    // Alpha: 5 days at 100% = 5,000,000
    create_allocation(&pool, res_a, project_id, 100.0, "2026-03-02", "2026-03-06", false).await;
    // Beta: 5 days at 50% = 2,000,000
    create_allocation(&pool, res_b, project_id, 50.0, "2026-03-02", "2026-03-06", false).await;

    let (status, body) = get_resource_costs(&app, &token, project_id).await;

    assert_eq!(status, StatusCode::OK);
    // Total: 5,000,000 + 2,000,000 = 7,000,000
    assert_eq!(body["total_resource_cost_idr"].as_i64().unwrap(), 7_000_000);

    let employees = body["employees"].as_array().unwrap();
    assert_eq!(employees.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn resource_cost_with_mid_period_rate_change(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Rate Change Project", pm_id).await;
    let dept_id = create_test_department(&pool, "Dept-RateChange").await;
    let resource_id = create_test_resource(&pool, "Dev RateChange", dept_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    create_ctc_for_resource(&pool, resource_id, pm_id, 1_000_000).await;
    create_ctc_revision(&pool, resource_id, pm_id, 1, "2026-03-02", 1_000_000).await;
    create_ctc_revision(&pool, resource_id, pm_id, 2, "2026-03-09", 2_000_000).await;

    create_allocation(
        &pool,
        resource_id,
        project_id,
        100.0,
        "2026-03-02",
        "2026-03-13",
        false,
    )
    .await;

    let (status, body) = get_resource_costs(&app, &token, project_id).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_resource_cost_idr"].as_i64().unwrap(), 15_000_000);

    let employees = body["employees"].as_array().unwrap();
    assert_eq!(employees.len(), 1);
    assert_eq!(employees[0]["has_rate_change"].as_bool().unwrap(), true);
    assert!(employees[0]["rate_change_note"].as_str().unwrap().contains("Rate changed during allocation"));
}
