use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("cashflow-{}@example.com", Uuid::new_v4())
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

async fn create_test_project(pool: &PgPool, name: &str, pm_id: Uuid) -> Uuid {
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

async fn create_cash_flow_via_api(
    app: &axum::Router,
    token: &str,
    payload: &Value,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/cash-flow/entries")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
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

async fn create_cash_flow_status_only(
    app: &axum::Router,
    token: &str,
    payload: &Value,
) -> StatusCode {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/cash-flow/entries")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .expect("request should be built");

    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("should return response");
    resp.status()
}

async fn list_cash_flow_via_api(
    app: &axum::Router,
    token: &str,
    query: &str,
) -> (StatusCode, Value) {
    let uri = if query.is_empty() {
        "/api/v1/cash-flow/entries".to_string()
    } else {
        format!("/api/v1/cash-flow/entries?{}", query)
    };

    let req = Request::builder()
        .method("GET")
        .uri(uri)
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

async fn list_project_cash_flow_via_api(
    app: &axum::Router,
    token: &str,
    project_id: Uuid,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}/cash-flow/entries", project_id))
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

// ── Role Access Tests ──────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn finance_can_create_cash_in_entry(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 5_000_000,
        "entry_date": "2026-03-01",
        "description": "Payment from client ABC"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["id"].as_str().is_some());
    assert_eq!(body["entry_type"].as_str().unwrap(), "cash_in");
    assert_eq!(body["category"].as_str().unwrap(), "client_payment");
    assert_eq!(body["amount_idr"].as_i64().unwrap(), 5_000_000);
    assert_eq!(
        body["description"].as_str().unwrap(),
        "Payment from client ABC"
    );
    assert!(body["project_id"].is_null());
}

#[sqlx::test(migrations = "../../migrations")]
async fn finance_can_create_cash_out_entry(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_out",
        "category": "payroll",
        "amount_idr": 15_000_000,
        "entry_date": "2026-03-05",
        "description": "March payroll run"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["entry_type"].as_str().unwrap(), "cash_out");
    assert_eq!(body["category"].as_str().unwrap(), "payroll");
    assert_eq!(body["amount_idr"].as_i64().unwrap(), 15_000_000);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_create_cash_flow_entry(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "admin").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "interest",
        "amount_idr": 250_000,
        "entry_date": "2026-03-10",
        "description": "Bank interest income"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["category"].as_str().unwrap(), "interest");
}

#[sqlx::test(migrations = "../../migrations")]
async fn project_manager_denied_create_cash_flow(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "project_manager").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 1_000_000,
        "entry_date": "2026-03-01",
        "description": "Should be denied"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "FORBIDDEN_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_denied_create_cash_flow(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "hr").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_out",
        "category": "payroll",
        "amount_idr": 500_000,
        "entry_date": "2026-03-01",
        "description": "Should be denied"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "FORBIDDEN_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn department_head_denied_create_cash_flow(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "interest",
        "amount_idr": 100_000,
        "entry_date": "2026-03-01",
        "description": "Should be denied"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "FORBIDDEN_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn finance_denied_list_returns_403(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "project_manager").await;
    let token = get_auth_token(&app, &email).await;

    let (status, body) = list_cash_flow_via_api(&app, &token, "").await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "FORBIDDEN_ERROR");
}

// ── Validation Tests ───────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_entry_type_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "invalid_type",
        "category": "client_payment",
        "amount_idr": 1_000_000,
        "entry_date": "2026-03-01",
        "description": "Should fail"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_category_for_cash_in_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "payroll",
        "amount_idr": 1_000_000,
        "entry_date": "2026-03-01",
        "description": "Payroll is not valid for cash_in"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_category_for_cash_out_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_out",
        "category": "client_payment",
        "amount_idr": 500_000,
        "entry_date": "2026-03-01",
        "description": "client_payment is not valid for cash_out"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn negative_amount_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": -100,
        "entry_date": "2026-03-01",
        "description": "Negative amount"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn zero_amount_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 0,
        "entry_date": "2026-03-01",
        "description": "Zero amount"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn empty_description_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 1_000_000,
        "entry_date": "2026-03-01",
        "description": "   "
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn missing_entry_type_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "category": "client_payment",
        "amount_idr": 1_000_000,
        "entry_date": "2026-03-01",
        "description": "Missing entry_type"
    });

    let status = create_cash_flow_status_only(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "../../migrations")]
async fn missing_amount_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "entry_date": "2026-03-01",
        "description": "Missing amount_idr"
    });

    let status = create_cash_flow_status_only(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "../../migrations")]
async fn missing_description_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 1_000_000,
        "entry_date": "2026-03-01"
    });

    let status = create_cash_flow_status_only(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "../../migrations")]
async fn nonexistent_project_id_returns_404(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let fake_project_id = Uuid::new_v4();
    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 1_000_000,
        "entry_date": "2026-03-01",
        "description": "Linked to non-existent project",
        "project_id": fake_project_id
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "NOT_FOUND");
}

// ── Category Matrix Tests ──────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn all_cash_in_categories_accepted(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    for category in &["client_payment", "interest", "other_income"] {
        let payload = json!({
            "entry_type": "cash_in",
            "category": category,
            "amount_idr": 1_000_000,
            "entry_date": "2026-03-01",
            "description": format!("Testing cash_in category: {}", category)
        });

        let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "cash_in category '{}' should be accepted, got: {:?}",
            category,
            body
        );
        assert_eq!(body["category"].as_str().unwrap(), *category);
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn all_cash_out_categories_accepted(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    for category in &["payroll", "vendor_payment", "expense", "tax"] {
        let payload = json!({
            "entry_type": "cash_out",
            "category": category,
            "amount_idr": 2_000_000,
            "entry_date": "2026-03-01",
            "description": format!("Testing cash_out category: {}", category)
        });

        let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "cash_out category '{}' should be accepted, got: {:?}",
            category,
            body
        );
        assert_eq!(body["category"].as_str().unwrap(), *category);
    }
}

// ── Project Link Tests ─────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn entry_with_project_id_appears_in_project_cash_flow(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let finance_email = test_email();
    let pm_email = test_email();
    let finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project(&pool, "CashFlow Project Link", pm_id).await;
    let token = get_auth_token(&app, &finance_email).await;

    // Create entry linked to project
    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 10_000_000,
        "entry_date": "2026-03-15",
        "description": "Linked client payment",
        "project_id": project_id
    });

    let (create_status, create_body) = create_cash_flow_via_api(&app, &token, &payload).await;
    assert_eq!(create_status, StatusCode::OK);
    assert_eq!(
        create_body["project_id"].as_str().unwrap(),
        project_id.to_string()
    );

    // Create entry NOT linked to project
    let unlinked_payload = json!({
        "entry_type": "cash_out",
        "category": "expense",
        "amount_idr": 500_000,
        "entry_date": "2026-03-16",
        "description": "Unlinked expense"
    });
    let (unlinked_status, _) = create_cash_flow_via_api(&app, &token, &unlinked_payload).await;
    assert_eq!(unlinked_status, StatusCode::OK);

    // Verify project-linked endpoint only returns linked entries
    let (list_status, list_body) = list_project_cash_flow_via_api(&app, &token, project_id).await;
    assert_eq!(list_status, StatusCode::OK);
    let entries = list_body.as_array().unwrap();
    assert_eq!(entries.len(), 1, "Only the linked entry should appear");
    assert_eq!(
        entries[0]["description"].as_str().unwrap(),
        "Linked client payment"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn project_cash_flow_for_nonexistent_project_returns_404(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let fake_project_id = Uuid::new_v4();
    let (status, body) = list_project_cash_flow_via_api(&app, &token, fake_project_id).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "NOT_FOUND");
}

// ── List and Filter Tests ──────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn list_entries_returns_all_created(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    // Create two entries
    let payload1 = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 3_000_000,
        "entry_date": "2026-03-01",
        "description": "First entry"
    });
    let payload2 = json!({
        "entry_type": "cash_out",
        "category": "tax",
        "amount_idr": 750_000,
        "entry_date": "2026-03-02",
        "description": "Second entry"
    });

    let (s1, _) = create_cash_flow_via_api(&app, &token, &payload1).await;
    assert_eq!(s1, StatusCode::OK);
    let (s2, _) = create_cash_flow_via_api(&app, &token, &payload2).await;
    assert_eq!(s2, StatusCode::OK);

    let (status, body) = list_cash_flow_via_api(&app, &token, "").await;
    assert_eq!(status, StatusCode::OK);
    let entries = body.as_array().unwrap();
    assert!(entries.len() >= 2, "Should have at least 2 entries");
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_entries_filter_by_entry_type(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let in_payload = json!({
        "entry_type": "cash_in",
        "category": "interest",
        "amount_idr": 100_000,
        "entry_date": "2026-03-01",
        "description": "Interest income"
    });
    let out_payload = json!({
        "entry_type": "cash_out",
        "category": "vendor_payment",
        "amount_idr": 200_000,
        "entry_date": "2026-03-01",
        "description": "Vendor payment"
    });

    let (s1, _) = create_cash_flow_via_api(&app, &token, &in_payload).await;
    assert_eq!(s1, StatusCode::OK);
    let (s2, _) = create_cash_flow_via_api(&app, &token, &out_payload).await;
    assert_eq!(s2, StatusCode::OK);

    // Filter for cash_in only
    let (status, body) = list_cash_flow_via_api(&app, &token, "entry_type=cash_in").await;
    assert_eq!(status, StatusCode::OK);
    let entries = body.as_array().unwrap();
    assert_eq!(
        entries.len(),
        1,
        "Only the seeded cash_in entry should match"
    );
    assert_eq!(
        entries[0]["description"].as_str().unwrap(),
        "Interest income"
    );
    assert!(
        entries
            .iter()
            .all(|e| e["entry_type"].as_str().unwrap() == "cash_in"),
        "All filtered entries should be cash_in"
    );
}

// ── Audit Logging Tests ────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn create_entry_produces_audit_log(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "other_income",
        "amount_idr": 1_500_000,
        "entry_date": "2026-03-20",
        "description": "Audit test entry"
    });

    let (status, body) = create_cash_flow_via_api(&app, &token, &payload).await;
    assert_eq!(status, StatusCode::OK);
    let entry_id = body["id"].as_str().unwrap();
    let entry_uuid = Uuid::parse_str(entry_id).expect("valid UUID");

    // Check audit log entry
    let audit_row = sqlx::query!(
        "SELECT action, entity_type, entity_id, user_id FROM audit_logs WHERE entity_type = 'cash_flow_entry' AND entity_id = $1 ORDER BY created_at DESC LIMIT 1",
        entry_uuid
    )
    .fetch_optional(&pool)
    .await
    .expect("audit query should succeed");

    assert!(audit_row.is_some(), "Audit log should exist for create");
    let audit = audit_row.unwrap();
    assert_eq!(audit.action, "create");
    assert_eq!(audit.entity_type, "cash_flow_entry");
    assert_eq!(audit.entity_id, Some(entry_uuid));
    assert_eq!(audit.user_id, Some(user_id));
}

#[sqlx::test(migrations = "../../migrations")]
async fn denied_access_produces_audit_log(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "project_manager").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 1_000_000,
        "entry_date": "2026-03-01",
        "description": "Should be denied"
    });

    let (status, _) = create_cash_flow_via_api(&app, &token, &payload).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Check ACCESS_DENIED audit log
    let audit_row = sqlx::query!(
        "SELECT action, entity_type, user_id FROM audit_logs WHERE action = 'ACCESS_DENIED' AND entity_type = 'cash_flow_entry' AND user_id = $1 ORDER BY created_at DESC LIMIT 1",
        user_id
    )
    .fetch_optional(&pool)
    .await
    .expect("audit query should succeed");

    assert!(audit_row.is_some(), "ACCESS_DENIED audit log should exist");
    let audit = audit_row.unwrap();
    assert_eq!(audit.action, "ACCESS_DENIED");
    assert_eq!(audit.entity_type, "cash_flow_entry");
}

// ── Status Code Contract Tests ─────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn status_code_200_on_success(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 1_000_000,
        "entry_date": "2026-03-01",
        "description": "Status 200 test"
    });

    let (status, _) = create_cash_flow_via_api(&app, &token, &payload).await;
    assert_eq!(status, StatusCode::OK);

    let (list_status, _) = list_cash_flow_via_api(&app, &token, "").await;
    assert_eq!(list_status, StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn status_code_400_on_validation(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "bad_type",
        "category": "client_payment",
        "amount_idr": 1_000_000,
        "entry_date": "2026-03-01",
        "description": "Validation test"
    });

    let (status, _) = create_cash_flow_via_api(&app, &token, &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn status_code_403_on_forbidden(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "hr").await;
    let token = get_auth_token(&app, &email).await;

    let payload = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 1_000_000,
        "entry_date": "2026-03-01",
        "description": "Forbidden test"
    });

    let (status, _) = create_cash_flow_via_api(&app, &token, &payload).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn status_code_404_on_not_found(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let fake_project_id = Uuid::new_v4();
    let (status, _) = list_project_cash_flow_via_api(&app, &token, fake_project_id).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── Date Range Filter Tests ───────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn list_entries_filter_by_date_range(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    // Create entries on different dates
    let early = json!({
        "entry_type": "cash_in",
        "category": "client_payment",
        "amount_idr": 1_000_000,
        "entry_date": "2026-01-15",
        "description": "January entry"
    });
    let mid = json!({
        "entry_type": "cash_out",
        "category": "payroll",
        "amount_idr": 2_000_000,
        "entry_date": "2026-02-15",
        "description": "February entry"
    });
    let late = json!({
        "entry_type": "cash_in",
        "category": "interest",
        "amount_idr": 500_000,
        "entry_date": "2026-03-15",
        "description": "March entry"
    });

    let (s1, _) = create_cash_flow_via_api(&app, &token, &early).await;
    assert_eq!(s1, StatusCode::OK);
    let (s2, _) = create_cash_flow_via_api(&app, &token, &mid).await;
    assert_eq!(s2, StatusCode::OK);
    let (s3, _) = create_cash_flow_via_api(&app, &token, &late).await;
    assert_eq!(s3, StatusCode::OK);

    // Filter by date range (February only)
    let (status, body) =
        list_cash_flow_via_api(&app, &token, "start_date=2026-02-01&end_date=2026-02-28").await;
    assert_eq!(status, StatusCode::OK);
    let entries = body.as_array().unwrap();
    assert_eq!(entries.len(), 1, "Only the February entry should match");
    assert_eq!(
        entries[0]["description"].as_str().unwrap(),
        "February entry"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_entries_invalid_entry_type_filter_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let (status, body) = list_cash_flow_via_api(&app, &token, "entry_type=invalid_type").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
}
