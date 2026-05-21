use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("cfdash-{}@example.com", Uuid::new_v4())
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

async fn insert_cash_flow_entry(
    pool: &PgPool,
    user_id: Uuid,
    entry_type: &str,
    category: &str,
    amount_idr: i64,
    entry_date: &str,
    description: &str,
    project_id: Option<Uuid>,
) {
    sqlx::query(
        "INSERT INTO cash_flow_entries (entry_type, category, amount_idr, entry_date, description, project_id, created_by)
         VALUES ($1, $2, $3, $4::DATE, $5, $6, $7)",
    )
    .bind(entry_type)
    .bind(category)
    .bind(amount_idr)
    .bind(entry_date)
    .bind(description)
    .bind(project_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("cash flow entry inserted");
}

async fn get_dashboard(app: &axum::Router, token: &str, query: &str) -> (StatusCode, Value) {
    let uri = if query.is_empty() {
        "/api/v1/cash-flow/dashboard".to_string()
    } else {
        format!("/api/v1/cash-flow/dashboard?{}", query)
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

// ── Role Access Tests ──────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn finance_can_fetch_dashboard(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let (status, body) =
        get_dashboard(&app, &token, "start_date=2026-01-01&end_date=2026-12-31").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["months"].is_array());
    assert_eq!(body["months"].as_array().unwrap().len(), 12);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_fetch_dashboard(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "admin").await;
    let token = get_auth_token(&app, &email).await;

    let (status, body) =
        get_dashboard(&app, &token, "start_date=2026-01-01&end_date=2026-06-30").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["months"].as_array().unwrap().len(), 6);
}

#[sqlx::test(migrations = "../../migrations")]
async fn project_manager_denied_dashboard(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "project_manager").await;
    let token = get_auth_token(&app, &email).await;

    let (status, body) = get_dashboard(&app, &token, "").await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "FORBIDDEN_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_denied_dashboard(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "hr").await;
    let token = get_auth_token(&app, &email).await;

    let (status, body) = get_dashboard(&app, &token, "").await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "FORBIDDEN_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn department_head_denied_dashboard(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "department_head").await;
    let token = get_auth_token(&app, &email).await;

    let (status, body) = get_dashboard(&app, &token, "").await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "FORBIDDEN_ERROR");
}

// ── Validation Tests ───────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_date_range_returns_400(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let (status, body) =
        get_dashboard(&app, &token, "start_date=2026-12-01&end_date=2026-01-01").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn nonexistent_project_id_returns_404(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let fake_id = Uuid::new_v4();
    let (status, body) = get_dashboard(
        &app,
        &token,
        &format!(
            "start_date=2026-01-01&end_date=2026-12-31&project_id={}",
            fake_id
        ),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "NOT_FOUND");
}

// ── Aggregation and Formula Tests ──────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn monthly_aggregation_returns_dense_month_buckets(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_in",
        "client_payment",
        10_000_000,
        "2026-01-15",
        "Jan payment",
        None,
    )
    .await;
    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_out",
        "payroll",
        3_000_000,
        "2026-03-10",
        "Mar payroll",
        None,
    )
    .await;

    let (status, body) =
        get_dashboard(&app, &token, "start_date=2026-01-01&end_date=2026-06-30").await;

    assert_eq!(status, StatusCode::OK);
    let months = body["months"].as_array().unwrap();
    assert_eq!(months.len(), 6, "Dense: Jan through Jun = 6 months");

    assert_eq!(months[0]["month_label"].as_str().unwrap(), "Jan");
    assert_eq!(months[0]["cash_in_idr"].as_i64().unwrap(), 10_000_000);
    assert_eq!(months[0]["cash_out_idr"].as_i64().unwrap(), 0);

    assert_eq!(months[1]["month_label"].as_str().unwrap(), "Feb");
    assert_eq!(months[1]["cash_in_idr"].as_i64().unwrap(), 0);
    assert_eq!(months[1]["cash_out_idr"].as_i64().unwrap(), 0);

    assert_eq!(months[2]["month_label"].as_str().unwrap(), "Mar");
    assert_eq!(months[2]["cash_out_idr"].as_i64().unwrap(), 3_000_000);
}

#[sqlx::test(migrations = "../../migrations")]
async fn partial_date_ranges_normalize_to_full_month_boundaries(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_in",
        "client_payment",
        4_000_000,
        "2026-01-01",
        "Early January payment",
        None,
    )
    .await;
    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_out",
        "payroll",
        1_500_000,
        "2026-03-31",
        "Late March payroll",
        None,
    )
    .await;

    let (status, body) =
        get_dashboard(&app, &token, "start_date=2026-01-15&end_date=2026-03-14").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["start_date"].as_str().unwrap(), "2026-01-01");
    assert_eq!(body["end_date"].as_str().unwrap(), "2026-03-31");

    let months = body["months"].as_array().unwrap();
    assert_eq!(months.len(), 3);
    assert_eq!(months[0]["cash_in_idr"].as_i64().unwrap(), 4_000_000);
    assert_eq!(months[2]["cash_out_idr"].as_i64().unwrap(), 1_500_000);
}

#[sqlx::test(migrations = "../../migrations")]
async fn net_cash_flow_equals_cash_in_minus_cash_out(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_in",
        "client_payment",
        20_000_000,
        "2026-02-10",
        "Feb in",
        None,
    )
    .await;
    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_out",
        "vendor_payment",
        8_000_000,
        "2026-02-15",
        "Feb out",
        None,
    )
    .await;
    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_out",
        "expense",
        2_000_000,
        "2026-02-20",
        "Feb expense",
        None,
    )
    .await;

    let (status, body) =
        get_dashboard(&app, &token, "start_date=2026-02-01&end_date=2026-02-28").await;

    assert_eq!(status, StatusCode::OK);

    let total_in = body["total_cash_in_idr"].as_i64().unwrap();
    let total_out = body["total_cash_out_idr"].as_i64().unwrap();
    let net = body["net_cash_flow_idr"].as_i64().unwrap();

    assert_eq!(total_in, 20_000_000);
    assert_eq!(total_out, 10_000_000);
    assert_eq!(net, 10_000_000, "net = cash_in - cash_out");

    let month = &body["months"].as_array().unwrap()[0];
    assert_eq!(month["net_cash_flow_idr"].as_i64().unwrap(), 10_000_000);
}

#[sqlx::test(migrations = "../../migrations")]
async fn cumulative_position_rolls_forward_month_to_month(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_in",
        "client_payment",
        10_000_000,
        "2026-01-15",
        "Jan in",
        None,
    )
    .await;
    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_out",
        "payroll",
        4_000_000,
        "2026-02-15",
        "Feb out",
        None,
    )
    .await;
    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_in",
        "interest",
        1_000_000,
        "2026-03-15",
        "Mar in",
        None,
    )
    .await;

    let (status, body) =
        get_dashboard(&app, &token, "start_date=2026-01-01&end_date=2026-03-31").await;

    assert_eq!(status, StatusCode::OK);
    let months = body["months"].as_array().unwrap();

    // Jan: cum = +10M
    assert_eq!(
        months[0]["cumulative_position_idr"].as_i64().unwrap(),
        10_000_000
    );
    // Feb: cum = 10M - 4M = 6M
    assert_eq!(
        months[1]["cumulative_position_idr"].as_i64().unwrap(),
        6_000_000
    );
    // Mar: cum = 6M + 1M = 7M
    assert_eq!(
        months[2]["cumulative_position_idr"].as_i64().unwrap(),
        7_000_000
    );

    assert_eq!(
        body["ending_cumulative_position_idr"].as_i64().unwrap(),
        7_000_000
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn months_with_no_entries_return_zeros(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let (status, body) =
        get_dashboard(&app, &token, "start_date=2026-07-01&end_date=2026-09-30").await;

    assert_eq!(status, StatusCode::OK);
    let months = body["months"].as_array().unwrap();
    assert_eq!(months.len(), 3);

    for m in months {
        assert_eq!(m["cash_in_idr"].as_i64().unwrap(), 0);
        assert_eq!(m["cash_out_idr"].as_i64().unwrap(), 0);
        assert_eq!(m["net_cash_flow_idr"].as_i64().unwrap(), 0);
        assert_eq!(m["cumulative_position_idr"].as_i64().unwrap(), 0);
    }

    assert_eq!(body["total_cash_in_idr"].as_i64().unwrap(), 0);
    assert_eq!(body["total_cash_out_idr"].as_i64().unwrap(), 0);
    assert_eq!(body["net_cash_flow_idr"].as_i64().unwrap(), 0);
    assert_eq!(body["ending_cumulative_position_idr"].as_i64().unwrap(), 0);
}

// ── Project Filter Tests ───────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn project_filter_limits_summary_and_drilldown(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let finance_email = test_email();
    let pm_email = test_email();
    let finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project(&pool, "Dashboard Filter Test", pm_id).await;
    let token = get_auth_token(&app, &finance_email).await;

    insert_cash_flow_entry(
        &pool,
        finance_id,
        "cash_in",
        "client_payment",
        5_000_000,
        "2026-04-10",
        "Project payment",
        Some(project_id),
    )
    .await;
    insert_cash_flow_entry(
        &pool,
        finance_id,
        "cash_in",
        "other_income",
        3_000_000,
        "2026-04-15",
        "Unlinked income",
        None,
    )
    .await;

    let (status, body) = get_dashboard(
        &app,
        &token,
        &format!(
            "start_date=2026-04-01&end_date=2026-04-30&project_id={}",
            project_id
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_cash_in_idr"].as_i64().unwrap(), 5_000_000);
    assert_eq!(body["project_id"].as_str().unwrap(), project_id.to_string());

    let months = body["months"].as_array().unwrap();
    assert_eq!(months.len(), 1);
    let april = &months[0];
    assert_eq!(april["cash_in_idr"].as_i64().unwrap(), 5_000_000);

    let entries = april["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "Only the project-linked entry");
    assert_eq!(
        entries[0]["description"].as_str().unwrap(),
        "Project payment"
    );
}

// ── Default Date Range Tests ───────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn default_date_range_is_current_year(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let _user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    let (status, body) = get_dashboard(&app, &token, "").await;

    assert_eq!(status, StatusCode::OK);
    let months = body["months"].as_array().unwrap();
    assert_eq!(months.len(), 12, "Full year = 12 months by default");
    assert_eq!(months[0]["month_label"].as_str().unwrap(), "Jan");
    assert_eq!(months[11]["month_label"].as_str().unwrap(), "Dec");
}

// ── Drill-Down Entry Tests ─────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn drilldown_entries_included_per_month(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email();
    let user_id = create_test_user_with_role(&pool, &email, "finance").await;
    let token = get_auth_token(&app, &email).await;

    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_in",
        "client_payment",
        7_000_000,
        "2026-05-05",
        "May payment A",
        None,
    )
    .await;
    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_out",
        "tax",
        1_500_000,
        "2026-05-20",
        "May tax",
        None,
    )
    .await;
    insert_cash_flow_entry(
        &pool,
        user_id,
        "cash_in",
        "interest",
        200_000,
        "2026-06-01",
        "Jun interest",
        None,
    )
    .await;

    let (status, body) =
        get_dashboard(&app, &token, "start_date=2026-05-01&end_date=2026-06-30").await;

    assert_eq!(status, StatusCode::OK);
    let months = body["months"].as_array().unwrap();

    let may_entries = months[0]["entries"].as_array().unwrap();
    assert_eq!(may_entries.len(), 2);

    let jun_entries = months[1]["entries"].as_array().unwrap();
    assert_eq!(jun_entries.len(), 1);
    assert_eq!(
        jun_entries[0]["description"].as_str().unwrap(),
        "Jun interest"
    );
}
