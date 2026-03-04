use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::{Datelike, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("projrevenue-{}@example.com", Uuid::new_v4())
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

async fn upsert_revenue_via_api(
    app: &axum::Router,
    token: &str,
    project_id: Uuid,
    revenue_month: &str,
    amount_idr: i64,
    override_erp: bool,
    source_reference: Option<&str>,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/revenue", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "revenue_month": revenue_month,
                "amount_idr": amount_idr,
                "override_erp": override_erp,
                "source_reference": source_reference,
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

async fn ingest_erp_revenue_via_api(
    app: &axum::Router,
    token: &str,
    project_id: Uuid,
    revenue_month: &str,
    amount_idr: i64,
    source_reference: &str,
) -> (StatusCode, Value) {
    ingest_erp_revenue_via_api_with_idempotency(
        app,
        token,
        project_id,
        revenue_month,
        amount_idr,
        source_reference,
        None,
    )
    .await
}

async fn ingest_erp_revenue_via_api_with_idempotency(
    app: &axum::Router,
    token: &str,
    project_id: Uuid,
    revenue_month: &str,
    amount_idr: i64,
    source_reference: &str,
    idempotency_key: Option<&str>,
) -> (StatusCode, Value) {
    let mut request_builder = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/projects/{}/revenue/erp-sync", project_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json");

    if let Some(key) = idempotency_key {
        request_builder = request_builder.header("Idempotency-Key", key);
    }

    let req = request_builder
        .body(Body::from(
            json!({
                "revenue_month": revenue_month,
                "amount_idr": amount_idr,
                "source_reference": source_reference,
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

async fn get_revenue_grid_via_api(
    app: &axum::Router,
    token: &str,
    project_id: Uuid,
    year: Option<i32>,
) -> (StatusCode, Value) {
    let uri = if let Some(year) = year {
        format!("/api/v1/projects/{}/revenue?year={}", project_id, year)
    } else {
        format!("/api/v1/projects/{}/revenue", project_id)
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

#[sqlx::test(migrations = "../../migrations")]
async fn pm_can_upsert_revenue_on_own_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue PM Upsert", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let (status, body) =
        upsert_revenue_via_api(&app, &token, project_id, "2026-03", 1_500_000, false, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["project_id"].as_str().unwrap(), project_id.to_string());
    assert_eq!(body["revenue_month"].as_str().unwrap(), "2026-03-01");
    assert_eq!(body["amount_idr"].as_i64().unwrap(), 1_500_000);
    assert_eq!(body["source_type"].as_str().unwrap(), "manual");
    assert!(body["id"].as_str().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_can_read_revenue_on_own_project_with_default_year_and_dense_grid(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let current_year = Utc::now().date_naive().year();
    let march = format!("{}-03", current_year);

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue PM Read", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let (upsert_status, _) =
        upsert_revenue_via_api(&app, &token, project_id, &march, 2_100_000, false, None).await;
    assert_eq!(upsert_status, StatusCode::OK);

    let (status, body) = get_revenue_grid_via_api(&app, &token, project_id, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["project_id"].as_str().unwrap(), project_id.to_string());
    assert_eq!(body["year"].as_i64().unwrap(), current_year as i64);

    let months = body["months"].as_array().unwrap();
    assert_eq!(months.len(), 12, "grid should always return 12 months");

    let mar = &months[2];
    assert_eq!(mar["month"].as_u64().unwrap(), 3);
    assert_eq!(mar["amount_idr"].as_i64().unwrap(), 2_100_000);
    assert_eq!(mar["source_type"].as_str().unwrap(), "manual");
    assert!(mar["revenue_id"].as_str().is_some());

    let apr = &months[3];
    assert_eq!(apr["month"].as_u64().unwrap(), 4);
    assert_eq!(apr["amount_idr"].as_i64().unwrap(), 0);
    assert!(apr["revenue_id"].is_null());
    assert!(apr["source_type"].is_null());
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_denied_upsert_revenue_on_non_owned_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm1_email = test_email();
    let pm2_email = test_email();
    let pm1_id = create_test_user_with_role(&pool, &pm1_email, "project_manager").await;
    let _pm2_id = create_test_user_with_role(&pool, &pm2_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue PM1 Project", pm1_id).await;
    let pm2_token = get_auth_token(&app, &pm2_email).await;

    let (status, body) = upsert_revenue_via_api(
        &app, &pm2_token, project_id, "2026-04", 500_000, false, None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "FORBIDDEN_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn pm_denied_read_revenue_on_non_owned_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm1_email = test_email();
    let pm2_email = test_email();
    let pm1_id = create_test_user_with_role(&pool, &pm1_email, "project_manager").await;
    let _pm2_id = create_test_user_with_role(&pool, &pm2_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue PM1 Read Project", pm1_id).await;
    let pm2_token = get_auth_token(&app, &pm2_email).await;

    let (status, body) = get_revenue_grid_via_api(&app, &pm2_token, project_id, Some(2026)).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "FORBIDDEN_ERROR");
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_upsert_and_read_revenue_on_any_project(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let admin_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue Admin Project", pm_id).await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    let (upsert_status, upsert_body) = upsert_revenue_via_api(
        &app,
        &admin_token,
        project_id,
        "2026-05",
        3_250_000,
        false,
        None,
    )
    .await;
    assert_eq!(upsert_status, StatusCode::OK);
    assert_eq!(upsert_body["source_type"].as_str().unwrap(), "manual");

    let (get_status, get_body) =
        get_revenue_grid_via_api(&app, &admin_token, project_id, Some(2026)).await;
    assert_eq!(get_status, StatusCode::OK);

    let may = &get_body["months"].as_array().unwrap()[4];
    assert_eq!(may["amount_idr"].as_i64().unwrap(), 3_250_000);
}

#[sqlx::test(migrations = "../../migrations")]
async fn finance_can_call_erp_ingest_endpoint(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let finance_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue ERP Project", pm_id).await;
    let finance_token = get_auth_token(&app, &finance_email).await;

    let (status, body) = ingest_erp_revenue_via_api(
        &app,
        &finance_token,
        project_id,
        "2026-06",
        4_000_000,
        "ERP-INV-1",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["source_type"].as_str().unwrap(), "erp_synced");
    assert_eq!(body["source_reference"].as_str().unwrap(), "ERP-INV-1");
}

#[sqlx::test(migrations = "../../migrations")]
async fn finance_cannot_call_manual_revenue_endpoints(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let finance_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue Finance Denied", pm_id).await;
    let finance_token = get_auth_token(&app, &finance_email).await;

    let (upsert_status, upsert_body) = upsert_revenue_via_api(
        &app,
        &finance_token,
        project_id,
        "2026-07",
        1_000_000,
        false,
        None,
    )
    .await;
    assert_eq!(upsert_status, StatusCode::FORBIDDEN);
    assert_eq!(
        upsert_body["error"]["code"].as_str().unwrap(),
        "FORBIDDEN_ERROR"
    );

    let (get_status, get_body) =
        get_revenue_grid_via_api(&app, &finance_token, project_id, Some(2026)).await;
    assert_eq!(get_status, StatusCode::FORBIDDEN);
    assert_eq!(
        get_body["error"]["code"].as_str().unwrap(),
        "FORBIDDEN_ERROR"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_revenue_month_format_rejected(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue Month Validation", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    for invalid_month in ["2026-13", "abcd", "2026-1"] {
        let (status, body) = upsert_revenue_via_api(
            &app,
            &token,
            project_id,
            invalid_month,
            100_000,
            false,
            None,
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("YYYY-MM"));
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn negative_amount_rejected_and_zero_amount_accepted(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue Amount Validation", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let (neg_status, neg_body) =
        upsert_revenue_via_api(&app, &token, project_id, "2026-08", -1, false, None).await;
    assert_eq!(neg_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        neg_body["error"]["code"].as_str().unwrap(),
        "VALIDATION_ERROR"
    );

    let (zero_status, zero_body) =
        upsert_revenue_via_api(&app, &token, project_id, "2026-08", 0, false, None).await;
    assert_eq!(zero_status, StatusCode::OK);
    assert_eq!(zero_body["amount_idr"].as_i64().unwrap(), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn erp_ingest_requires_source_reference_and_creates_erp_synced_row(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let finance_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue ERP Source Ref", pm_id).await;
    let finance_token = get_auth_token(&app, &finance_email).await;

    let (missing_status, missing_body) =
        ingest_erp_revenue_via_api(&app, &finance_token, project_id, "2026-09", 5_000_000, "")
            .await;
    assert_eq!(missing_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        missing_body["error"]["code"].as_str().unwrap(),
        "VALIDATION_ERROR"
    );

    let (status, body) = ingest_erp_revenue_via_api(
        &app,
        &finance_token,
        project_id,
        "2026-09",
        5_000_000,
        "ERP-INV-9",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["source_type"].as_str().unwrap(), "erp_synced");
    assert_eq!(body["source_reference"].as_str().unwrap(), "ERP-INV-9");
}

#[sqlx::test(migrations = "../../migrations")]
async fn manual_override_of_erp_row_requires_flag_and_sets_manual_override(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let finance_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue Manual Override", pm_id).await;
    let pm_token = get_auth_token(&app, &pm_email).await;
    let finance_token = get_auth_token(&app, &finance_email).await;

    let (erp_status, _) = ingest_erp_revenue_via_api(
        &app,
        &finance_token,
        project_id,
        "2026-10",
        6_000_000,
        "ERP-INV-10",
    )
    .await;
    assert_eq!(erp_status, StatusCode::OK);

    let (override_status, override_body) = upsert_revenue_via_api(
        &app,
        &pm_token,
        project_id,
        "2026-10",
        6_250_000,
        true,
        Some("manual adjustment"),
    )
    .await;

    assert_eq!(override_status, StatusCode::OK);
    assert_eq!(override_body["amount_idr"].as_i64().unwrap(), 6_250_000);
    assert_eq!(
        override_body["source_type"].as_str().unwrap(),
        "manual_override"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn manual_override_of_erp_row_without_flag_returns_validation_error(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let finance_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue Override Validation", pm_id).await;
    let pm_token = get_auth_token(&app, &pm_email).await;
    let finance_token = get_auth_token(&app, &finance_email).await;

    let (erp_status, _) = ingest_erp_revenue_via_api(
        &app,
        &finance_token,
        project_id,
        "2026-11",
        7_000_000,
        "ERP-INV-11",
    )
    .await;
    assert_eq!(erp_status, StatusCode::OK);

    let (status, body) = upsert_revenue_via_api(
        &app,
        &pm_token,
        project_id,
        "2026-11",
        7_250_000,
        false,
        Some("manual adjustment"),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"].as_str().unwrap(), "VALIDATION_ERROR");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("override_erp=true"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn ytd_total_equals_sum_of_monthly_values(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue YTD Project", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let (s1, _) =
        upsert_revenue_via_api(&app, &token, project_id, "2026-01", 100_000, false, None).await;
    let (s2, _) =
        upsert_revenue_via_api(&app, &token, project_id, "2026-02", 250_000, false, None).await;
    let (s3, _) =
        upsert_revenue_via_api(&app, &token, project_id, "2026-03", 50_000, false, None).await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(s3, StatusCode::OK);

    let (status, body) = get_revenue_grid_via_api(&app, &token, project_id, Some(2026)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ytd_total_idr"].as_i64().unwrap(), 400_000);

    let months = body["months"].as_array().unwrap();
    assert_eq!(months[0]["amount_idr"].as_i64().unwrap(), 100_000);
    assert_eq!(months[1]["amount_idr"].as_i64().unwrap(), 250_000);
    assert_eq!(months[2]["amount_idr"].as_i64().unwrap(), 50_000);
}

#[sqlx::test(migrations = "../../migrations")]
async fn idempotent_erp_ingest_does_not_duplicate_or_inflate_totals(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let finance_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue ERP Idempotent", pm_id).await;
    let finance_token = get_auth_token(&app, &finance_email).await;
    let pm_token = get_auth_token(&app, &pm_email).await;

    let (first_status, _) = ingest_erp_revenue_via_api(
        &app,
        &finance_token,
        project_id,
        "2026-12",
        1_100_000,
        "ERP-INV-12",
    )
    .await;
    let (second_status, _) = ingest_erp_revenue_via_api(
        &app,
        &finance_token,
        project_id,
        "2026-12",
        1_100_000,
        "ERP-INV-12",
    )
    .await;

    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::OK);

    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM project_revenues WHERE project_id = $1 AND revenue_month = '2026-12-01'::date",
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await
    .expect("count query should succeed");
    assert_eq!(count, 1, "ERP ingest should keep one row per month");

    let (grid_status, grid_body) =
        get_revenue_grid_via_api(&app, &pm_token, project_id, Some(2026)).await;
    assert_eq!(grid_status, StatusCode::OK);
    assert_eq!(grid_body["ytd_total_idr"].as_i64().unwrap(), 1_100_000);
}

#[sqlx::test(migrations = "../../migrations")]
async fn idempotency_key_short_circuits_repeated_erp_ingest(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let finance_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue ERP Idempotency Key", pm_id).await;
    let finance_token = get_auth_token(&app, &finance_email).await;

    let (first_status, first_body) = ingest_erp_revenue_via_api_with_idempotency(
        &app,
        &finance_token,
        project_id,
        "2026-08",
        1_500_000,
        "ERP-INV-K1",
        Some("erp-key-001"),
    )
    .await;

    let (second_status, second_body) = ingest_erp_revenue_via_api_with_idempotency(
        &app,
        &finance_token,
        project_id,
        "2026-08",
        9_999_999,
        "ERP-INV-K1-RETRY",
        Some("erp-key-001"),
    )
    .await;

    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(
        first_body["id"].as_str().unwrap(),
        second_body["id"].as_str().unwrap()
    );
    assert_eq!(second_body["amount_idr"].as_i64().unwrap(), 1_500_000);

    let stored_source_reference = second_body["source_reference"].as_str().unwrap();
    assert!(stored_source_reference.starts_with("erp:erp-key-001:"));
    assert_eq!(
        stored_source_reference, "erp:erp-key-001:ERP-INV-K1",
        "replayed request with same idempotency key should return original row",
    );

    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM project_revenues WHERE project_id = $1 AND revenue_month = '2026-08-01'::date",
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await
    .expect("count query should succeed");
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn upsert_same_month_twice_updates_existing_row(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue Upsert Same Month", pm_id).await;
    let token = get_auth_token(&app, &pm_email).await;

    let (first_status, first_body) =
        upsert_revenue_via_api(&app, &token, project_id, "2026-04", 700_000, false, None).await;
    let (second_status, second_body) =
        upsert_revenue_via_api(&app, &token, project_id, "2026-04", 850_000, false, None).await;

    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(
        first_body["id"].as_str().unwrap(),
        second_body["id"].as_str().unwrap(),
        "same month should update existing row"
    );
    assert_eq!(second_body["amount_idr"].as_i64().unwrap(), 850_000);

    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM project_revenues WHERE project_id = $1 AND revenue_month = '2026-04-01'::date",
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await
    .expect("count query should succeed");
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn erp_ingest_does_not_overwrite_manual_or_manual_override_entries(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email();
    let finance_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let _finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let project_id = create_test_project_with_pm(&pool, "Revenue Preserve Manual", pm_id).await;
    let pm_token = get_auth_token(&app, &pm_email).await;
    let finance_token = get_auth_token(&app, &finance_email).await;

    let (manual_status, _) =
        upsert_revenue_via_api(&app, &pm_token, project_id, "2026-09", 900_000, false, None).await;
    assert_eq!(manual_status, StatusCode::OK);

    let (erp_on_manual_status, erp_on_manual_body) = ingest_erp_revenue_via_api(
        &app,
        &finance_token,
        project_id,
        "2026-09",
        1_200_000,
        "ERP-INV-9B",
    )
    .await;
    assert_eq!(erp_on_manual_status, StatusCode::OK);
    assert_eq!(
        erp_on_manual_body["source_type"].as_str().unwrap(),
        "manual"
    );
    assert_eq!(erp_on_manual_body["amount_idr"].as_i64().unwrap(), 900_000);

    let (erp_seed_status, _) = ingest_erp_revenue_via_api(
        &app,
        &finance_token,
        project_id,
        "2026-10",
        1_000_000,
        "ERP-INV-10A",
    )
    .await;
    assert_eq!(erp_seed_status, StatusCode::OK);

    let (override_status, _) = upsert_revenue_via_api(
        &app,
        &pm_token,
        project_id,
        "2026-10",
        1_350_000,
        true,
        Some("override from PM"),
    )
    .await;
    assert_eq!(override_status, StatusCode::OK);

    let (erp_on_override_status, erp_on_override_body) = ingest_erp_revenue_via_api(
        &app,
        &finance_token,
        project_id,
        "2026-10",
        2_000_000,
        "ERP-INV-10B",
    )
    .await;
    assert_eq!(erp_on_override_status, StatusCode::OK);
    assert_eq!(
        erp_on_override_body["source_type"].as_str().unwrap(),
        "manual_override"
    );
    assert_eq!(
        erp_on_override_body["amount_idr"].as_i64().unwrap(),
        1_350_000
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn revenue_for_non_existent_project_returns_404(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email();
    let finance_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let _finance_id = create_test_user_with_role(&pool, &finance_email, "finance").await;
    let admin_token = get_auth_token(&app, &admin_email).await;
    let finance_token = get_auth_token(&app, &finance_email).await;

    let missing_project_id = Uuid::new_v4();

    let (get_status, get_body) =
        get_revenue_grid_via_api(&app, &admin_token, missing_project_id, Some(2026)).await;
    assert_eq!(get_status, StatusCode::NOT_FOUND);
    assert_eq!(get_body["error"]["code"].as_str().unwrap(), "NOT_FOUND");

    let (manual_status, manual_body) = upsert_revenue_via_api(
        &app,
        &admin_token,
        missing_project_id,
        "2026-01",
        100_000,
        false,
        None,
    )
    .await;
    assert_eq!(manual_status, StatusCode::NOT_FOUND);
    assert_eq!(manual_body["error"]["code"].as_str().unwrap(), "NOT_FOUND");

    let (erp_status, erp_body) = ingest_erp_revenue_via_api(
        &app,
        &finance_token,
        missing_project_id,
        "2026-01",
        200_000,
        "ERP-MISSING-1",
    )
    .await;
    assert_eq!(erp_status, StatusCode::NOT_FOUND);
    assert_eq!(erp_body["error"]["code"].as_str().unwrap(), "NOT_FOUND");
}
