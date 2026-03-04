use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("projexpense-{}@example.com", Uuid::new_v4())
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

/// Helper: create an expense via API and return the response body
async fn create_expense_via_api(
    app: &axum::Router,
    token: &str,
    project_id: Uuid,
    category: &str,
    description: &str,
    amount_idr: i64,
    expense_date: &str,
    vendor: Option<&str>,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/expenses", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "category": category,
                "description": description,
                "amount_idr": amount_idr,
                "expense_date": expense_date,
                "vendor": vendor,
            })
            .to_string(),
        ))
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

// ── Test 1: PM can create expense on own project ──────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn pm_can_create_expense_on_own_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "PM Expense Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let (status, body) = create_expense_via_api(
        &app,
        &token,
        project_id,
        "software",
        "IDE License",
        500000,
        "2026-03-01",
        Some("JetBrains"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["category"].as_str().unwrap(), "software");
    assert_eq!(body["description"].as_str().unwrap(), "IDE License");
    assert_eq!(body["amount_idr"].as_i64().unwrap(), 500000);
    assert_eq!(body["expense_date"].as_str().unwrap(), "2026-03-01");
    assert_eq!(body["vendor"].as_str().unwrap(), "JetBrains");
    assert!(body["id"].as_str().is_some());
    assert_eq!(
        body["project_id"].as_str().unwrap(),
        project_id.to_string()
    );
}

// ── Test 2: PM denied on non-owned project ────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn pm_denied_expense_on_non_owned_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm1_email = test_email();
    let pm2_email = test_email();
    let pm1_id = create_test_user_with_role(&pool, &pm1_email, "project_manager").await;
    let pm2_id = create_test_user_with_role(&pool, &pm2_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "PM1 Project", pm1_id).await;
    let token_pm2 = get_auth_token(&app, &pm2_email).await;

    let (status, body) = create_expense_via_api(
        &app,
        &token_pm2,
        project_id,
        "hr",
        "Unauthorized expense",
        100000,
        "2026-03-01",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        body["error"]["code"].as_str().unwrap(),
        "FORBIDDEN_ERROR"
    );

    let audit_entry = sqlx::query!(
        "SELECT action, entity_type, entity_id, user_id FROM audit_logs WHERE action = 'ACCESS_DENIED' AND entity_type = 'project_expense' AND entity_id = $1 AND user_id = $2 ORDER BY created_at DESC LIMIT 1",
        project_id,
        pm2_id
    )
    .fetch_optional(&pool)
    .await
    .expect("audit query should succeed");

    assert!(
        audit_entry.is_some(),
        "access denied audit should be logged for non-owned PM"
    );
    let entry = audit_entry.unwrap();
    assert_eq!(entry.action, "ACCESS_DENIED");
    assert_eq!(entry.entity_type, "project_expense");
    assert_eq!(entry.entity_id, Some(project_id));
    assert_eq!(entry.user_id, Some(pm2_id));
}

// ── Test 3: Admin can create expense on any project ───────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_create_expense_on_any_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let admin_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let project_id = create_test_project_with_pm(&pool, "PM Project For Admin", pm_id).await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    let (status, body) = create_expense_via_api(
        &app,
        &admin_token,
        project_id,
        "overhead",
        "Office rent share",
        2000000,
        "2026-03-15",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["category"].as_str().unwrap(), "overhead");
    assert_eq!(body["amount_idr"].as_i64().unwrap(), 2000000);
}

// ── Test 4: Non-PM/non-admin role denied ──────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn non_pm_non_admin_denied_expense(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let hr_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let project_id = create_test_project_with_pm(&pool, "HR Denied Project", pm_id).await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let (status, body) = create_expense_via_api(
        &app,
        &hr_token,
        project_id,
        "hr",
        "Should fail",
        100000,
        "2026-03-01",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        body["error"]["code"].as_str().unwrap(),
        "FORBIDDEN_ERROR"
    );
}

// ── Test 5: Invalid category rejected ─────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_category_rejected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Category Test Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let (status, body) = create_expense_via_api(
        &app,
        &token,
        project_id,
        "marketing", // Invalid category
        "Marketing campaign",
        100000,
        "2026-03-01",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["error"]["code"].as_str().unwrap(),
        "VALIDATION_ERROR"
    );
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Invalid category")
    );
}

// ── Test 6: Zero and negative amount rejected ─────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn zero_and_negative_amount_rejected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Amount Test Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // Zero amount
    let (status, body) = create_expense_via_api(
        &app,
        &token,
        project_id,
        "hr",
        "Zero expense",
        0,
        "2026-03-01",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["error"]["code"].as_str().unwrap(),
        "VALIDATION_ERROR"
    );
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("positive integer")
    );

    // Negative amount
    let (status2, body2) = create_expense_via_api(
        &app,
        &token,
        project_id,
        "hr",
        "Negative expense",
        -500,
        "2026-03-01",
        None,
    )
    .await;

    assert_eq!(status2, StatusCode::BAD_REQUEST);
    assert_eq!(
        body2["error"]["code"].as_str().unwrap(),
        "VALIDATION_ERROR"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn decimal_amount_payload_rejected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Decimal Amount Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/expenses", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "category": "hr",
                "description": "Decimal amount attempt",
                "amount_idr": 1000.50,
                "expense_date": "2026-03-01",
                "vendor": null
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
        "expected 400 or 422 for decimal payload, got {}",
        resp.status()
    );
}

// ── Test 7: Update requires edit_reason ───────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn update_requires_edit_reason(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Edit Reason Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // Create an expense first
    let (status, create_body) = create_expense_via_api(
        &app,
        &token,
        project_id,
        "software",
        "License",
        500000,
        "2026-03-01",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let expense_id: Uuid = create_body["id"]
        .as_str()
        .unwrap()
        .parse()
        .expect("valid UUID");

    // Update with empty edit_reason — should fail
    let req = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/v1/projects/{}/expenses/{}",
            project_id, expense_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "amount_idr": 600000,
                "edit_reason": ""
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
        .expect("readable response body");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON response");
    assert_eq!(
        body["error"]["code"].as_str().unwrap(),
        "VALIDATION_ERROR"
    );
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("edit_reason")
    );

    // Update with valid edit_reason — should succeed
    let req2 = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/v1/projects/{}/expenses/{}",
            project_id, expense_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "amount_idr": 600000,
                "edit_reason": "Price increased"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let resp2 = app
        .clone()
        .oneshot(req2)
        .await
        .expect("should return response");
    assert_eq!(resp2.status(), StatusCode::OK);
    let bytes2 = to_bytes(resp2.into_body(), usize::MAX)
        .await
        .expect("readable response body");
    let body2: Value = serde_json::from_slice(&bytes2).expect("valid JSON response");
    assert_eq!(body2["amount_idr"].as_i64().unwrap(), 600000);

    let update_audit = sqlx::query!(
        "SELECT action, entity_type, entity_id, user_id, changes->'after'->>'edit_reason' as \"edit_reason?\" FROM audit_logs WHERE action = 'update' AND entity_type = 'project_expense' AND entity_id = $1 ORDER BY created_at DESC LIMIT 1",
        expense_id
    )
    .fetch_optional(&pool)
    .await
    .expect("audit query should succeed");

    assert!(
        update_audit.is_some(),
        "update audit log entry should exist for expense update"
    );
    let update_entry = update_audit.unwrap();
    assert_eq!(update_entry.action, "update");
    assert_eq!(update_entry.entity_type, "project_expense");
    assert_eq!(update_entry.entity_id, Some(expense_id));
    assert_eq!(update_entry.user_id, Some(pm_id));
    assert_eq!(
        update_entry.edit_reason,
        Some("Price increased".to_string())
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_can_clear_vendor_field(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Vendor Clear Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let (status, create_body) = create_expense_via_api(
        &app,
        &token,
        project_id,
        "software",
        "Vendor clear test",
        450000,
        "2026-03-01",
        Some("Original Vendor"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let expense_id: Uuid = create_body["id"]
        .as_str()
        .unwrap()
        .parse()
        .expect("valid UUID");

    let req = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/v1/projects/{}/expenses/{}",
            project_id, expense_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "vendor": "",
                "edit_reason": "Vendor removed"
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
    assert!(body["vendor"].is_null(), "vendor should be cleared");
}

// ── Test 8: Expense appears in list ───────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn expense_appears_in_list(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "List Test Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // Create two expenses
    let (_s1, _b1) = create_expense_via_api(
        &app,
        &token,
        project_id,
        "hr",
        "First Expense",
        100000,
        "2026-03-01",
        None,
    )
    .await;

    let (_s2, _b2) = create_expense_via_api(
        &app,
        &token,
        project_id,
        "software",
        "Second Expense",
        200000,
        "2026-03-02",
        Some("Vendor A"),
    )
    .await;

    // List expenses
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/expenses", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
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
    let expenses = body.as_array().expect("response should be array");

    assert_eq!(expenses.len(), 2);
    // Should be ordered by expense_date DESC
    assert_eq!(
        expenses[0]["expense_date"].as_str().unwrap(),
        "2026-03-02"
    );
    assert_eq!(
        expenses[1]["expense_date"].as_str().unwrap(),
        "2026-03-01"
    );
}

// ── Test 9: Delete removes expense ────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn delete_removes_expense(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Delete Test Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // Create expense
    let (status, create_body) = create_expense_via_api(
        &app,
        &token,
        project_id,
        "hardware",
        "Monitor",
        3000000,
        "2026-03-01",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let expense_id = create_body["id"].as_str().unwrap();

    // Delete it
    let req = Request::builder()
        .method("DELETE")
        .uri(format!(
            "/api/v1/projects/{}/expenses/{}",
            project_id, expense_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    // List expenses — should be empty
    let req2 = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/expenses", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp2 = app
        .clone()
        .oneshot(req2)
        .await
        .expect("should return response");
    assert_eq!(resp2.status(), StatusCode::OK);

    let bytes2 = to_bytes(resp2.into_body(), usize::MAX)
        .await
        .expect("readable response body");
    let body2: Value = serde_json::from_slice(&bytes2).expect("valid JSON response");
    let expenses = body2.as_array().expect("response should be array");
    assert_eq!(expenses.len(), 0);
}

// ── Test 10: Budget reflects expense sum ──────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn budget_reflects_expense_sum(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Budget Sum Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // Set a budget first
    let budget_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "total_budget_idr": 10000000_i64,
                "budget_hr_idr": 4000000_i64,
                "budget_software_idr": 3000000_i64,
                "budget_hardware_idr": 2000000_i64,
                "budget_overhead_idr": 1000000_i64,
            })
            .to_string(),
        ))
        .expect("request should be built");
    let budget_resp = app
        .clone()
        .oneshot(budget_req)
        .await
        .expect("should return response");
    assert_eq!(budget_resp.status(), StatusCode::OK);

    // Create two expenses: 500000 + 300000 = 800000
    create_expense_via_api(
        &app,
        &token,
        project_id,
        "hr",
        "Expense A",
        500000,
        "2026-03-01",
        None,
    )
    .await;
    create_expense_via_api(
        &app,
        &token,
        project_id,
        "software",
        "Expense B",
        300000,
        "2026-03-02",
        None,
    )
    .await;

    // Fetch budget — spent_to_date_idr should be 800000
    let get_budget_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/budget", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let get_budget_resp = app
        .clone()
        .oneshot(get_budget_req)
        .await
        .expect("should return response");
    assert_eq!(get_budget_resp.status(), StatusCode::OK);

    let bytes = to_bytes(get_budget_resp.into_body(), usize::MAX)
        .await
        .expect("readable response body");
    let body: Value = serde_json::from_slice(&bytes).expect("valid JSON response");

    assert_eq!(
        body["spent_to_date_idr"].as_i64().unwrap(),
        800000,
        "spent_to_date_idr should be sum of expenses"
    );
    assert_eq!(
        body["remaining_idr"].as_i64().unwrap(),
        10000000 - 800000,
        "remaining_idr should be total minus spent"
    );
}

// ── Test 11: Audit log created on expense creation ────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn audit_log_created_on_expense_create(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Audit Log Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let (status, create_body) = create_expense_via_api(
        &app,
        &token,
        project_id,
        "overhead",
        "Utilities",
        750000,
        "2026-03-10",
        Some("PLN"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let expense_id_str = create_body["id"].as_str().unwrap();
    let expense_id: Uuid = expense_id_str.parse().expect("valid UUID");

    // Check audit log entry exists
    let audit_entry = sqlx::query!(
        "SELECT action, entity_type, entity_id, user_id FROM audit_logs WHERE entity_id = $1 AND entity_type = 'project_expense' AND action = 'create' ORDER BY created_at DESC LIMIT 1",
        expense_id
    )
    .fetch_optional(&pool)
    .await
    .expect("audit query should succeed");

    assert!(
        audit_entry.is_some(),
        "audit log entry should exist for expense creation"
    );
    let entry = audit_entry.unwrap();
    assert_eq!(entry.action, "create");
    assert_eq!(entry.entity_type, "project_expense");
    assert_eq!(entry.entity_id, Some(expense_id));
    assert_eq!(entry.user_id, Some(pm_id));
}

// ── Test 12: Audit log created on expense deletion ────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn audit_log_created_on_expense_delete(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Delete Audit Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    // Create and delete an expense
    let (status, create_body) = create_expense_via_api(
        &app,
        &token,
        project_id,
        "hardware",
        "Keyboard",
        250000,
        "2026-03-05",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let expense_id_str = create_body["id"].as_str().unwrap();
    let expense_id: Uuid = expense_id_str.parse().expect("valid UUID");

    let req = Request::builder()
        .method("DELETE")
        .uri(format!(
            "/api/v1/projects/{}/expenses/{}",
            project_id, expense_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    assert_eq!(resp.status(), StatusCode::OK);

    // Check audit log for expense_deleted action
    let audit_entry = sqlx::query!(
        "SELECT action, entity_type, entity_id FROM audit_logs WHERE entity_id = $1 AND entity_type = 'project_expense' AND action = 'expense_deleted' LIMIT 1",
        expense_id
    )
    .fetch_optional(&pool)
    .await
    .expect("audit query should succeed");

    assert!(
        audit_entry.is_some(),
        "audit log entry should exist for expense deletion"
    );
    let entry = audit_entry.unwrap();
    assert_eq!(entry.action, "expense_deleted");
}
