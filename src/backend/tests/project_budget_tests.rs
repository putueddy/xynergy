use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("projbudget-{}@example.com", Uuid::new_v4())
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
         VALUES ($1, 'Active', CURRENT_DATE, CURRENT_DATE + INTERVAL '90 days', $2)
         RETURNING id",
    )
    .bind(name)
    .bind(pm_id)
    .fetch_one(pool)
    .await
    .expect("project created")
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_can_set_budget_on_own_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "PM Owned Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 100000000_i64,
                "budget_hr_idr": 50000000_i64,
                "budget_software_idr": 20000000_i64,
                "budget_hardware_idr": 15000000_i64,
                "budget_overhead_idr": 15000000_i64,
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("readable response body");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON response");

    assert_eq!(
        body["project_id"]
            .as_str()
            .expect("project_id should be present"),
        project_id.to_string()
    );
    assert_eq!(
        body["total_budget_idr"]
            .as_i64()
            .expect("total_budget_idr should be present"),
        100000000_i64
    );
    assert_eq!(
        body["spent_to_date_idr"]
            .as_i64()
            .expect("spent_to_date_idr should be present"),
        0_i64
    );

    let hr_pct = body["hr_pct"].as_f64().expect("hr_pct should be present");
    let software_pct = body["software_pct"]
        .as_f64()
        .expect("software_pct should be present");
    let hardware_pct = body["hardware_pct"]
        .as_f64()
        .expect("hardware_pct should be present");
    let overhead_pct = body["overhead_pct"]
        .as_f64()
        .expect("overhead_pct should be present");

    assert!((hr_pct - 50.0).abs() < 0.000001, "hr_pct should be 50.0");
    assert!(
        (software_pct - 20.0).abs() < 0.000001,
        "software_pct should be 20.0"
    );
    assert!(
        (hardware_pct - 15.0).abs() < 0.000001,
        "hardware_pct should be 15.0"
    );
    assert!(
        (overhead_pct - 15.0).abs() < 0.000001,
        "overhead_pct should be 15.0"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_can_get_budget_on_own_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "PM Budget Read Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let set_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 100000000_i64,
                "budget_hr_idr": 50000000_i64,
                "budget_software_idr": 20000000_i64,
                "budget_hardware_idr": 15000000_i64,
                "budget_overhead_idr": 15000000_i64,
            })
            .to_string(),
        ))
        .expect("set budget request should be built");
    let set_resp = app
        .clone()
        .oneshot(set_req)
        .await
        .expect("set budget should return response");
    assert_eq!(set_resp.status(), StatusCode::OK);

    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("get budget request should be built");
    let get_resp = app
        .clone()
        .oneshot(get_req)
        .await
        .expect("get budget should return response");
    assert_eq!(get_resp.status(), StatusCode::OK);

    let bytes = to_bytes(get_resp.into_body(), usize::MAX)
        .await
        .expect("get response body should be readable");
    let body: Value = serde_json::from_slice(&bytes).expect("get response should be valid JSON");

    assert_eq!(
        body["project_id"]
            .as_str()
            .expect("project_id should be present"),
        project_id.to_string()
    );
    assert_eq!(
        body["total_budget_idr"]
            .as_i64()
            .expect("total_budget_idr should be present"),
        100000000_i64
    );
    assert_eq!(
        body["budget_hr_idr"]
            .as_i64()
            .expect("budget_hr_idr should be present"),
        50000000_i64
    );
    assert_eq!(
        body["budget_software_idr"]
            .as_i64()
            .expect("budget_software_idr should be present"),
        20000000_i64
    );
    assert_eq!(
        body["budget_hardware_idr"]
            .as_i64()
            .expect("budget_hardware_idr should be present"),
        15000000_i64
    );
    assert_eq!(
        body["budget_overhead_idr"]
            .as_i64()
            .expect("budget_overhead_idr should be present"),
        15000000_i64
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_denied_budget_on_non_owned_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_a_email = test_email();
    let pm_a_id = create_test_user_with_role(&pool, &pm_a_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "PM A Project", pm_a_id).await;

    let pm_b_email = test_email();
    let _pm_b_id = create_test_user_with_role(&pool, &pm_b_email, "project_manager").await;
    let pm_b_token = get_auth_token(&app, &pm_b_email).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", pm_b_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 100000000_i64,
                "budget_hr_idr": 50000000_i64,
                "budget_software_idr": 20000000_i64,
                "budget_hardware_idr": 15000000_i64,
                "budget_overhead_idr": 15000000_i64,
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_denied_get_budget_on_non_owned_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_a_email = test_email();
    let pm_a_id = create_test_user_with_role(&pool, &pm_a_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "PM A Read Project", pm_a_id).await;

    let pm_b_email = test_email();
    let _pm_b_id = create_test_user_with_role(&pool, &pm_b_email, "project_manager").await;
    let pm_b_token = get_auth_token(&app, &pm_b_email).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", pm_b_token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_pm_role_denied_budget_write(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Project For Dept Head Test", pm_id).await;

    let dept_head_email = test_email();
    let _dept_head_id =
        create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    let dept_head_token = get_auth_token(&app, &dept_head_email).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", dept_head_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 100000000_i64,
                "budget_hr_idr": 50000000_i64,
                "budget_software_idr": 20000000_i64,
                "budget_hardware_idr": 15000000_i64,
                "budget_overhead_idr": 15000000_i64,
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_set_budget_on_any_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Admin Budget Project", pm_id).await;

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 100000000_i64,
                "budget_hr_idr": 50000000_i64,
                "budget_software_idr": 20000000_i64,
                "budget_hardware_idr": 15000000_i64,
                "budget_overhead_idr": 15000000_i64,
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_category_sum_rejected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Invalid Sum Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 100000000_i64,
                "budget_hr_idr": 50000000_i64,
                "budget_software_idr": 20000000_i64,
                "budget_hardware_idr": 15000000_i64,
                "budget_overhead_idr": 10000000_i64,
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("error response body should be readable");
    let body: Value = serde_json::from_slice(&bytes).expect("error response should be valid JSON");
    assert_eq!(
        body["error"]["code"]
            .as_str()
            .expect("error code should be present"),
        "VALIDATION_ERROR"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn negative_budget_value_rejected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Negative Budget Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 100000000_i64,
                "budget_hr_idr": -50000000_i64,
                "budget_software_idr": 20000000_i64,
                "budget_hardware_idr": 15000000_i64,
                "budget_overhead_idr": 115000000_i64,
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn zero_total_budget_rejected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Zero Budget Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 0_i64,
                "budget_hr_idr": 0_i64,
                "budget_software_idr": 0_i64,
                "budget_hardware_idr": 0_i64,
                "budget_overhead_idr": 0_i64,
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn decimal_budget_value_rejected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Decimal Budget Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 100000000.5,
                "budget_hr_idr": 50000000,
                "budget_software_idr": 20000000,
                "budget_hardware_idr": 15000000,
                "budget_overhead_idr": 15000000,
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");

    assert!(
        resp.status() == StatusCode::BAD_REQUEST
            || resp.status() == StatusCode::UNPROCESSABLE_ENTITY,
        "decimal payload should be rejected; got status {}",
        resp.status()
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_project_with_budget_fields_persists_budget(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let token = get_auth_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Create With Budget",
                "client": "Budget Client",
                "description": "Budget create test",
                "start_date": "2026-02-01",
                "end_date": "2026-04-30",
                "status": "Active",
                "project_manager_id": Value::Null,
                "total_budget_idr": 100000000_i64,
                "budget_hr_idr": 50000000_i64,
                "budget_software_idr": 20000000_i64,
                "budget_hardware_idr": 15000000_i64,
                "budget_overhead_idr": 15000000_i64
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("create should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let json: Value = serde_json::from_slice(&body).expect("response should be valid JSON");
    assert_eq!(json["total_budget_idr"].as_i64(), Some(100000000_i64));
    assert_eq!(json["budget_hr_idr"].as_i64(), Some(50000000_i64));
    assert_eq!(json["budget_software_idr"].as_i64(), Some(20000000_i64));
    assert_eq!(json["budget_hardware_idr"].as_i64(), Some(15000000_i64));
    assert_eq!(json["budget_overhead_idr"].as_i64(), Some(15000000_i64));
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_project_with_budget_fields_persists_budget(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let token = get_auth_token(&app, &admin_email).await;

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Update Budget Project",
                "description": "Initial project",
                "start_date": "2026-01-01",
                "end_date": "2026-03-31",
                "status": "Active",
                "project_manager_id": Value::Null
            })
            .to_string(),
        ))
        .expect("create request should be built");
    let create_resp = app
        .clone()
        .oneshot(create_req)
        .await
        .expect("create should return response");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = to_bytes(create_resp.into_body(), usize::MAX)
        .await
        .expect("create body should be readable");
    let created: Value = serde_json::from_slice(&create_body).expect("create body should be JSON");
    let project_id = created["id"]
        .as_str()
        .expect("project id should be present")
        .to_string();

    let update_req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/projects/{}", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 120000000_i64,
                "budget_hr_idr": 60000000_i64,
                "budget_software_idr": 30000000_i64,
                "budget_hardware_idr": 15000000_i64,
                "budget_overhead_idr": 15000000_i64
            })
            .to_string(),
        ))
        .expect("update request should be built");
    let update_resp = app
        .clone()
        .oneshot(update_req)
        .await
        .expect("update should return response");
    assert_eq!(update_resp.status(), StatusCode::OK);

    let update_body = to_bytes(update_resp.into_body(), usize::MAX)
        .await
        .expect("update body should be readable");
    let updated: Value =
        serde_json::from_slice(&update_body).expect("update body should be valid JSON");
    assert_eq!(updated["total_budget_idr"].as_i64(), Some(120000000_i64));
    assert_eq!(updated["budget_hr_idr"].as_i64(), Some(60000000_i64));
    assert_eq!(updated["budget_software_idr"].as_i64(), Some(30000000_i64));
    assert_eq!(updated["budget_hardware_idr"].as_i64(), Some(15000000_i64));
    assert_eq!(updated["budget_overhead_idr"].as_i64(), Some(15000000_i64));
}

#[sqlx::test(migrations = "../../migrations")]
async fn audit_log_emitted_on_budget_set(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Audit Budget Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 100000000_i64,
                "budget_hr_idr": 50000000_i64,
                "budget_software_idr": 20000000_i64,
                "budget_hardware_idr": 15000000_i64,
                "budget_overhead_idr": 15000000_i64,
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*)
         FROM audit_logs
         WHERE entity_type = 'project_budget'
           AND action = 'update'
           AND entity_id = $1",
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await
    .expect("audit log query should succeed");

    assert!(
        audit_count > 0,
        "audit log should contain project_budget update for project"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn existing_project_crud_still_works(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let token = get_auth_token(&app, &admin_email).await;

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/projects")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "CRUD Project",
                "client": "ACME Corp",
                "description": "Initial project",
                "start_date": "2026-01-01",
                "end_date": "2026-03-31",
                "status": "Active",
                "project_manager_id": Value::Null,
            })
            .to_string(),
        ))
        .expect("create project request should be built");
    let create_resp = app
        .clone()
        .oneshot(create_req)
        .await
        .expect("create project should return response");
    assert_eq!(create_resp.status(), StatusCode::OK);

    let create_bytes = to_bytes(create_resp.into_body(), usize::MAX)
        .await
        .expect("create response body should be readable");
    let created: Value =
        serde_json::from_slice(&create_bytes).expect("create response should be valid JSON");
    let project_id = created["id"]
        .as_str()
        .expect("created project should have id")
        .to_string();

    let list_req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("list projects request should be built");
    let list_resp = app
        .clone()
        .oneshot(list_req)
        .await
        .expect("list projects should return response");
    assert_eq!(list_resp.status(), StatusCode::OK);

    let list_bytes = to_bytes(list_resp.into_body(), usize::MAX)
        .await
        .expect("list response body should be readable");
    let projects: Value =
        serde_json::from_slice(&list_bytes).expect("list response should be valid JSON");
    let projects_arr = projects
        .as_array()
        .expect("projects list should be an array");

    let created_project = projects_arr
        .iter()
        .find(|p| p["id"].as_str().expect("project entry should have id") == project_id)
        .expect("created project should be present in list");
    assert_eq!(
        created_project["client"]
            .as_str()
            .expect("created project should have client"),
        "ACME Corp"
    );
    assert_eq!(
        created_project["total_budget_idr"]
            .as_i64()
            .expect("created project should have total_budget_idr"),
        0_i64
    );
    assert_eq!(
        created_project["budget_hr_idr"]
            .as_i64()
            .expect("created project should have budget_hr_idr"),
        0_i64
    );
    assert_eq!(
        created_project["budget_software_idr"]
            .as_i64()
            .expect("created project should have budget_software_idr"),
        0_i64
    );
    assert_eq!(
        created_project["budget_hardware_idr"]
            .as_i64()
            .expect("created project should have budget_hardware_idr"),
        0_i64
    );
    assert_eq!(
        created_project["budget_overhead_idr"]
            .as_i64()
            .expect("created project should have budget_overhead_idr"),
        0_i64
    );

    let update_req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/projects/{}", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "CRUD Project Updated"
            })
            .to_string(),
        ))
        .expect("update project request should be built");
    let update_resp = app
        .clone()
        .oneshot(update_req)
        .await
        .expect("update project should return response");
    assert_eq!(update_resp.status(), StatusCode::OK);

    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("get project request should be built");
    let get_resp = app
        .clone()
        .oneshot(get_req)
        .await
        .expect("get project should return response");
    assert_eq!(get_resp.status(), StatusCode::OK);

    let get_bytes = to_bytes(get_resp.into_body(), usize::MAX)
        .await
        .expect("get response body should be readable");
    let got: Value = serde_json::from_slice(&get_bytes).expect("get response should be valid JSON");
    assert_eq!(
        got["name"].as_str().expect("project name should be present"),
        "CRUD Project Updated"
    );

    let delete_req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/projects/{}", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("delete project request should be built");
    let delete_resp = app
        .clone()
        .oneshot(delete_req)
        .await
        .expect("delete project should return response");
    assert_eq!(delete_resp.status(), StatusCode::OK);

    let get_deleted_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("get deleted project request should be built");
    let get_deleted_resp = app
        .clone()
        .oneshot(get_deleted_req)
        .await
        .expect("get deleted project should return response");
    assert_eq!(get_deleted_resp.status(), StatusCode::NOT_FOUND);
}
