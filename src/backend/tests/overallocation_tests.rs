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
    format!("overallocation-{}@example.com", Uuid::new_v4())
}

fn set_test_env() {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );
}

fn allocation_dates() -> (String, String) {
    let today = Local::now().date_naive();
    let start = today + chrono::Duration::days(7);
    let end = today + chrono::Duration::days(60);
    (
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
    )
}

/// Returns dates that span CURRENT_DATE for team endpoint tests.
/// start = today (matches project start), end = today + 60d.
fn current_spanning_allocation_dates() -> (String, String) {
    let today = Local::now().date_naive();
    let start = today;
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

async fn assign_user_to_department(pool: &PgPool, user_id: Uuid, dept_id: Uuid) {
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(dept_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("user department assigned");
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
    .expect("ctc record created");
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
    let body: Value = serde_json::from_slice(&bytes).expect("login response should be JSON");
    body["token"]
        .as_str()
        .expect("login response should include token")
        .to_string()
}

async fn create_allocation_request(
    app: &axum::Router,
    token: &str,
    resource_id: Uuid,
    project_id: Uuid,
    start_date: &str,
    end_date: &str,
    allocation_percentage: f64,
    confirm_overallocation: bool,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "resource_id": resource_id,
                "project_id": project_id,
                "start_date": start_date,
                "end_date": end_date,
                "allocation_percentage": allocation_percentage,
                "include_weekend": false,
                "confirm_overallocation": confirm_overallocation
            })
            .to_string(),
        ))
        .expect("allocation request should be built");

    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("allocation request should return response");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("allocation response body should be readable");
    let body: Value = serde_json::from_slice(&bytes).expect("allocation response should be JSON");
    (status, body)
}

#[sqlx::test(migrations = "../../migrations")]
async fn warning_returned_when_overallocated_without_confirmation(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (start_date, end_date) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    assign_user_to_department(&pool, admin_id, dept_id).await;
    let resource_id = create_test_resource_in_dept(&pool, "Overallocated Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project_a = create_test_project(&pool, "Project A").await;
    let project_b = create_test_project(&pool, "Project B").await;

    let token = get_auth_token(&app, &admin_email).await;

    let (status_a, body_a) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_a,
        &start_date,
        &end_date,
        80.0,
        false,
    )
    .await;
    assert_eq!(status_a, StatusCode::OK);
    assert_eq!(body_a["status"], "created");

    let (status_b, body_b) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_b,
        &start_date,
        &end_date,
        30.0,
        false,
    )
    .await;

    assert_eq!(status_b, StatusCode::OK);
    assert_eq!(body_b["status"], "overallocation_warning");
    assert_eq!(
        body_b["projected_allocation_percentage"].as_f64(),
        Some(110.0)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn confirmed_overallocation_creates_assignment(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (start_date, end_date) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    assign_user_to_department(&pool, admin_id, dept_id).await;
    let resource_id = create_test_resource_in_dept(&pool, "Confirmed Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project_a = create_test_project(&pool, "Project A").await;
    let project_b = create_test_project(&pool, "Project B").await;

    let token = get_auth_token(&app, &admin_email).await;

    let (_status_a, _body_a) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_a,
        &start_date,
        &end_date,
        80.0,
        false,
    )
    .await;

    let (status_b, body_b) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_b,
        &start_date,
        &end_date,
        30.0,
        true,
    )
    .await;

    assert_eq!(status_b, StatusCode::OK);
    assert_eq!(body_b["status"], "created");
    assert_eq!(
        body_b["allocation"]["allocation_percentage"].as_f64(),
        Some(30.0)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn overallocation_confirmation_creates_audit_entry(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (start_date, end_date) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    assign_user_to_department(&pool, admin_id, dept_id).await;
    let resource_id = create_test_resource_in_dept(&pool, "Audit Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project_a = create_test_project(&pool, "Project A").await;
    let project_b = create_test_project(&pool, "Project B").await;

    let token = get_auth_token(&app, &admin_email).await;

    // First allocation at 80%
    let (_status_a, _body_a) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_a,
        &start_date,
        &end_date,
        80.0,
        false,
    )
    .await;

    // Second allocation at 30% with confirm=true → triggers overallocation_confirmed audit
    let (status_b, _body_b) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_b,
        &start_date,
        &end_date,
        30.0,
        true,
    )
    .await;
    assert_eq!(status_b, StatusCode::OK);

    // Verify audit_logs contains overallocation_confirmed entry
    let audit_entry = sqlx::query_as::<_, (String, String)>(
        "SELECT action, changes::TEXT FROM audit_logs WHERE action = 'overallocation_confirmed' ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await
    .expect("audit query should succeed");

    let (action, changes_text) =
        audit_entry.expect("overallocation_confirmed audit entry should exist");
    assert_eq!(action, "overallocation_confirmed");

    let changes: Value = serde_json::from_str(&changes_text).expect("changes should be valid JSON");
    assert_eq!(
        changes["current_allocation_percentage"].as_f64(),
        Some(80.0)
    );
    assert_eq!(
        changes["requested_allocation_percentage"].as_f64(),
        Some(30.0)
    );
    assert_eq!(
        changes["projected_allocation_percentage"].as_f64(),
        Some(110.0)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn team_endpoint_marks_overallocated_resources(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (start_date, end_date) = current_spanning_allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let dept_head_email = test_email();
    let dept_head_id = create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, dept_id).await;
    let resource_id = create_test_resource_in_dept(&pool, "Team Flag Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, dept_head_id).await;
    let project_a = create_test_project(&pool, "Project A").await;
    let project_b = create_test_project(&pool, "Project B").await;

    let token = get_auth_token(&app, &dept_head_email).await;

    let (_status_a, _body_a) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_a,
        &start_date,
        &end_date,
        60.0,
        false,
    )
    .await;

    let (_status_b, _body_b) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_b,
        &start_date,
        &end_date,
        50.0,
        true,
    )
    .await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/team")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("team request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("team request should return response");

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("team response should be readable");
    let body: Vec<Value> = serde_json::from_slice(&bytes).expect("team response should be JSON");

    let resource = body
        .iter()
        .find(|item| item["resource_id"].as_str() == Some(resource_id.to_string().as_str()))
        .expect("resource should be present in team response");

    assert_eq!(resource["is_overallocated"].as_bool(), Some(true));
    assert_eq!(
        resource["current_allocation_percentage"].as_f64(),
        Some(110.0)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn capacity_report_highlights_overallocated_periods(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (start_date, end_date) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let dept_head_email = test_email();
    let dept_head_id = create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, dept_id).await;
    let resource_id = create_test_resource_in_dept(&pool, "Capacity Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, dept_head_id).await;
    let project_a = create_test_project(&pool, "Project A").await;
    let project_b = create_test_project(&pool, "Project B").await;

    let token = get_auth_token(&app, &dept_head_email).await;

    let (_status_a, _body_a) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_a,
        &start_date,
        &end_date,
        60.0,
        false,
    )
    .await;

    let (_status_b, _body_b) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_b,
        &start_date,
        &end_date,
        50.0,
        true,
    )
    .await;

    let url = format!(
        "/api/v1/team/capacity-report?start_date={}&end_date={}",
        start_date, end_date
    );
    let req = Request::builder()
        .method("GET")
        .uri(&url)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("capacity report request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("capacity report request should return response");

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("capacity report response should be readable");
    let body: Value = serde_json::from_slice(&bytes).expect("capacity report should be JSON");

    let employees = body["employees"]
        .as_array()
        .expect("employees should be an array");
    let employee = employees
        .iter()
        .find(|e| e["resource_id"].as_str() == Some(resource_id.to_string().as_str()))
        .expect("resource should be present in capacity report");

    let periods = employee["periods"]
        .as_array()
        .expect("periods should be an array");
    assert!(
        periods
            .iter()
            .any(|p| p["is_overallocated"].as_bool() == Some(true)),
        "at least one capacity period should be overallocated"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn under_100_creates_normally_regardless_of_confirm_flag(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());
    let (start_date, end_date) = allocation_dates();

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    assign_user_to_department(&pool, admin_id, dept_id).await;
    let resource_id = create_test_resource_in_dept(&pool, "Under 100 Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, admin_id).await;
    let project = create_test_project(&pool, "Project").await;

    let token = get_auth_token(&app, &admin_email).await;

    let (status, body) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project,
        &start_date,
        &end_date,
        50.0,
        true,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "created");
    assert_eq!(
        body["allocation"]["allocation_percentage"].as_f64(),
        Some(50.0)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn date_aware_overlap_accuracy(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let dept_head_email = test_email();
    let dept_head_id = create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, dept_id).await;
    let resource_id = create_test_resource_in_dept(&pool, "Date Aware Resource", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, dept_head_id).await;

    let project_a_id: Uuid = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, status, start_date, end_date) VALUES ($1, 'Active', '2027-01-01', '2027-04-30') RETURNING id"
    )
    .bind("Project A")
    .fetch_one(&pool)
    .await
    .expect("project a created");

    let project_b_id: Uuid = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, status, start_date, end_date) VALUES ($1, 'Active', '2027-01-01', '2027-04-30') RETURNING id"
    )
    .bind("Project B")
    .fetch_one(&pool)
    .await
    .expect("project b created");

    let token = get_auth_token(&app, &dept_head_email).await;

    let (_status_a, _body_a) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_a_id,
        "2027-01-01",
        "2027-03-31",
        60.0,
        false,
    )
    .await;

    let (_status_b, _body_b) = create_allocation_request(
        &app,
        &token,
        resource_id,
        project_b_id,
        "2027-02-01",
        "2027-04-30",
        50.0,
        true,
    )
    .await;

    let url = "/api/v1/team/capacity-report?start_date=2027-01-01&end_date=2027-04-30";
    let req = Request::builder()
        .method("GET")
        .uri(url)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("capacity report request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("capacity report request should return response");

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("capacity report response should be readable");
    let body: Value = serde_json::from_slice(&bytes).expect("capacity report should be JSON");

    let employees = body["employees"]
        .as_array()
        .expect("employees should be an array");
    let employee = employees
        .iter()
        .find(|e| e["resource_id"].as_str() == Some(resource_id.to_string().as_str()))
        .expect("resource should be present in capacity report");

    let periods = employee["periods"]
        .as_array()
        .expect("periods should be an array");

    let find_period = |periods: &Vec<Value>, key: &str| -> Option<Value> {
        periods
            .iter()
            .find(|p| p["period"].as_str() == Some(key))
            .cloned()
    };

    let jan_period = find_period(periods, "2027-01").expect("2027-01 period should exist");
    assert_eq!(
        jan_period["total_allocation_percentage"].as_f64(),
        Some(60.0)
    );
    assert_eq!(jan_period["is_overallocated"].as_bool(), Some(false));

    let feb_period = find_period(periods, "2027-02").expect("2027-02 period should exist");
    assert_eq!(
        feb_period["total_allocation_percentage"].as_f64(),
        Some(110.0)
    );
    assert_eq!(feb_period["is_overallocated"].as_bool(), Some(true));

    let mar_period = find_period(periods, "2027-03").expect("2027-03 period should exist");
    assert_eq!(
        mar_period["total_allocation_percentage"].as_f64(),
        Some(110.0)
    );
    assert_eq!(mar_period["is_overallocated"].as_bool(), Some(true));

    let apr_period = find_period(periods, "2027-04").expect("2027-04 period should exist");
    assert_eq!(
        apr_period["total_allocation_percentage"].as_f64(),
        Some(50.0)
    );
    assert_eq!(apr_period["is_overallocated"].as_bool(), Some(false));
}
