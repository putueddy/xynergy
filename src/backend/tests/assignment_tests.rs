use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Local;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("assign-{}@example.com", Uuid::new_v4())
}

/// Returns (start_date, end_date) strings within a project's 90-day window.
/// start = today + 7d, end = today + 60d (well within CURRENT_DATE + 90 days).
fn allocation_dates() -> (String, String) {
    let today = Local::now().date_naive();
    let start = today + chrono::Duration::days(7);
    let end = today + chrono::Duration::days(60);
    (
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
    )
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
    let _encrypted_components = crypto_svc
        .encrypt_components(&json!({"base_salary": 10000000}))
        .await
        .expect("components encryption should work");

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
         VALUES ($1, 'Active', CURRENT_DATE, CURRENT_DATE + INTERVAL '90 days')
         RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("project created")
}

async fn create_test_project_with_manager(pool: &PgPool, name: &str, manager_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, status, start_date, end_date, project_manager_id)
         VALUES ($1, 'Active', CURRENT_DATE, CURRENT_DATE + INTERVAL '90 days', $2)
         RETURNING id",
    )
    .bind(name)
    .bind(manager_id)
    .fetch_one(pool)
    .await
    .expect("project with manager created")
}

async fn create_inactive_project(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, status, start_date, end_date)
         VALUES ($1, 'Completed', CURRENT_DATE - INTERVAL '90 days', CURRENT_DATE - INTERVAL '1 day')
         RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("inactive project created")
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

fn set_test_env() {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );
}

// =========================================================================
// Task 2 Tests: Assignable projects endpoint visibility
// =========================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn assignable_projects_dept_head_sees_all_active(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let dept_head_email = test_email();
    let dept_head_id =
        create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, dept_id).await;

    let _active_project = create_test_project(&pool, "Active Project").await;
    let _inactive_project = create_inactive_project(&pool, "Completed Project").await;

    let token = get_auth_token(&app, &dept_head_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/assignable")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Vec<Value> = serde_json::from_slice(&bytes).unwrap();

    assert!(
        body.iter().any(|p| p["name"] == "Active Project"),
        "dept head should see active project"
    );
    assert!(
        !body.iter().any(|p| p["name"] == "Completed Project"),
        "dept head should not see inactive project"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn assignable_projects_admin_sees_all_active(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;

    let _proj1 = create_test_project(&pool, "Alpha Project").await;
    let _proj2 = create_test_project(&pool, "Beta Project").await;
    let _inactive = create_inactive_project(&pool, "Old Project").await;

    let token = get_auth_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/assignable")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Vec<Value> = serde_json::from_slice(&bytes).unwrap();

    let names: Vec<&str> = body.iter().filter_map(|p| p["name"].as_str()).collect();
    assert!(names.contains(&"Alpha Project"), "admin should see Alpha");
    assert!(names.contains(&"Beta Project"), "admin should see Beta");
    assert!(
        !names.contains(&"Old Project"),
        "admin should not see inactive"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn assignable_projects_pm_sees_only_managed(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;

    let _managed = create_test_project_with_manager(&pool, "My Project", pm_id).await;
    let _unmanaged = create_test_project(&pool, "Other Project").await;
    let _inactive_managed = create_inactive_project(&pool, "My Old Project").await;
    sqlx::query("UPDATE projects SET project_manager_id = $1 WHERE name = 'My Old Project'")
        .bind(pm_id)
        .execute(&pool)
        .await
        .unwrap();

    let token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/assignable")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Vec<Value> = serde_json::from_slice(&bytes).unwrap();

    let names: Vec<&str> = body.iter().filter_map(|p| p["name"].as_str()).collect();
    assert!(
        names.contains(&"My Project"),
        "PM should see managed active project"
    );
    assert!(
        !names.contains(&"Other Project"),
        "PM should not see unmanaged project"
    );
    assert!(
        !names.contains(&"My Old Project"),
        "PM should not see inactive managed project"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn assignable_projects_unauthorized_role_forbidden(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let token = get_auth_token(&app, &hr_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/assignable")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "hr role should be forbidden from assignable projects"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn assignable_projects_response_has_required_fields(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let _proj = create_test_project(&pool, "Field Check Project").await;

    let token = get_auth_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/assignable")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Vec<Value> = serde_json::from_slice(&bytes).unwrap();

    let proj = body
        .iter()
        .find(|p| p["name"] == "Field Check Project")
        .expect("project should be in response");
    assert!(!proj["id"].is_null(), "id should be present");
    assert!(!proj["name"].is_null(), "name should be present");
    assert!(
        !proj["start_date"].is_null(),
        "start_date should be present"
    );
    assert!(!proj["end_date"].is_null(), "end_date should be present");
    assert!(!proj["status"].is_null(), "status should be present");
    assert!(
        proj.get("description").is_none(),
        "description should not be in assignable response"
    );
    assert!(
        proj.get("project_manager_id").is_none(),
        "project_manager_id should not be in assignable response"
    );
}

// =========================================================================
// Task 3 Tests: Capacity and overlap validation
// =========================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn capacity_exact_100_percent_allowed(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let resource_id =
        create_test_resource_in_dept(&pool, "Full Capacity Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project_id = create_test_project(&pool, "Capacity Project").await;

    let token = get_auth_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 100.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(bytes.to_vec()).unwrap();

    assert_eq!(
        status,
        StatusCode::OK,
        "allocation at exactly 100% should be allowed, got: {}",
        body_str
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn capacity_over_100_percent_rejected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let resource_id =
        create_test_resource_in_dept(&pool, "Over Capacity Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project_a = create_test_project(&pool, "Project A").await;
    let project_b = create_test_project(&pool, "Project B").await;

    let token = get_auth_token(&app, &admin_email).await;

    // First allocation at 60%
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_a,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 60.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "first allocation should succeed"
    );

    // Second allocation at 50% — total 110% should be rejected
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_b,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 50.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "combined allocation >100% should be rejected"
    );

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        body.contains("over-allocated"),
        "rejection message should mention over-allocation: {}",
        body
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn capacity_sum_exactly_100_with_two_allocations(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let resource_id = create_test_resource_in_dept(&pool, "Split Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project_a = create_test_project(&pool, "Project X").await;
    let project_b = create_test_project(&pool, "Project Y").await;

    let token = get_auth_token(&app, &admin_email).await;

    // First allocation at 60%
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_a,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 60.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "first 60% allocation should succeed"
    );

    // Second allocation at 40% — total exactly 100% should pass
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_b,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 40.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "combined allocation at exactly 100% should be allowed"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn capacity_decimal_edge_case(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let resource_id = create_test_resource_in_dept(&pool, "Decimal Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project_a = create_test_project(&pool, "Decimal A").await;
    let project_b = create_test_project(&pool, "Decimal B").await;
    let project_c = create_test_project(&pool, "Decimal C").await;

    let token = get_auth_token(&app, &admin_email).await;

    // 33.33%
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_a,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 33.33,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK, "33.33% should succeed");

    // 33.33%
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_b,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 33.33,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK, "second 33.33% should succeed");

    // 33.34% — total 100.00 — should succeed
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_c,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 33.34,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "33.33 + 33.33 + 33.34 = 100 should be allowed"
    );
}

// =========================================================================
// Task 6 Tests: Assignment create by role, CTC guard, unauthorized
// =========================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn dept_head_can_create_assignment(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let dept_head_email = test_email();
    let dept_head_id =
        create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, dept_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "Eng Dev", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, dept_head_id).await;
    let project_id = create_test_project(&pool, "Eng Project").await;

    let token = get_auth_token(&app, &dept_head_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 50.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "department_head should be able to create assignment"
    );

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["allocation_percentage"], 50.0);
    assert_eq!(body["resource_id"], resource_id.to_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_can_create_assignment_on_managed_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;

    let resource_id = create_test_resource_in_dept(&pool, "Dev", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, pm_id).await;
    let project_id = create_test_project_with_manager(&pool, "PM Project", pm_id).await;

    let token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 30.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "PM should create assignment on managed project"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_cannot_create_assignment_on_unmanaged_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;

    let resource_id = create_test_resource_in_dept(&pool, "Dev", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, pm_id).await;
    let project_id = create_test_project(&pool, "Unmanaged Project").await;

    let token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 30.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "PM should not create assignment on unmanaged project"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_cannot_create_assignment(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let hr_email = test_email();
    let hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;

    let resource_id = create_test_resource_in_dept(&pool, "Dev", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, hr_id).await;
    let project_id = create_test_project(&pool, "Any Project").await;

    let token = get_auth_token(&app, &hr_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 30.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "hr role should not be able to create assignment"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn ctc_missing_rejects_assignment_with_message(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let resource_id = create_test_resource_in_dept(&pool, "No CTC Dev", dept_id).await;
    let project_id = create_test_project(&pool, "CTC Test Project").await;

    let token = get_auth_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 50.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        body.contains("Cannot assign resource without CTC data"),
        "CTC rejection should have deterministic message: {}",
        body
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_create_assignment(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let resource_id = create_test_resource_in_dept(&pool, "Admin Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project_id = create_test_project(&pool, "Admin Project").await;

    let token = get_auth_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 75.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "admin should be able to create assignment"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn assignment_creates_audit_log(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let resource_id = create_test_resource_in_dept(&pool, "Audited Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project_id = create_test_project(&pool, "Audit Project").await;

    let token = get_auth_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 50.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE entity_type = 'allocation' AND action = 'create'",
    )
    .fetch_one(&pool)
    .await
    .expect("audit query should work");

    assert!(
        audit_count > 0,
        "audit log should contain create entry for allocation"
    );
}

// =========================================================================
// Review Fix Tests: Backend input validation (H1, H2)
// =========================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn allocation_percentage_bounds_rejected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (sd, ed) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let resource_id = create_test_resource_in_dept(&pool, "Bounds Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project_id = create_test_project(&pool, "Bounds Project").await;

    let token = get_auth_token(&app, &admin_email).await;

    // Zero percent should be rejected
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 0.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "allocation at 0% should be rejected"
    );

    // Over 100% single allocation should be rejected
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": 150.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "allocation at 150% should be rejected"
    );

    // Negative should be rejected
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": sd,
                "end_date": ed,
                "allocation_percentage": -10.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "allocation at -10% should be rejected"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn inverted_dates_rejected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let resource_id = create_test_resource_in_dept(&pool, "Date Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project_id = create_test_project(&pool, "Date Project").await;

    let token = get_auth_token(&app, &admin_email).await;

    // start_date after end_date should be rejected
    let today = chrono::Local::now().date_naive();
    let start = (today + chrono::Duration::days(60))
        .format("%Y-%m-%d")
        .to_string();
    let end = (today + chrono::Duration::days(7))
        .format("%Y-%m-%d")
        .to_string();

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": start,
                "end_date": end,
                "allocation_percentage": 50.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app.clone().oneshot(req).await.expect("response");

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "inverted date range should be rejected"
    );

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        body.contains("Start date cannot be after end date"),
        "rejection message should mention date ordering: {}",
        body
    );
}
