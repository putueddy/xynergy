use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn set_test_env() {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );
}

fn test_email(prefix: &str) -> String {
    format!("{}-{}@example.com", prefix, Uuid::new_v4())
}

async fn create_user_with_role(pool: &PgPool, email: &str, role: &str) -> Uuid {
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

async fn assign_user_to_department(pool: &PgPool, user_id: Uuid, dept_id: Uuid) {
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(dept_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("user dept assigned");
}

async fn create_department(pool: &PgPool, name: &str, head_id: Option<Uuid>) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO departments (name, head_id) VALUES ($1, $2) RETURNING id",
    )
    .bind(name)
    .bind(head_id)
    .fetch_one(pool)
    .await
    .expect("department created")
}

async fn set_department_head(pool: &PgPool, dept_id: Uuid, user_id: Uuid) {
    sqlx::query("UPDATE departments SET head_id = $1 WHERE id = $2")
        .bind(user_id)
        .bind(dept_id)
        .execute(pool)
        .await
        .expect("department head set");
}

async fn create_resource_in_dept(pool: &PgPool, name: &str, dept_id: Uuid) -> Uuid {
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

async fn add_ctc_revision(pool: &PgPool, resource_id: Uuid, user_id: Uuid) {
    sqlx::query(
        "INSERT INTO ctc_revisions (resource_id, revision_number, key_version, encryption_version, encryption_algorithm, encrypted_at, encrypted_components, encrypted_daily_rate, effective_date_policy, effective_date, working_days_per_month, status, changed_by, reason)
         VALUES ($1, 1, 'v1', 'v1', 'aes-256-gcm', CURRENT_TIMESTAMP, 'ciphertext-redacted', 'ciphertext-redacted', 'pro_rata', CURRENT_DATE, 22, 'Active', $2, 'Quarterly raise')",
    )
    .bind(resource_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("ctc revision inserted");
}

async fn create_project_with_pm(pool: &PgPool, name: &str, pm_id: Uuid) -> Uuid {
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

async fn create_allocation(
    pool: &PgPool,
    resource_id: Uuid,
    project_id: Uuid,
    pct: f64,
    days_offset_start: i64,
    days_offset_end: i64,
) {
    sqlx::query(
        "INSERT INTO allocations (resource_id, project_id, allocation_percentage, start_date, end_date, include_weekend)
         VALUES ($1, $2, $3,
                 CURRENT_DATE + ($4 || ' days')::INTERVAL,
                 CURRENT_DATE + ($5 || ' days')::INTERVAL,
                 false)",
    )
    .bind(resource_id)
    .bind(project_id)
    .bind(sqlx::types::BigDecimal::try_from(pct).expect("pct"))
    .bind(days_offset_start.to_string())
    .bind(days_offset_end.to_string())
    .execute(pool)
    .await
    .expect("allocation created");
}

async fn insert_cash_flow_entry(
    pool: &PgPool,
    user_id: Uuid,
    entry_type: &str,
    category: &str,
    amount_idr: i64,
    description: &str,
) {
    sqlx::query(
        "INSERT INTO cash_flow_entries (entry_type, category, amount_idr, entry_date, description, created_by)
         VALUES ($1, $2, $3, CURRENT_DATE, $4, $5)",
    )
    .bind(entry_type)
    .bind(category)
    .bind(amount_idr)
    .bind(description)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("cash flow entry");
}

async fn insert_cash_flow_entry_with_day_offset(
    pool: &PgPool,
    user_id: Uuid,
    entry_type: &str,
    category: &str,
    amount_idr: i64,
    description: &str,
    day_offset: i64,
) {
    sqlx::query(
        "INSERT INTO cash_flow_entries (entry_type, category, amount_idr, entry_date, description, created_by)
         VALUES ($1, $2, $3, CURRENT_DATE + ($4 || ' days')::INTERVAL, $5, $6)",
    )
    .bind(entry_type)
    .bind(category)
    .bind(amount_idr)
    .bind(day_offset.to_string())
    .bind(description)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("cash flow entry with date offset");
}

async fn insert_export_request(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        "INSERT INTO audit_export_requests (id, requested_by, status, note, report_type)
         VALUES ($1, $2, 'pending_approval', 'Needs review', 'compliance_audit')",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .execute(pool)
    .await
    .expect("export request");
}

async fn login_token(app: &axum::Router, email: &str) -> String {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"email": email, "password": "Password123!"}).to_string(),
        ))
        .expect("login request");
    let resp = app.clone().oneshot(req).await.expect("login response");
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("body");
    let body: Value = serde_json::from_slice(&bytes).expect("login payload");
    body["token"].as_str().expect("token").to_string()
}

async fn get_dashboard(app: &axum::Router, token: Option<&str>) -> (StatusCode, Value) {
    let mut req_builder = Request::builder().method("GET").uri("/api/v1/dashboard");
    if let Some(t) = token {
        req_builder = req_builder.header("Authorization", format!("Bearer {}", t));
    }
    let req = req_builder.body(Body::empty()).expect("dashboard request");
    let resp = app.clone().oneshot(req).await.expect("dashboard response");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("body");
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, body)
}

// ── Authentication ───────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn unauthenticated_dashboard_returns_401(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let (status, _body) = get_dashboard(&app, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ── HR ──────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn hr_dashboard_returns_hr_section_only(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_department(&pool, "Engineering", None).await;
    let hr_email = test_email("hr");
    let hr_id = create_user_with_role(&pool, &hr_email, "hr").await;
    assign_user_to_department(&pool, hr_id, dept_id).await;

    let with_ctc = create_resource_in_dept(&pool, "Has CTC", dept_id).await;
    let _without_ctc = create_resource_in_dept(&pool, "Missing CTC", dept_id).await;
    create_ctc_for_resource(&pool, with_ctc, hr_id).await;
    add_ctc_revision(&pool, with_ctc, hr_id).await;

    let token = login_token(&app, &hr_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["role"], "hr");
    assert!(body["hr"].is_object(), "expected hr section");
    assert!(body["department_head"].is_null());
    assert!(body["project_manager"].is_null());
    assert!(body["finance"].is_null());
    assert!(body["admin"].is_null());

    let hr = &body["hr"];
    assert!(hr["completeness"].is_object(), "completeness present");
    assert!(hr["pending_updates"]["missing_count"].as_i64().unwrap() >= 1);
    assert!(hr["recent_changes"].is_array());
    assert!(hr["compliance_alerts"].is_object());
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_recent_changes_do_not_expose_encrypted_fields(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_department(&pool, "Engineering", None).await;
    let hr_email = test_email("hr-safe");
    let hr_id = create_user_with_role(&pool, &hr_email, "hr").await;
    assign_user_to_department(&pool, hr_id, dept_id).await;

    let resource_id = create_resource_in_dept(&pool, "Person", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, hr_id).await;
    add_ctc_revision(&pool, resource_id, hr_id).await;

    let token = login_token(&app, &hr_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let serialized = serde_json::to_string(&body["hr"]["recent_changes"]).unwrap();
    for forbidden in [
        "encrypted_components",
        "encrypted_daily_rate",
        "ciphertext",
        "key_version",
        "encryption_algorithm",
        "encryption_version",
        "base_salary",
        "daily_rate",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "recent_changes leaked sensitive field `{}`",
            forbidden
        );
    }
}

// ── Department Head ─────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn dept_head_only_sees_own_department(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let eng_id = create_department(&pool, "Engineering", None).await;
    let mkt_id = create_department(&pool, "Marketing", None).await;

    let dh_email = test_email("dh");
    let dh_id = create_user_with_role(&pool, &dh_email, "department_head").await;
    assign_user_to_department(&pool, dh_id, eng_id).await;
    set_department_head(&pool, eng_id, dh_id).await;

    let eng_resource = create_resource_in_dept(&pool, "Eng Person", eng_id).await;
    let mkt_resource = create_resource_in_dept(&pool, "Mkt Person", mkt_id).await;
    create_ctc_for_resource(&pool, eng_resource, dh_id).await;
    create_ctc_for_resource(&pool, mkt_resource, dh_id).await;

    let pm_email = test_email("pm-dh");
    let pm_id = create_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = create_project_with_pm(&pool, "Atlas", pm_id).await;

    // Allocations: one in own dept (future), one cross-dept (should be excluded)
    create_allocation(&pool, eng_resource, project_id, 50.0, 1, 30).await;
    create_allocation(&pool, mkt_resource, project_id, 80.0, 1, 30).await;

    let token = login_token(&app, &dh_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["role"], "department_head");
    let dh = &body["department_head"];
    assert!(dh.is_object(), "department_head section present");

    let upcoming = dh["upcoming_assignments"]
        .as_array()
        .expect("upcoming_assignments array");
    let names: Vec<&str> = upcoming
        .iter()
        .map(|item| item["resource_name"].as_str().unwrap_or(""))
        .collect();
    assert!(names.contains(&"Eng Person"));
    assert!(
        !names.contains(&"Mkt Person"),
        "department head must not see other departments"
    );

    // Other role sections empty
    assert!(body["hr"].is_null());
    assert!(body["project_manager"].is_null());
    assert!(body["finance"].is_null());
    assert!(body["admin"].is_null());
}

#[sqlx::test(migrations = "../../migrations")]
async fn dept_head_response_has_utilization_and_overallocation(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_department(&pool, "Engineering", None).await;
    let dh_email = test_email("dh-util");
    let dh_id = create_user_with_role(&pool, &dh_email, "department_head").await;
    assign_user_to_department(&pool, dh_id, dept_id).await;
    set_department_head(&pool, dept_id, dh_id).await;

    let resource_id = create_resource_in_dept(&pool, "Loaded", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, dh_id).await;
    let pm_id = create_user_with_role(&pool, &test_email("pm-util"), "project_manager").await;
    let project_id = create_project_with_pm(&pool, "OverProject", pm_id).await;
    // Two 60% allocations -> overallocated
    create_allocation(&pool, resource_id, project_id, 60.0, -2, 30).await;
    create_allocation(&pool, resource_id, project_id, 60.0, -2, 30).await;

    let token = login_token(&app, &dh_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let dh = &body["department_head"];
    assert!(dh["utilization"].is_object());
    assert!(
        dh["overallocations"]["overallocated_count"]
            .as_i64()
            .unwrap()
            >= 1
    );
}

// ── Project Manager ─────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn project_manager_only_sees_owned_projects(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email("pm");
    let pm_id = create_user_with_role(&pool, &pm_email, "project_manager").await;
    let other_pm_id =
        create_user_with_role(&pool, &test_email("pm-other"), "project_manager").await;

    let owned = create_project_with_pm(&pool, "Owned", pm_id).await;
    sqlx::query(
        "UPDATE projects SET total_budget_idr = 10000000, budget_hr_idr = 10000000 WHERE id = $1",
    )
    .bind(owned)
    .execute(&pool)
    .await
    .expect("project budget updated");
    let _not_owned = create_project_with_pm(&pool, "Not Owned", other_pm_id).await;

    let token = login_token(&app, &pm_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["role"], "project_manager");
    let pm = &body["project_manager"];
    assert!(pm.is_object());

    let projects = pm["active_projects"].as_array().expect("active projects");
    let names: Vec<&str> = projects
        .iter()
        .map(|p| p["project_name"].as_str().unwrap_or(""))
        .collect();
    assert!(names.contains(&"Owned"));
    assert!(!names.contains(&"Not Owned"));

    // First card should at least include the owned project's id
    assert_eq!(
        projects[0]["project_id"].as_str().unwrap(),
        owned.to_string().as_str()
    );
    assert_eq!(
        projects[0]["total_budget_idr"].as_i64().unwrap(),
        10_000_000
    );
    assert_eq!(projects[0]["budget_spent_idr"].as_i64().unwrap(), 0);
    assert_eq!(
        projects[0]["budget_remaining_idr"].as_i64().unwrap(),
        10_000_000
    );
    assert_eq!(projects[0]["budget_status"], "healthy");

    // Margin alert array must exist
    assert!(pm["margin_alerts"].is_array());

    assert!(body["hr"].is_null());
    assert!(body["department_head"].is_null());
    assert!(body["finance"].is_null());
}

// ── Finance ─────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn finance_dashboard_has_cash_and_audit_state(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let fin_email = test_email("fin");
    let fin_id = create_user_with_role(&pool, &fin_email, "finance").await;

    insert_cash_flow_entry(
        &pool,
        fin_id,
        "cash_in",
        "client_payment",
        5_000_000,
        "Invoice",
    )
    .await;
    insert_cash_flow_entry(
        &pool,
        fin_id,
        "cash_out",
        "vendor_payment",
        2_000_000,
        "Vendor",
    )
    .await;
    insert_export_request(&pool, fin_id).await;

    let token = login_token(&app, &fin_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["role"], "finance");
    let finance = &body["finance"];
    assert!(finance.is_object());
    assert!(finance["cash_position"].is_object());
    assert_eq!(
        finance["cash_position"]["total_cash_in_idr"]
            .as_i64()
            .unwrap(),
        5_000_000
    );
    assert_eq!(
        finance["cash_position"]["total_cash_out_idr"]
            .as_i64()
            .unwrap(),
        2_000_000
    );

    let validation_status = finance["ctc_validation"]["status"].as_str().unwrap();
    assert!(
        matches!(validation_status, "no_data" | "ok" | "error"),
        "unexpected validation status: {}",
        validation_status
    );

    assert!(finance["audit_alerts"].is_object());
    assert!(
        finance["export_requests"]["pending_count"]
            .as_i64()
            .unwrap()
            >= 1
    );

    // Finance dashboard must not leak HR/DH/PM
    assert!(body["hr"].is_null());
    assert!(body["department_head"].is_null());
    assert!(body["project_manager"].is_null());
}

#[sqlx::test(migrations = "../../migrations")]
async fn finance_validation_returns_no_data_when_no_payroll(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let fin_email = test_email("fin-empty");
    let _ = create_user_with_role(&pool, &fin_email, "finance").await;

    let token = login_token(&app, &fin_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let validation = &body["finance"]["ctc_validation"];
    assert_eq!(validation["status"], "no_data");
    assert!(validation["message"].is_string());
    assert!(validation["total_compared"].is_null());
}

#[sqlx::test(migrations = "../../migrations")]
async fn finance_cash_position_excludes_future_dated_entries(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let fin_email = test_email("fin-future-cash");
    let fin_id = create_user_with_role(&pool, &fin_email, "finance").await;

    insert_cash_flow_entry(
        &pool,
        fin_id,
        "cash_in",
        "client_payment",
        3_000_000,
        "Today invoice",
    )
    .await;
    insert_cash_flow_entry_with_day_offset(
        &pool,
        fin_id,
        "cash_in",
        "client_payment",
        9_000_000,
        "Future invoice",
        30,
    )
    .await;
    insert_cash_flow_entry_with_day_offset(
        &pool,
        fin_id,
        "cash_out",
        "vendor_payment",
        4_000_000,
        "Future vendor",
        30,
    )
    .await;

    let token = login_token(&app, &fin_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let cash = &body["finance"]["cash_position"];
    assert_eq!(cash["total_cash_in_idr"].as_i64().unwrap(), 3_000_000);
    assert_eq!(cash["total_cash_out_idr"].as_i64().unwrap(), 0);
    assert_eq!(cash["net_cash_flow_idr"].as_i64().unwrap(), 3_000_000);
}

// ── Admin ───────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn admin_dashboard_returns_admin_section(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email("admin");
    let _ = create_user_with_role(&pool, &admin_email, "admin").await;

    let token = login_token(&app, &admin_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["role"], "admin");
    let admin = &body["admin"];
    assert!(admin.is_object());
    assert!(admin["total_users"].as_i64().unwrap() >= 1);
    assert!(admin["total_departments"].as_i64().unwrap() >= 0);

    assert!(body["hr"].is_null());
    assert!(body["department_head"].is_null());
    assert!(body["project_manager"].is_null());
    assert!(body["finance"].is_null());
}

// ── Unsupported role ────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn unsupported_role_returns_403(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let email = test_email("tm");
    let _ = create_user_with_role(&pool, &email, "team_member").await;

    let token = login_token(&app, &email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "FORBIDDEN_ERROR");
}

// ───────────────────────────────────────────────────────────────────────
// Story 6.1 expansion coverage (Test Architect: data isolation, bounds,
// empty-state, security boundary).
// ───────────────────────────────────────────────────────────────────────

async fn insert_audit_log(pool: &PgPool, user_id: Uuid, action: &str) {
    sqlx::query(
        "INSERT INTO audit_logs (user_id, action, entity_type, entity_id, changes, created_at)
         VALUES ($1, $2, 'users', $1, '{}'::jsonb, CURRENT_TIMESTAMP)",
    )
    .bind(user_id)
    .bind(action)
    .execute(pool)
    .await
    .expect("audit log inserted");
}

async fn create_project_with_status(pool: &PgPool, name: &str, pm_id: Uuid, status: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, status, start_date, end_date, project_manager_id)
         VALUES ($1, $2, CURRENT_DATE - INTERVAL '60 days', CURRENT_DATE + INTERVAL '60 days', $3)
         RETURNING id",
    )
    .bind(name)
    .bind(status)
    .bind(pm_id)
    .fetch_one(pool)
    .await
    .expect("project with status created")
}

async fn create_project_with_status_and_end_offset(
    pool: &PgPool,
    name: &str,
    pm_id: Uuid,
    status: &str,
    end_offset_days: i64,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, status, start_date, end_date, project_manager_id)
         VALUES ($1, $2, CURRENT_DATE - INTERVAL '60 days', CURRENT_DATE + ($3 || ' days')::INTERVAL, $4)
         RETURNING id",
    )
    .bind(name)
    .bind(status)
    .bind(end_offset_days.to_string())
    .bind(pm_id)
    .fetch_one(pool)
    .await
    .expect("project with status and end offset created")
}

// ── 6.1-INT-011 Security: DH without department_id → 403 ───────────────
#[sqlx::test(migrations = "../../migrations")]
async fn dept_head_without_department_returns_403(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    // Department head user but no department_id set on users row.
    let dh_email = test_email("dh-orphan");
    let _ = create_user_with_role(&pool, &dh_email, "department_head").await;

    let token = login_token(&app, &dh_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "department_head without department must be rejected, not silently scoped"
    );
    assert_eq!(body["error"]["code"], "FORBIDDEN_ERROR");
}

// ── 6.1-INT-011B Security: DH dashboard uses current DB department, not stale JWT ─
#[sqlx::test(migrations = "../../migrations")]
async fn dept_head_dashboard_uses_current_department_not_stale_token(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dh_email = test_email("dh-stale-token");
    let dh_id = create_user_with_role(&pool, &dh_email, "department_head").await;

    let old_dept_id = create_department(&pool, "OldDept", Some(dh_id)).await;
    let new_dept_id = create_department(&pool, "NewDept", Some(dh_id)).await;
    set_department_head(&pool, old_dept_id, dh_id).await;
    assign_user_to_department(&pool, dh_id, old_dept_id).await;

    let old_resource = create_resource_in_dept(&pool, "Old Dept Resource", old_dept_id).await;
    let new_resource = create_resource_in_dept(&pool, "New Dept Resource", new_dept_id).await;
    let pm_id = create_user_with_role(&pool, &test_email("dh-stale-pm"), "project_manager").await;
    let old_project = create_project_with_pm(&pool, "Old Dept Project", pm_id).await;
    let new_project = create_project_with_pm(&pool, "New Dept Project", pm_id).await;
    create_allocation(&pool, old_resource, old_project, 50.0, 1, 30).await;
    create_allocation(&pool, new_resource, new_project, 50.0, 1, 30).await;

    let token = login_token(&app, &dh_email).await;

    set_department_head(&pool, new_dept_id, dh_id).await;
    assign_user_to_department(&pool, dh_id, new_dept_id).await;

    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let upcoming = body["department_head"]["upcoming_assignments"]
        .as_array()
        .expect("upcoming_assignments array");
    let project_names: Vec<&str> = upcoming
        .iter()
        .map(|a| a["project_name"].as_str().unwrap_or(""))
        .collect();

    assert!(project_names.contains(&"New Dept Project"));
    assert!(
        !project_names.contains(&"Old Dept Project"),
        "dashboard must use the current users.department_id, not the stale department_id embedded in the token"
    );
}

// ── 6.1-INT-012 PM empty-state ─────────────────────────────────────────
#[sqlx::test(migrations = "../../migrations")]
async fn project_manager_with_no_projects_returns_empty_state(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email("pm-empty");
    let _ = create_user_with_role(&pool, &pm_email, "project_manager").await;

    let token = login_token(&app, &pm_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["role"], "project_manager");
    let pm = &body["project_manager"];
    assert!(pm.is_object(), "pm section present even when empty");
    assert_eq!(
        pm["active_projects"]
            .as_array()
            .expect("active_projects array")
            .len(),
        0
    );
    assert_eq!(
        pm["margin_alerts"]
            .as_array()
            .expect("margin_alerts array")
            .len(),
        0
    );
}

// ── 6.1-INT-013 PM excludes non-active projects ───────────────────────
#[sqlx::test(migrations = "../../migrations")]
async fn project_manager_excludes_non_active_projects(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email("pm-filter");
    let pm_id = create_user_with_role(&pool, &pm_email, "project_manager").await;

    let _active = create_project_with_status(&pool, "Active Project", pm_id, "Active").await;
    let _completed =
        create_project_with_status(&pool, "Completed Project", pm_id, "Completed").await;
    let _planning = create_project_with_status(&pool, "Planning Project", pm_id, "Planning").await;
    let _ended_active = create_project_with_status_and_end_offset(
        &pool,
        "Ended Active Project",
        pm_id,
        "Active",
        -1,
    )
    .await;
    let _closed = create_project_with_status(&pool, "Closed Project", pm_id, "Closed").await;
    let _cancelled =
        create_project_with_status(&pool, "Cancelled Project", pm_id, "Cancelled").await;

    let token = login_token(&app, &pm_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let projects = body["project_manager"]["active_projects"]
        .as_array()
        .expect("active_projects array");
    let names: Vec<&str> = projects
        .iter()
        .map(|p| p["project_name"].as_str().unwrap_or(""))
        .collect();

    assert!(names.contains(&"Active Project"));
    for forbidden in [
        "Completed Project",
        "Planning Project",
        "Ended Active Project",
        "Closed Project",
        "Cancelled Project",
    ] {
        assert!(
            !names.contains(&forbidden),
            "non-active project `{}` must not appear on PM dashboard",
            forbidden
        );
    }
}

// ── 6.1-INT-014 HR pending_updates sample bounded ─────────────────────
#[sqlx::test(migrations = "../../migrations")]
async fn hr_pending_updates_sample_bounded_to_limit(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_department(&pool, "Engineering", None).await;
    let hr_email = test_email("hr-bound");
    let hr_id = create_user_with_role(&pool, &hr_email, "hr").await;
    assign_user_to_department(&pool, hr_id, dept_id).await;

    // 8 resources without CTC → missing count = 8 but sample must be capped at 5.
    for idx in 0..8 {
        let _ = create_resource_in_dept(&pool, &format!("Missing CTC #{idx}"), dept_id).await;
    }

    let token = login_token(&app, &hr_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let pending = &body["hr"]["pending_updates"];
    assert_eq!(
        pending["missing_count"].as_i64().unwrap(),
        8,
        "missing_count must reflect full population, not truncated sample"
    );
    let sample = pending["sample"].as_array().expect("sample array");
    assert_eq!(
        sample.len(),
        5,
        "sample list must be bounded to MISSING_CTC_SAMPLE_LIMIT=5"
    );
}

// ── 6.1-INT-015 DH upcoming excludes past allocations ─────────────────
#[sqlx::test(migrations = "../../migrations")]
async fn dept_head_upcoming_excludes_past_allocations(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dept_id = create_department(&pool, "Engineering", None).await;
    let dh_email = test_email("dh-past");
    let dh_id = create_user_with_role(&pool, &dh_email, "department_head").await;
    assign_user_to_department(&pool, dh_id, dept_id).await;
    set_department_head(&pool, dept_id, dh_id).await;

    let resource_id = create_resource_in_dept(&pool, "Person", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, dh_id).await;

    let pm_id = create_user_with_role(&pool, &test_email("pm-past"), "project_manager").await;
    let past_project = create_project_with_pm(&pool, "PastProject", pm_id).await;
    let future_project = create_project_with_pm(&pool, "FutureProject", pm_id).await;

    // Past allocation ended yesterday — must be excluded from "upcoming".
    create_allocation(&pool, resource_id, past_project, 50.0, -30, -1).await;
    // Future allocation starting tomorrow — must appear.
    create_allocation(&pool, resource_id, future_project, 50.0, 1, 30).await;

    let token = login_token(&app, &dh_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let upcoming = body["department_head"]["upcoming_assignments"]
        .as_array()
        .expect("upcoming_assignments array");
    let project_names: Vec<&str> = upcoming
        .iter()
        .map(|item| item["project_name"].as_str().unwrap_or(""))
        .collect();

    assert!(
        project_names.contains(&"FutureProject"),
        "future allocation must appear: {:?}",
        project_names
    );
    assert!(
        !project_names.contains(&"PastProject"),
        "past allocation must be excluded from upcoming: {:?}",
        project_names
    );
}

// ── 6.1-INT-016 Contract: generated_at present and recent ─────────────
#[sqlx::test(migrations = "../../migrations")]
async fn dashboard_response_includes_recent_generated_at(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email("admin-ts");
    let _ = create_user_with_role(&pool, &admin_email, "admin").await;

    let before = chrono::Utc::now();
    let token = login_token(&app, &admin_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    let after = chrono::Utc::now();
    assert_eq!(status, StatusCode::OK);

    let generated_at = body["generated_at"]
        .as_str()
        .expect("generated_at must be present on every successful dashboard response");

    let parsed: chrono::DateTime<chrono::Utc> = generated_at
        .parse()
        .expect("generated_at must parse as RFC3339 UTC timestamp");

    // generated_at should fall in the request window (allow 5s slack on each side
    // for clock drift between the test harness and the server task).
    let slack = chrono::Duration::seconds(5);
    assert!(
        parsed >= before - slack && parsed <= after + slack,
        "generated_at {} must be within request window [{}, {}]",
        parsed,
        before,
        after
    );
}

// ── 6.1-INT-017 Finance audit alerts surfacing real audit signal ──────
#[sqlx::test(migrations = "../../migrations")]
async fn finance_audit_alerts_count_recent_security_events(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let fin_email = test_email("fin-alerts");
    let fin_id = create_user_with_role(&pool, &fin_email, "finance").await;

    // Seed three security events the dashboard summary watches for.
    insert_audit_log(&pool, fin_id, "ACCESS_DENIED").await;
    insert_audit_log(&pool, fin_id, "ACCESS_DENIED").await;
    insert_audit_log(&pool, fin_id, "LOGIN_FAILED").await;
    insert_audit_log(&pool, fin_id, "LOGIN_BLOCKED").await;

    let token = login_token(&app, &fin_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let alerts = &body["finance"]["audit_alerts"];
    assert_eq!(
        alerts["access_denied_count"].as_i64().unwrap(),
        2,
        "access_denied_count must reflect seeded ACCESS_DENIED rows"
    );
    assert!(
        alerts["login_failed_count"].as_i64().unwrap() >= 1,
        "login_failed_count must include seeded LOGIN_FAILED row"
    );
    assert!(
        alerts["login_blocked_count"].as_i64().unwrap() >= 1,
        "login_blocked_count must include seeded LOGIN_BLOCKED row"
    );

    let recent = alerts["recent"].as_array().expect("recent array");
    assert!(
        recent.len() >= 3,
        "recent list must include the seeded security events"
    );
    for entry in recent {
        let action = entry["action"].as_str().unwrap_or("");
        assert!(
            matches!(
                action,
                "ACCESS_DENIED"
                    | "LOGIN_FAILED"
                    | "LOGIN_BLOCKED"
                    | "CHAIN_VERIFICATION_FAILED"
                    | "VERIFY_CHAIN_FAILED"
            ),
            "recent audit alert action `{}` is outside the documented allow-list",
            action
        );
    }
}

// ── 6.1-INT-018 Security: malformed bearer token rejected ─────────────
#[sqlx::test(migrations = "../../migrations")]
async fn invalid_bearer_token_returns_401(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let (status, _body) = get_dashboard(&app, Some("not-a-real-jwt")).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "malformed bearer token must be rejected with 401, not silently treated as anonymous or accepted"
    );
}

// ───────────────────────────────────────────────────────────────────────
// Story 6.1 expansion coverage v2 (Test Architect): list bounds, ordering,
// status filters, time-window correctness, chain-failure signal.
// ───────────────────────────────────────────────────────────────────────

async fn add_ctc_revision_at(
    pool: &PgPool,
    resource_id: Uuid,
    user_id: Uuid,
    revision_number: i32,
    seconds_ago: i64,
) {
    sqlx::query(
        "INSERT INTO ctc_revisions (resource_id, revision_number, key_version, encryption_version, encryption_algorithm, encrypted_at, encrypted_components, encrypted_daily_rate, effective_date_policy, effective_date, working_days_per_month, status, changed_by, reason, created_at)
         VALUES ($1, $2, 'v1', 'v1', 'aes-256-gcm', CURRENT_TIMESTAMP, 'ciphertext-redacted', 'ciphertext-redacted', 'pro_rata', CURRENT_DATE, 22, 'Active', $3, 'Bulk revision', CURRENT_TIMESTAMP - ($4 || ' seconds')::INTERVAL)",
    )
    .bind(resource_id)
    .bind(revision_number)
    .bind(user_id)
    .bind(seconds_ago.to_string())
    .execute(pool)
    .await
    .expect("ctc revision inserted at offset");
}

async fn insert_audit_log_at(pool: &PgPool, user_id: Uuid, action: &str, days_ago: i64) {
    sqlx::query(
        "INSERT INTO audit_logs (user_id, action, entity_type, entity_id, changes, created_at)
         VALUES ($1, $2, 'users', $1, '{}'::jsonb, CURRENT_TIMESTAMP - ($3 || ' days')::INTERVAL)",
    )
    .bind(user_id)
    .bind(action)
    .bind(days_ago.to_string())
    .execute(pool)
    .await
    .expect("audit log inserted at offset");
}

async fn insert_export_request_with_status(pool: &PgPool, user_id: Uuid, status: &str) {
    sqlx::query(
        "INSERT INTO audit_export_requests (id, requested_by, status, note, report_type)
         VALUES ($1, $2, $3, 'Needs review', 'compliance_audit')",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(status)
    .execute(pool)
    .await
    .expect("export request inserted");
}

// ── 6.1-INT-019 HR recent_changes capped at RECENT_CHANGE_LIMIT (10) and DESC ─
#[sqlx::test(migrations = "../../migrations")]
async fn hr_recent_changes_limited_to_constant(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email("hr-recent");
    let hr_id = create_user_with_role(&pool, &hr_email, "hr").await;

    let dept_id = create_department(&pool, "RecentChangesDept", None).await;
    let resource_id = create_resource_in_dept(&pool, "Recent Subject", dept_id).await;
    create_ctc_for_resource(&pool, resource_id, hr_id).await;

    // Seed 12 revisions with decreasing seconds_ago so the most recent has revision_number = 12.
    for n in 1..=12_i32 {
        let seconds_ago = (13 - n) as i64 * 60; // 720, 660, 600, ..., 60
        add_ctc_revision_at(&pool, resource_id, hr_id, n, seconds_ago).await;
    }

    let token = login_token(&app, &hr_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let recent = body["hr"]["recent_changes"]
        .as_array()
        .expect("recent_changes array");

    assert_eq!(
        recent.len(),
        10,
        "recent_changes must be capped at RECENT_CHANGE_LIMIT=10"
    );

    // Must be ordered DESC by created_at → newest revision_number first.
    let revision_numbers: Vec<i64> = recent
        .iter()
        .map(|r| r["revision_number"].as_i64().unwrap_or(0))
        .collect();
    assert_eq!(
        revision_numbers[0], 12,
        "first entry must be the newest revision"
    );
    for window in revision_numbers.windows(2) {
        assert!(
            window[0] > window[1],
            "recent_changes must be strictly descending by revision_number, got {:?}",
            revision_numbers
        );
    }
}

// ── 6.1-INT-020 Finance audit alerts count chain-verification failures ────
#[sqlx::test(migrations = "../../migrations")]
async fn finance_audit_alerts_count_chain_verification_failures(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let fin_email = test_email("fin-chain");
    let fin_id = create_user_with_role(&pool, &fin_email, "finance").await;

    insert_audit_log(&pool, fin_id, "CHAIN_VERIFICATION_FAILED").await;
    insert_audit_log(&pool, fin_id, "VERIFY_CHAIN_FAILED").await;

    let token = login_token(&app, &fin_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let alerts = &body["finance"]["audit_alerts"];
    assert_eq!(
        alerts["chain_verification_failure_count"].as_i64().unwrap(),
        2,
        "chain_verification_failure_count must count both CHAIN_VERIFICATION_FAILED and VERIFY_CHAIN_FAILED actions"
    );

    let recent = alerts["recent"].as_array().expect("recent array");
    let actions: Vec<&str> = recent
        .iter()
        .map(|e| e["action"].as_str().unwrap_or(""))
        .collect();
    assert!(
        actions.contains(&"CHAIN_VERIFICATION_FAILED"),
        "recent list must surface CHAIN_VERIFICATION_FAILED rows, got {:?}",
        actions
    );
    assert!(
        actions.contains(&"VERIFY_CHAIN_FAILED"),
        "recent list must surface VERIFY_CHAIN_FAILED rows, got {:?}",
        actions
    );
}

// ── 6.1-INT-021 Finance audit alerts exclude events outside AUDIT_WINDOW_DAYS ─
#[sqlx::test(migrations = "../../migrations")]
async fn finance_audit_alerts_exclude_events_outside_window(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let fin_email = test_email("fin-window");
    let fin_id = create_user_with_role(&pool, &fin_email, "finance").await;

    // Inside the 7-day window
    insert_audit_log_at(&pool, fin_id, "ACCESS_DENIED", 1).await;
    // Outside the 7-day window — must be excluded
    insert_audit_log_at(&pool, fin_id, "ACCESS_DENIED", 30).await;
    insert_audit_log_at(&pool, fin_id, "LOGIN_FAILED", 14).await;

    let token = login_token(&app, &fin_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let alerts = &body["finance"]["audit_alerts"];
    assert_eq!(
        alerts["access_denied_count"].as_i64().unwrap(),
        1,
        "ACCESS_DENIED older than AUDIT_WINDOW_DAYS=7 must be excluded"
    );
    assert_eq!(
        alerts["login_failed_count"].as_i64().unwrap(),
        0,
        "LOGIN_FAILED at 14 days ago must be excluded from the 7-day window"
    );

    let recent = alerts["recent"].as_array().expect("recent array");
    assert_eq!(
        recent.len(),
        1,
        "only the in-window security event must appear in recent"
    );
}

// ── 6.1-INT-022 Finance export_requests filter on pending_approval only ───
#[sqlx::test(migrations = "../../migrations")]
async fn finance_export_requests_exclude_non_pending_status(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let fin_email = test_email("fin-export");
    let fin_id = create_user_with_role(&pool, &fin_email, "finance").await;

    insert_export_request_with_status(&pool, fin_id, "pending_approval").await;
    insert_export_request_with_status(&pool, fin_id, "approved").await;
    insert_export_request_with_status(&pool, fin_id, "rejected").await;

    let token = login_token(&app, &fin_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let exports = &body["finance"]["export_requests"];
    assert_eq!(
        exports["pending_count"].as_i64().unwrap(),
        1,
        "pending_count must count only status='pending_approval' rows"
    );

    let latest = exports["latest_pending"]
        .as_array()
        .expect("latest_pending array");
    assert_eq!(
        latest.len(),
        1,
        "latest_pending must include only pending_approval requests"
    );
    assert_eq!(
        latest[0]["status"].as_str().unwrap(),
        "pending_approval",
        "non-pending export requests must not leak into latest_pending"
    );
}

// ── 6.1-INT-023 PM active_projects bounded to PM_ACTIVE_PROJECT_LIMIT (10) ──
#[sqlx::test(migrations = "../../migrations")]
async fn project_manager_active_projects_bounded_to_ten(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let pm_email = test_email("pm-bounded");
    let pm_id = create_user_with_role(&pool, &pm_email, "project_manager").await;

    for i in 0..12 {
        create_project_with_pm(&pool, &format!("Bounded Project {:02}", i), pm_id).await;
    }

    let token = login_token(&app, &pm_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let projects = body["project_manager"]["active_projects"]
        .as_array()
        .expect("active_projects array");
    assert_eq!(
        projects.len(),
        10,
        "active_projects must be capped at PM_ACTIVE_PROJECT_LIMIT=10 to bound N+1 P&L lookups"
    );
}

// ── 6.1-INT-024 DH upcoming_assignments bounded + ordered ASC by start_date ─
#[sqlx::test(migrations = "../../migrations")]
async fn dept_head_upcoming_limited_and_ordered_by_start(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let dh_email = test_email("dh-upcoming-order");
    let dh_id = create_user_with_role(&pool, &dh_email, "department_head").await;

    let dept_id = create_department(&pool, "UpcomingDept", Some(dh_id)).await;
    set_department_head(&pool, dept_id, dh_id).await;
    assign_user_to_department(&pool, dh_id, dept_id).await;

    let resource_id = create_resource_in_dept(&pool, "Upcoming Resource", dept_id).await;
    let pm_id = create_user_with_role(&pool, &test_email("upcoming-pm"), "project_manager").await;
    let project_id = create_project_with_pm(&pool, "Upcoming Project", pm_id).await;

    // 12 allocations with decreasing start offsets (smaller offset = later).
    // Insert in reverse order to defeat any "insertion order" implicit sort.
    let starts: Vec<i64> = vec![60, 55, 50, 45, 40, 35, 30, 25, 20, 15, 10, 5];
    for &days in &starts {
        create_allocation(&pool, resource_id, project_id, 10.0, days, days + 30).await;
    }

    let token = login_token(&app, &dh_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let upcoming = body["department_head"]["upcoming_assignments"]
        .as_array()
        .expect("upcoming_assignments array");

    assert_eq!(
        upcoming.len(),
        10,
        "upcoming_assignments must be capped at DH_UPCOMING_LIMIT=10"
    );

    // ASC by start_date — earliest first.
    let start_dates: Vec<&str> = upcoming
        .iter()
        .map(|a| a["start_date"].as_str().unwrap_or(""))
        .collect();
    for window in start_dates.windows(2) {
        assert!(
            window[0] <= window[1],
            "upcoming_assignments must be ordered ASC by start_date, got {:?}",
            start_dates
        );
    }
}

// ── 6.1-INT-025 Admin counts exclude non-active projects ──────────────────
#[sqlx::test(migrations = "../../migrations")]
async fn admin_counts_exclude_non_active_projects(pool: PgPool) {
    set_test_env();
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email("admin-status");
    let _ = create_user_with_role(&pool, &admin_email, "admin").await;

    let pm_id = create_user_with_role(&pool, &test_email("status-pm"), "project_manager").await;

    // Capture baseline so seeded projects from other tests/migrations don't break the delta.
    let baseline = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM projects WHERE status IN ('Active', 'active')",
    )
    .fetch_one(&pool)
    .await
    .expect("baseline active project count");

    create_project_with_status(&pool, "Active Counted", pm_id, "Active").await;
    create_project_with_status(&pool, "Completed Excluded", pm_id, "Completed").await;
    create_project_with_status(&pool, "Cancelled Excluded", pm_id, "Cancelled").await;
    create_project_with_status(&pool, "Closed Excluded", pm_id, "Closed").await;

    let token = login_token(&app, &admin_email).await;
    let (status, body) = get_dashboard(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    let count = body["admin"]["total_active_projects"].as_i64().unwrap();
    assert_eq!(
        count - baseline,
        1,
        "admin total_active_projects must increase by exactly 1 (only the Active project), got delta = {}",
        count - baseline
    );
}
