use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("team-{}@example.com", Uuid::new_v4())
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
         VALUES ($1, 'employee', 1.0, $2)
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

fn find_resource<'a>(body: &'a [Value], resource_id: Uuid) -> &'a Value {
    body.iter()
        .find(|item| item["resource_id"].as_str() == Some(resource_id.to_string().as_str()))
        .expect("resource should be present in response")
}

#[sqlx::test(migrations = "../../migrations")]
async fn dept_head_sees_own_department_only(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );

    let app = xynergy_backend::create_app(pool.clone());

    let engineering_id = create_test_department(&pool, "Engineering").await;
    let marketing_id = create_test_department(&pool, "Marketing").await;

    let dept_head_email = test_email();
    let dept_head_id = create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, engineering_id).await;

    let engineering_resource_id =
        create_test_resource_in_dept(&pool, "Eng Resource", engineering_id).await;
    let marketing_resource_id =
        create_test_resource_in_dept(&pool, "Mkt Resource", marketing_id).await;
    create_ctc_for_resource(&pool, engineering_resource_id, dept_head_id).await;
    create_ctc_for_resource(&pool, marketing_resource_id, dept_head_id).await;

    let token = get_auth_token(&app, &dept_head_email).await;

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
        .expect("team response body should be readable");
    let body: Vec<Value> =
        serde_json::from_slice(&bytes).expect("team response should be an array payload");

    assert_eq!(
        body.len(),
        1,
        "department head should only see one department"
    );
    let only_member = body.first().expect("response should contain one member");
    assert_eq!(only_member["department_name"], "Engineering");
    assert_eq!(
        only_member["resource_id"]
            .as_str()
            .expect("resource id should be string"),
        engineering_resource_id.to_string()
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_sees_own_department_and_can_access_other(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );

    let app = xynergy_backend::create_app(pool.clone());

    let engineering_id = create_test_department(&pool, "Engineering").await;
    let marketing_id = create_test_department(&pool, "Marketing").await;
    let eng_resource_id = create_test_resource_in_dept(&pool, "Engineer 1", engineering_id).await;
    let mkt_resource_id = create_test_resource_in_dept(&pool, "Marketer 1", marketing_id).await;

    let hr_email = test_email();
    let hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    assign_user_to_department(&pool, hr_id, engineering_id).await;
    create_ctc_for_resource(&pool, eng_resource_id, hr_id).await;
    create_ctc_for_resource(&pool, mkt_resource_id, hr_id).await;
    let token = get_auth_token(&app, &hr_email).await;

    // HR default (own department) — should see Engineering only
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
        .expect("team response body should be readable");
    let body: Vec<Value> =
        serde_json::from_slice(&bytes).expect("team response should be an array payload");

    // HR sees own department by default
    assert!(
        body.iter()
            .any(|item| item["resource_id"].as_str() == Some(eng_resource_id.to_string().as_str())),
        "HR should see Engineering resource in own department"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_gets_403_forbidden(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );

    let app = xynergy_backend::create_app(pool.clone());
    let pm_email = test_email();
    let _pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let token = get_auth_token(&app, &pm_email).await;

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

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn response_includes_daily_rate_for_ctc_active(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );

    let app = xynergy_backend::create_app(pool.clone());

    let engineering_id = create_test_department(&pool, "Engineering").await;
    let dept_head_email = test_email();
    let dept_head_id = create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, engineering_id).await;

    let resource_id =
        create_test_resource_in_dept(&pool, "CTC Enabled Member", engineering_id).await;
    create_ctc_for_resource(&pool, resource_id, dept_head_id).await;

    let token = get_auth_token(&app, &dept_head_email).await;

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
        .expect("team response body should be readable");
    let body: Vec<Value> =
        serde_json::from_slice(&bytes).expect("team response should be an array payload");

    let resource = find_resource(&body, resource_id);
    assert!(
        !resource["daily_rate"].is_null(),
        "daily_rate should be present for active CTC"
    );
    assert_eq!(
        resource["ctc_status"]
            .as_str()
            .expect("ctc_status should be string"),
        "Active"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn response_shows_missing_for_no_ctc(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );

    let app = xynergy_backend::create_app(pool.clone());

    let engineering_id = create_test_department(&pool, "Engineering").await;
    let dept_head_email = test_email();
    let dept_head_id = create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, engineering_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "No CTC Member", engineering_id).await;
    let token = get_auth_token(&app, &dept_head_email).await;

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
        .expect("team response body should be readable");
    let body: Vec<Value> =
        serde_json::from_slice(&bytes).expect("team response should be an array payload");

    let resource = find_resource(&body, resource_id);
    assert!(
        resource["daily_rate"].is_null(),
        "daily_rate should be null when CTC is missing"
    );
    assert_eq!(
        resource["ctc_status"]
            .as_str()
            .expect("ctc_status should be string"),
        "Missing"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn response_does_not_contain_sensitive_ctc_fields(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );

    let app = xynergy_backend::create_app(pool.clone());

    let engineering_id = create_test_department(&pool, "Engineering").await;
    let dept_head_email = test_email();
    let dept_head_id = create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, engineering_id).await;

    let resource_id =
        create_test_resource_in_dept(&pool, "Security Test Member", engineering_id).await;
    create_ctc_for_resource(&pool, resource_id, dept_head_id).await;
    let token = get_auth_token(&app, &dept_head_email).await;

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
        .expect("team response body should be readable");
    let body = String::from_utf8(bytes.to_vec()).expect("response body should be utf8");

    let sensitive_fields = [
        "base_salary",
        "hra_allowance",
        "medical_allowance",
        "transport_allowance",
        "meal_allowance",
        "bpjs_kesehatan",
        "bpjs_ketenagakerjaan",
        "thr_monthly_accrual",
        "total_monthly_ctc",
        "encrypted_components",
    ];

    for field in sensitive_fields {
        assert!(
            !body.contains(field),
            "team response should not expose sensitive field: {}",
            field
        );
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn allocation_aggregation_correct(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );

    let app = xynergy_backend::create_app(pool.clone());

    let engineering_id = create_test_department(&pool, "Engineering").await;
    let dept_head_email = test_email();
    let dept_head_id = create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, engineering_id).await;

    let resource_id =
        create_test_resource_in_dept(&pool, "Allocation Member", engineering_id).await;
    create_ctc_for_resource(&pool, resource_id, dept_head_id).await;

    let project_a_id = create_test_project(&pool, "Project A").await;
    let project_b_id = create_test_project(&pool, "Project B").await;
    create_allocation(&pool, resource_id, project_a_id, 40.0).await;
    create_allocation(&pool, resource_id, project_b_id, 30.0).await;

    let token = get_auth_token(&app, &dept_head_email).await;

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
        .expect("team response body should be readable");
    let body: Vec<Value> =
        serde_json::from_slice(&bytes).expect("team response should be an array payload");

    let resource = find_resource(&body, resource_id);

    let total_allocation_pct = resource["total_allocation_pct"]
        .as_f64()
        .expect("total_allocation_pct should be number");
    assert!(
        (total_allocation_pct - 70.0).abs() < f64::EPSILON,
        "total allocation should be exactly 70.0"
    );

    let active_assignments = resource["active_assignments"]
        .as_array()
        .expect("active_assignments should be an array");
    assert_eq!(
        active_assignments.len(),
        2,
        "resource should have two active assignments"
    );

    let mut saw_project_a = false;
    let mut saw_project_b = false;

    for assignment in active_assignments {
        let project_name = assignment["project_name"]
            .as_str()
            .expect("project_name should be string");
        let allocation_pct = assignment["allocation_pct"]
            .as_f64()
            .expect("allocation_pct should be number");

        if project_name == "Project A" {
            saw_project_a = (allocation_pct - 40.0).abs() < f64::EPSILON;
        }
        if project_name == "Project B" {
            saw_project_b = (allocation_pct - 30.0).abs() < f64::EPSILON;
        }
    }

    assert!(saw_project_a, "Project A assignment should be 40%");
    assert!(saw_project_b, "Project B assignment should be 30%");
}

#[sqlx::test(migrations = "../../migrations")]
async fn ctc_guard_rejects_allocation_without_ctc(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );

    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let resource_id = create_test_resource_in_dept(&pool, "No CTC Resource", dept_id).await;
    let project_id = create_test_project(&pool, "Test Project").await;

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
                "start_date": "2026-04-01",
                "end_date": "2026-06-30",
                "allocation_percentage": 50.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("allocation request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("allocation request should return response");

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "allocation without CTC should be rejected with 400"
    );

    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body = String::from_utf8(bytes.to_vec()).expect("response body should be utf8");
    assert!(
        body.contains("CTC") || body.contains("ctc"),
        "rejection message should mention CTC"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn daily_rate_value_matches_encrypted_input(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );

    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let dept_head_email = test_email();
    let dept_head_id = create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, dept_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "Rate Check Member", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, dept_head_id).await;

    let token = get_auth_token(&app, &dept_head_email).await;

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
        .expect("team response body should be readable");
    let body: Vec<Value> =
        serde_json::from_slice(&bytes).expect("team response should be an array payload");

    let resource = find_resource(&body, resource_id);
    let daily_rate = resource["daily_rate"]
        .as_i64()
        .expect("daily_rate should be a number");
    assert_eq!(
        daily_rate, 1200000,
        "daily_rate should match the encrypted input value of 1200000"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn plaintext_daily_rate_fallback_works(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );

    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_test_department(&pool, "Engineering").await;
    let dept_head_email = test_email();
    let dept_head_id = create_test_user_with_role(&pool, &dept_head_email, "department_head").await;
    assign_user_to_department(&pool, dept_head_id, dept_id).await;

    let resource_id = create_test_resource_in_dept(&pool, "Plaintext Rate Member", dept_id).await;

    sqlx::query(
        "INSERT INTO ctc_records (resource_id, components, daily_rate, working_days_per_month, effective_date, status, created_by, created_at, updated_by, reason)
         VALUES ($1, '{}'::jsonb, 950000, 22, CURRENT_DATE, 'Active', $2, CURRENT_TIMESTAMP, $2, 'Test plaintext CTC')",
    )
    .bind(resource_id)
    .bind(dept_head_id)
    .execute(&pool)
    .await
    .expect("plaintext CTC record created");

    let token = get_auth_token(&app, &dept_head_email).await;

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
        .expect("team response body should be readable");
    let body: Vec<Value> =
        serde_json::from_slice(&bytes).expect("team response should be an array payload");

    let resource = find_resource(&body, resource_id);
    let daily_rate = resource["daily_rate"]
        .as_i64()
        .expect("daily_rate should be a number");
    assert_eq!(
        daily_rate, 950000,
        "plaintext daily_rate fallback should return 950000"
    );
    assert_eq!(
        resource["ctc_status"]
            .as_str()
            .expect("ctc_status should be string"),
        "Active"
    );
}
