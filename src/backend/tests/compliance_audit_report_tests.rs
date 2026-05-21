//! Integration tests for Story 5.4 Compliance Audit Reports.
//!
//! Exercises `/api/v1/audit-logs/reports` and the report-aware export endpoint
//! built on top of `audit_logs` + `audit_export_requests` + `ctc_revisions`.

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::{json, Value};
use sqlx::PgPool;
use sqlx::Row;
use tower::ServiceExt;
use uuid::Uuid;
use xynergy_backend::routes::Claims;

fn test_email() -> String {
    format!("audit-rep-{}@example.com", Uuid::new_v4())
}

fn set_test_env() {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );
}

async fn create_department(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("INSERT INTO departments (name) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("department created")
}

async fn create_user(pool: &PgPool, email: &str, role: &str, department_id: Option<Uuid>) -> Uuid {
    let password_hash = xynergy_backend::routes::auth::hash_password("Password123!")
        .expect("password hashing should succeed");
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, first_name, last_name, role, department_id)
         VALUES ($1, $2, 'Audit', 'Tester', $3, $4)
         RETURNING id",
    )
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .bind(department_id)
    .fetch_one(pool)
    .await
    .expect("user created")
}

async fn create_resource(pool: &PgPool, name: &str, department_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO resources (name, resource_type, capacity, department_id)
         VALUES ($1, 'employee', 1.0, $2)
         RETURNING id",
    )
    .bind(name)
    .bind(department_id)
    .fetch_one(pool)
    .await
    .expect("resource created")
}

async fn create_project(pool: &PgPool, name: &str, pm_id: Uuid) -> Uuid {
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

async fn get_token(app: &axum::Router, email: &str) -> String {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"email": email, "password": "Password123!"}).to_string(),
        ))
        .expect("login request should build");

    let resp = app.clone().oneshot(req).await.expect("login response");
    assert_eq!(resp.status(), StatusCode::OK, "login should succeed");
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    body["token"].as_str().unwrap().to_string()
}

fn invalid_subject_token(role: &str) -> String {
    let claims = Claims {
        sub: "not-a-uuid".to_string(),
        email: "invalid-subject@example.com".to_string(),
        role: role.to_string(),
        department_id: None,
        exp: (chrono::Utc::now() + chrono::Duration::minutes(15)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("test-secret".as_bytes()),
    )
    .expect("invalid-subject test token should encode")
}

async fn build_app(pool: &PgPool) -> axum::Router {
    set_test_env();
    xynergy_backend::create_app(pool.clone())
}

async fn create_ctc_via_api(app: &axum::Router, hr_token: &str, resource_id: Uuid) {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": 15_000_000,
                "hra_allowance": 3_000_000,
                "medical_allowance": 1_000_000,
                "transport_allowance": 500_000,
                "meal_allowance": 500_000,
                "working_days_per_month": 22,
                "risk_tier": 1
            })
            .to_string(),
        ))
        .expect("ctc create request");
    let resp = app.clone().oneshot(req).await.expect("ctc create response");
    assert_eq!(resp.status(), StatusCode::OK, "ctc create should succeed");
}

async fn build_components(app: &axum::Router, hr_token: &str, resource_id: Uuid) -> Value {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc/calculate")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "resource_id": resource_id.to_string(),
                "base_salary": 16_000_000,
                "hra_allowance": 3_000_000,
                "medical_allowance": 1_000_000,
                "transport_allowance": 500_000,
                "meal_allowance": 500_000,
                "working_days_per_month": 22,
                "risk_tier": 1
            })
            .to_string(),
        ))
        .expect("calculate request");
    let resp = app.clone().oneshot(req).await.expect("calc response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let preview: Value = serde_json::from_slice(&bytes).unwrap();

    json!({
        "base_salary": preview["base_salary"],
        "hra_allowance": preview["allowances"]["hra"],
        "medical_allowance": preview["allowances"]["medical"],
        "transport_allowance": preview["allowances"]["transport"],
        "meal_allowance": preview["allowances"]["meal"],
        "bpjs_kesehatan_employer": preview["bpjs"]["kesehatan"]["employer"],
        "bpjs_ketenagakerjaan_employer": preview["bpjs"]["ketenagakerjaan"]["employer"],
        "thr_monthly_accrual": preview["thr_monthly_accrual"],
        "total_monthly_ctc": preview["total_monthly_ctc"],
        "daily_rate": format!("{:.2}", preview["daily_rate"].as_f64().unwrap_or(0.0)),
        "working_days_per_month": preview["working_days_per_month"],
        "risk_tier": 1,
        "thr_eligible": true,
    })
}

async fn update_ctc_via_api(
    app: &axum::Router,
    hr_token: &str,
    resource_id: Uuid,
    components: &Value,
    reason: &str,
) {
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/ctc/{}/components", resource_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "components": components,
                "reason": reason
            })
            .to_string(),
        ))
        .expect("ctc update request");
    let resp = app.clone().oneshot(req).await.expect("ctc update response");
    assert_eq!(resp.status(), StatusCode::OK, "ctc update should succeed");
}

// 90-day window ending today. Kept inside REPORT_MAX_RANGE_DAYS so tests that
// span "everything we just inserted" still pass after the interactive cap.
fn current_year_dates() -> (String, String) {
    let today = chrono::Utc::now().date_naive();
    let start = today - chrono::Duration::days(89);
    (
        start.format("%Y-%m-%d").to_string(),
        today.format("%Y-%m-%d").to_string(),
    )
}

fn report_i64(value: &Value) -> Option<i64> {
    if let Some(value) = value.as_i64() {
        return Some(value);
    }
    if let Some(value) = value.as_f64() {
        return Some(value.round() as i64);
    }
    value
        .as_str()
        .and_then(|raw| raw.parse::<f64>().ok())
        .map(|value| value.round() as i64)
}

async fn reports_request(
    app: &axum::Router,
    token: &str,
    report_type: &str,
    start_date: &str,
    end_date: &str,
    extra_query: &str,
) -> (StatusCode, Value) {
    let url = format!(
        "/api/v1/audit-logs/reports?report_type={}&start_date={}&end_date={}&limit=100{}",
        report_type, start_date, end_date, extra_query
    );
    let req = Request::builder()
        .method("GET")
        .uri(url)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("reports request");
    let resp = app.clone().oneshot(req).await.expect("reports response");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

#[sqlx::test(migrations = "../../migrations")]
async fn finance_can_generate_each_report_type(pool: PgPool) {
    let app = build_app(&pool).await;
    let dept = create_department(&pool, "Finance Dept").await;
    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance", Some(dept)).await;
    let token = get_token(&app, &finance_email).await;

    let (start, end) = current_year_dates();
    for report_type in [
        "ctc_change_log",
        "assignment_history",
        "budget_modifications",
        "access_logs",
    ] {
        let (status, body) = reports_request(&app, &token, report_type, &start, &end, "").await;
        assert_eq!(
            status,
            StatusCode::OK,
            "finance should access {}",
            report_type
        );
        assert_eq!(body["report_type"], report_type);
        assert!(body["rows"].is_object(), "rows wrapper for {}", report_type);
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_generate_each_report_type(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let (start, end) = current_year_dates();
    for report_type in [
        "ctc_change_log",
        "assignment_history",
        "budget_modifications",
        "access_logs",
    ] {
        let (status, _) = reports_request(&app, &token, report_type, &start, &end, "").await;
        assert_eq!(
            status,
            StatusCode::OK,
            "admin should access {}",
            report_type
        );
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_finance_roles_are_denied(pool: PgPool) {
    let app = build_app(&pool).await;
    let dept = create_department(&pool, "Ops").await;
    let (start, end) = current_year_dates();
    for role in ["hr", "department_head", "project_manager"] {
        let email = test_email();
        let _ = create_user(&pool, &email, role, Some(dept)).await;
        let token = get_token(&app, &email).await;

        for report_type in [
            "ctc_change_log",
            "assignment_history",
            "budget_modifications",
            "access_logs",
        ] {
            let (status, _) = reports_request(&app, &token, report_type, &start, &end, "").await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "{} should not access {}",
                role,
                report_type
            );
        }

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/audit-logs/export")
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "report_type": "access_logs",
                    "start_date": start.clone(),
                    "end_date": end.clone(),
                })
                .to_string(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "{} should not request report exports",
            role
        );
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn inverted_date_range_returns_400(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let (status, _) = reports_request(
        &app,
        &token,
        "ctc_change_log",
        "2026-12-01",
        "2026-01-01",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn date_range_exceeding_max_returns_400(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let today = chrono::Utc::now().date_naive();
    let allowed_start = today - chrono::Duration::days(89);
    let allowed_start_str = allowed_start.format("%Y-%m-%d").to_string();
    let end_str = today.format("%Y-%m-%d").to_string();
    let (status, _) = reports_request(
        &app,
        &token,
        "access_logs",
        &allowed_start_str,
        &end_str,
        "",
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "90 inclusive days should be accepted"
    );

    // 91-day window: one day past REPORT_MAX_RANGE_DAYS (90).
    let start = today - chrono::Duration::days(90);
    let start_str = start.format("%Y-%m-%d").to_string();

    for report_type in [
        "ctc_change_log",
        "assignment_history",
        "budget_modifications",
        "access_logs",
    ] {
        let (status, body) =
            reports_request(&app, &token, report_type, &start_str, &end_str, "").await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "{} should reject windows over the cap",
            report_type
        );
        let message = body["error"]["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("exceeds maximum"),
            "{} error message should mention the cap, got: {}",
            report_type,
            body
        );
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn offset_page_without_snapshot_returns_400(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let (start, end) = current_year_dates();
    let (status, body) =
        reports_request(&app, &token, "access_logs", &start, &end, "&offset=100").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    let message = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("snapshot_at is required"),
        "offset pagination should require the original snapshot, got {message}"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn ctc_change_log_returns_expected_fields(pool: PgPool) {
    let app = build_app(&pool).await;
    let dept = create_department(&pool, "Engineering").await;
    let hr_email = test_email();
    let hr_id = create_user(&pool, &hr_email, "hr", Some(dept)).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let hr_token = get_token(&app, &hr_email).await;
    let admin_token = get_token(&app, &admin_email).await;

    let resource_id = create_resource(&pool, "Eve Engineer", dept).await;

    create_ctc_via_api(&app, &hr_token, resource_id).await;
    let components = build_components(&app, &hr_token, resource_id).await;
    update_ctc_via_api(&app, &hr_token, resource_id, &components, "Annual raise").await;

    let (start, end) = current_year_dates();
    let (status, body) =
        reports_request(&app, &admin_token, "ctc_change_log", &start, &end, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["report_type"], "ctc_change_log");

    let rows = body["rows"]["data"].as_array().expect("rows array");
    assert!(!rows.is_empty(), "expected at least one CTC change row");

    // Find the base_salary change which we explicitly induced (15M -> 16M).
    let base = rows
        .iter()
        .find(|r| r["field"] == "base_salary")
        .expect("base_salary diff row should exist");
    assert_eq!(base["employee_name"], "Eve Engineer");
    assert_eq!(base["reason"], "Annual raise");
    assert!(base["change_date"].is_string());
    assert_eq!(base["changed_by_id"], Value::String(hr_id.to_string()));
    assert_eq!(report_i64(&base["old_value"]), Some(15_000_000));
    assert_eq!(report_i64(&base["new_value"]), Some(16_000_000));
}

#[sqlx::test(migrations = "../../migrations")]
async fn access_logs_filter_by_user_and_action(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    // Hitting the audit-logs endpoint generates VIEW_AUDIT_REPORT entries.
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-logs?limit=5")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    let (start, end) = current_year_dates();
    let extra = format!("&action_type=VIEW_AUDIT_REPORT&user_id={}", admin_id);
    let (status, body) = reports_request(&app, &token, "access_logs", &start, &end, &extra).await;
    assert_eq!(status, StatusCode::OK);

    let rows = body["rows"]["data"].as_array().expect("access log rows");
    assert!(!rows.is_empty(), "expected at least one access log row");
    for row in rows {
        assert_eq!(row["action"], "VIEW_AUDIT_REPORT");
        assert_eq!(row["success"], Value::Bool(true));
        assert_eq!(row["user_id"], Value::String(admin_id.to_string()));
        assert_eq!(row["user_name"], Value::String("Audit Tester".to_string()));
        assert_eq!(row["resource_type"], "audit_logs");
        assert_eq!(row["resource_id"], Value::String(admin_id.to_string()));
        assert!(row["timestamp"].is_string());
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn assignment_history_surfaces_allocation_audit_rows(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let admin_token = get_token(&app, &admin_email).await;

    let dept = create_department(&pool, "Delivery").await;
    let resource_id = create_resource(&pool, "Allie Allocator", dept).await;
    let project_id = create_project(&pool, "Project Phoenix", admin_id).await;

    let allocation_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO allocations (resource_id, project_id, allocation_percentage, start_date, end_date)
         VALUES ($1, $2, 50, CURRENT_DATE, CURRENT_DATE + INTERVAL '30 days')
         RETURNING id",
    )
    .bind(resource_id)
    .bind(project_id)
    .fetch_one(&pool)
    .await
    .expect("allocation created");

    xynergy_backend::services::log_audit(
        &pool,
        Some(admin_id),
        "CREATE_ALLOCATION",
        "allocation",
        allocation_id,
        json!({
            "before": null,
            "after": {
                "allocation_percentage": 50,
                "resource_id": resource_id,
                "project_id": project_id,
            }
        }),
    )
    .await
    .expect("audit log inserted");

    let (start, end) = current_year_dates();
    let (status, body) =
        reports_request(&app, &admin_token, "assignment_history", &start, &end, "").await;
    assert_eq!(status, StatusCode::OK);

    let rows = body["rows"]["data"].as_array().expect("assignment rows");
    assert!(!rows.is_empty(), "expected at least one assignment row");
    let first = &rows[0];
    assert_eq!(first["action"], "CREATE_ALLOCATION");
    assert_eq!(first["resource_id"], Value::String(resource_id.to_string()));
    assert_eq!(first["project_id"], Value::String(project_id.to_string()));
    assert!(first["before_summary"].is_string());
    assert!(first["after_summary"]
        .as_str()
        .unwrap_or_default()
        .contains("allocation_percentage"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn budget_modifications_surfaces_project_budget_rows(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let admin_token = get_token(&app, &admin_email).await;

    let project_id = create_project(&pool, "Project Apollo", admin_id).await;

    xynergy_backend::services::log_audit(
        &pool,
        Some(admin_id),
        "UPDATE_BUDGET",
        "project_budget",
        project_id,
        json!({
            "before": { "total_budget": 100000000 },
            "after": { "total_budget": 120000000 }
        }),
    )
    .await
    .unwrap();

    let (start, end) = current_year_dates();
    let (status, body) =
        reports_request(&app, &admin_token, "budget_modifications", &start, &end, "").await;
    assert_eq!(status, StatusCode::OK);

    let rows = body["rows"]["data"].as_array().expect("budget rows");
    assert!(!rows.is_empty(), "expected budget rows");
    let first = &rows[0];
    assert_eq!(first["action"], "UPDATE_BUDGET");
    assert_eq!(first["project_id"], Value::String(project_id.to_string()));
    assert_eq!(first["project_name"], "Project Apollo");
    assert!(
        first.get("before").is_none(),
        "raw before payload must not be part of the report contract"
    );
    assert!(
        first.get("after").is_none(),
        "raw after payload must not be part of the report contract"
    );
    assert!(first["before_summary"]
        .as_str()
        .unwrap_or_default()
        .contains("total_budget"));
    assert!(first["after_summary"]
        .as_str()
        .unwrap_or_default()
        .contains("120000000"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn export_request_persists_with_report_metadata(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let (start, end) = current_year_dates();
    let preview_snapshot_at = chrono::Utc::now() - chrono::Duration::minutes(5);
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "report_type": "ctc_change_log",
                "start_date": start,
                "end_date": end,
                "snapshot_at": preview_snapshot_at,
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["status"], "pending_approval");
    assert_eq!(body["report_type"], "ctc_change_log");
    assert!(body["watermark"]["text"]
        .as_str()
        .unwrap_or("")
        .contains("Xynergy audit export"));

    let export_id = Uuid::parse_str(body["export_id"].as_str().unwrap()).unwrap();
    let row = sqlx::query(
        "SELECT requested_by, status, report_type, filters, watermark
         FROM audit_export_requests WHERE id = $1",
    )
    .bind(export_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let requested_by: Uuid = row.try_get("requested_by").unwrap();
    let status: String = row.try_get("status").unwrap();
    let report_type: Option<String> = row.try_get("report_type").unwrap();
    let filters: Option<serde_json::Value> = row.try_get("filters").unwrap();
    let watermark: Option<serde_json::Value> = row.try_get("watermark").unwrap();

    assert_eq!(requested_by, finance_id);
    assert_eq!(status, "pending_approval");
    assert_eq!(report_type.as_deref(), Some("ctc_change_log"));
    assert_eq!(
        filters
            .as_ref()
            .and_then(|v| v.get("start_date"))
            .map(|v| v.as_str().unwrap_or_default().to_string()),
        Some(start.clone())
    );
    let persisted_snapshot_at = filters
        .as_ref()
        .and_then(|v| v.get("snapshot_at"))
        .and_then(|v| v.as_str())
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));
    assert_eq!(
        persisted_snapshot_at,
        Some(preview_snapshot_at),
        "interactive export must preserve the reviewed preview snapshot"
    );
    assert_eq!(
        filters.as_ref().and_then(|v| v.get("scope")),
        Some(&Value::from("all_matching_rows"))
    );
    assert!(filters.as_ref().and_then(|v| v.get("limit")).is_none());
    assert!(filters.as_ref().and_then(|v| v.get("offset")).is_none());
    assert!(watermark
        .as_ref()
        .and_then(|v| v.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .contains("Xynergy audit export"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn export_accepts_wide_date_range_for_approval_workflow(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let today = chrono::Utc::now().date_naive();
    let start = (today - chrono::Duration::days(180))
        .format("%Y-%m-%d")
        .to_string();
    let end = today.format("%Y-%m-%d").to_string();

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "report_type": "access_logs",
                "start_date": start,
                "end_date": end,
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_export_requests WHERE requested_by = $1")
            .bind(finance_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1, "wide export requests should persist for approval");
}

#[sqlx::test(migrations = "../../migrations")]
async fn export_watermark_sanitizes_action_filter_delimiters(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let (start, end) = current_year_dates();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "report_type": "access_logs",
                "start_date": start,
                "end_date": end,
                "action_type": "LOGIN_SUCCESS|spoof\nnext",
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let watermark = body["watermark"]["text"].as_str().unwrap();

    assert!(!watermark.contains("LOGIN_SUCCESS|spoof"));
    assert!(!watermark.contains('\n'));
    assert!(watermark.contains("filter_action_type=LOGIN_SUCCESS spoof next"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn export_rejects_pagination_parameters(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let (start, end) = current_year_dates();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "report_type": "ctc_change_log",
                "start_date": start,
                "end_date": end,
                "limit": 100,
                "offset": 25,
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_export_requests WHERE requested_by = $1")
            .bind(finance_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0, "paginated export payloads must not persist");
}

#[sqlx::test(migrations = "../../migrations")]
async fn export_request_without_payload_remains_backward_compatible(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let _finance_id = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["status"], "pending_approval");
    assert!(body.get("report_type").is_none());
    assert!(body.get("watermark").is_none());

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "empty JSON body remains backward-compatible"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn report_generation_creates_audit_log_entry(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let (start, end) = current_year_dates();
    let (status, _body) = reports_request(&app, &token, "access_logs", &start, &end, "").await;
    assert_eq!(status, StatusCode::OK);

    let row = sqlx::query(
        "SELECT action FROM audit_logs
         WHERE user_id = $1 AND action = 'COMPLIANCE_AUDIT_REPORT_GENERATED'
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(admin_id)
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(row.is_some(), "report generation should be audited");
}

#[sqlx::test(migrations = "../../migrations")]
async fn export_request_creates_audit_log_entry(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let (start, end) = current_year_dates();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "report_type": "access_logs",
                "start_date": start,
                "end_date": end,
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let row = sqlx::query(
        "SELECT changes FROM audit_logs
         WHERE user_id = $1 AND action = 'EXPORT_REQUESTED'
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(finance_id)
    .fetch_optional(&pool)
    .await
    .unwrap();
    let row = row.expect("export request should be audited");
    let changes: serde_json::Value = row.try_get("changes").unwrap();
    assert_eq!(changes["workflow"], "four_eyes_approval");
    assert_eq!(changes["status"], "pending_approval");
}

#[sqlx::test(migrations = "../../migrations")]
async fn legacy_audit_logs_endpoint_still_returns_200(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-logs?limit=5")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn legacy_audit_verify_endpoint_still_returns_200(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-logs/verify")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_report_type_returns_400(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let (start, end) = current_year_dates();
    let (status, _) = reports_request(&app, &token, "not_a_report", &start, &end, "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ---------------- Expanded coverage (Story 5.4 automation pass) ----------------

// G1 (P0): failure actions must be classified success=false in access logs.
#[sqlx::test(migrations = "../../migrations")]
async fn access_logs_classify_failure_actions_as_unsuccessful(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    for action in [
        "LOGIN_FAILED",
        "LOGIN_BLOCKED",
        "ACCOUNT_LOCKED",
        "ACCESS_DENIED",
    ] {
        xynergy_backend::services::log_audit(
            &pool,
            Some(admin_id),
            action,
            "users",
            admin_id,
            json!({"reason": "test_injected"}),
        )
        .await
        .expect("audit insert succeeded");
    }

    let (start, end) = current_year_dates();
    for action in [
        "LOGIN_FAILED",
        "LOGIN_BLOCKED",
        "ACCOUNT_LOCKED",
        "ACCESS_DENIED",
    ] {
        let extra = format!("&action_type={}", action);
        let (status, body) =
            reports_request(&app, &token, "access_logs", &start, &end, &extra).await;
        assert_eq!(status, StatusCode::OK, "{} should be queryable", action);
        let rows = body["rows"]["data"].as_array().expect("rows array");
        assert!(!rows.is_empty(), "expected at least one {} row", action);
        for row in rows {
            assert_eq!(row["action"], action);
            assert_eq!(
                row["success"],
                Value::Bool(false),
                "{} must classify success=false",
                action
            );
        }
    }
}

// G2 (P0): denied report access must produce an ACCESS_DENIED audit log row.
#[sqlx::test(migrations = "../../migrations")]
async fn denied_report_access_creates_access_denied_audit_entry(pool: PgPool) {
    let app = build_app(&pool).await;
    let dept = create_department(&pool, "People Ops").await;
    let hr_email = test_email();
    let hr_id = create_user(&pool, &hr_email, "hr", Some(dept)).await;
    let token = get_token(&app, &hr_email).await;

    let (start, end) = current_year_dates();
    let (status, _) = reports_request(&app, &token, "ctc_change_log", &start, &end, "").await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let row = sqlx::query(
        "SELECT changes FROM audit_logs
         WHERE user_id = $1 AND action = 'ACCESS_DENIED' AND entity_type = 'audit_report'
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(hr_id)
    .fetch_optional(&pool)
    .await
    .unwrap();

    let row = row.expect("ACCESS_DENIED entry must be logged for the denied attempt");
    let changes: serde_json::Value = row.try_get("changes").unwrap();
    assert_eq!(changes["reason"], "insufficient_role");
    assert_eq!(changes["attempted_role"], "hr");
}

// G3 (P0): unauthenticated callers must be rejected with 401 on both new endpoints.
#[sqlx::test(migrations = "../../migrations")]
async fn unauthenticated_requests_return_401(pool: PgPool) {
    let app = build_app(&pool).await;
    let (start, end) = current_year_dates();

    let reports_req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/audit-logs/reports?report_type=access_logs&start_date={}&end_date={}",
            start, end
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(reports_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let export_req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(export_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs
         WHERE user_id IS NULL
           AND action = 'ACCESS_DENIED'
           AND entity_type IN ('audit_report', 'audit_export_request')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        count, 2,
        "unauthenticated report/export attempts should be audited"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_subject_token_creates_access_denied_audit_entry(pool: PgPool) {
    let app = build_app(&pool).await;
    let (start, end) = current_year_dates();
    let token = invalid_subject_token("finance");

    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/audit-logs/reports?report_type=access_logs&start_date={}&end_date={}",
            start, end
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let row = sqlx::query(
        "SELECT changes FROM audit_logs
         WHERE user_id IS NULL AND action = 'ACCESS_DENIED' AND entity_type = 'audit_report'
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    let row = row.expect("invalid-subject denial should be audited");
    let changes: serde_json::Value = row.try_get("changes").unwrap();
    assert_eq!(changes["reason"], "invalid_subject");
}

// G4 (P0): CTC change log must not leak any encryption metadata or ciphertext.
#[sqlx::test(migrations = "../../migrations")]
async fn ctc_change_log_never_exposes_encryption_metadata(pool: PgPool) {
    let app = build_app(&pool).await;
    let dept = create_department(&pool, "Security").await;
    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr", Some(dept)).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let hr_token = get_token(&app, &hr_email).await;
    let admin_token = get_token(&app, &admin_email).await;

    let resource_id = create_resource(&pool, "Vault Subject", dept).await;
    create_ctc_via_api(&app, &hr_token, resource_id).await;
    let components = build_components(&app, &hr_token, resource_id).await;
    update_ctc_via_api(&app, &hr_token, resource_id, &components, "Adjustment").await;

    let (start, end) = current_year_dates();
    let (status, body) =
        reports_request(&app, &admin_token, "ctc_change_log", &start, &end, "").await;
    assert_eq!(status, StatusCode::OK);

    let raw = body.to_string();
    for forbidden in [
        "encrypted_components",
        "encrypted_daily_rate",
        "key_version",
        "encryption_version",
        "encryption_algorithm",
        "encrypted_at",
        "ciphertext",
    ] {
        assert!(
            !raw.contains(forbidden),
            "response leaked encryption field: {}",
            forbidden
        );
    }
}

// G5 (P1): user_id / action_type filters are rejected for non-access-log reports.
#[sqlx::test(migrations = "../../migrations")]
async fn filter_combinations_only_supported_for_access_logs(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let (start, end) = current_year_dates();
    for rt in [
        "ctc_change_log",
        "assignment_history",
        "budget_modifications",
    ] {
        let extra_user = format!("&user_id={}", admin_id);
        let (status, _) = reports_request(&app, &token, rt, &start, &end, &extra_user).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "{} must reject user_id filter",
            rt
        );

        let (status, _) =
            reports_request(&app, &token, rt, &start, &end, "&action_type=VIEW").await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "{} must reject action_type filter",
            rt
        );
    }
}

// G6 (P1): malformed date string must return 400 instead of 500.
#[sqlx::test(migrations = "../../migrations")]
async fn malformed_date_returns_400(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let (status, _) =
        reports_request(&app, &token, "access_logs", "2026/01/01", "2026-12-31", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = reports_request(&app, &token, "access_logs", "yesterday", "today", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// G7 (P1): missing or empty required dates must return 400.
#[sqlx::test(migrations = "../../migrations")]
async fn missing_or_empty_dates_return_400(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-logs/reports?report_type=access_logs&end_date=2026-12-31")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let (status, _) = reports_request(&app, &token, "access_logs", "", "2026-12-31", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// G8 (P1): pagination — has_more=true and offset actually skips rows.
#[sqlx::test(migrations = "../../migrations")]
async fn pagination_reports_has_more_and_offset_works(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    // Seed 5 access-log rows so a limit=2 page leaves more behind.
    for i in 0..5 {
        xynergy_backend::services::log_audit(
            &pool,
            Some(admin_id),
            "LOGIN_SUCCESS",
            "users",
            admin_id,
            json!({"seq": i}),
        )
        .await
        .unwrap();
    }

    let (start, end) = current_year_dates();
    let page1_url = format!(
        "/api/v1/audit-logs/reports?report_type=access_logs&start_date={}&end_date={}&limit=2&offset=0&action_type=LOGIN_SUCCESS",
        start, end
    );
    let req = Request::builder()
        .method("GET")
        .uri(page1_url)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let page1: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(page1["limit"], 2);
    assert_eq!(page1["offset"], 0);
    assert_eq!(page1["has_more"], Value::Bool(true));
    let page1_rows = page1["rows"]["data"].as_array().unwrap();
    assert_eq!(page1_rows.len(), 2);
    let snapshot_at = page1["snapshot_at"]
        .as_str()
        .expect("snapshot_at should be returned");

    let page2_url = format!(
        "/api/v1/audit-logs/reports?report_type=access_logs&start_date={}&end_date={}&limit=2&offset=2&action_type=LOGIN_SUCCESS&snapshot_at={}",
        start, end, snapshot_at
    );
    let req = Request::builder()
        .method("GET")
        .uri(page2_url)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let page2: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(page2["offset"], 2);
    let page2_rows = page2["rows"]["data"].as_array().unwrap();
    assert_eq!(page2_rows.len(), 2);

    let page1_ids: Vec<&str> = page1_rows
        .iter()
        .map(|r| r["audit_id"].as_str().unwrap())
        .collect();
    let page2_ids: Vec<&str> = page2_rows
        .iter()
        .map(|r| r["audit_id"].as_str().unwrap())
        .collect();
    for id in &page2_ids {
        assert!(
            !page1_ids.contains(id),
            "offset failed to skip — id {} appears on both pages",
            id
        );
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn access_log_pagination_uses_stable_snapshot(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    for i in 0..4 {
        xynergy_backend::services::log_audit(
            &pool,
            Some(admin_id),
            "LOGIN_SUCCESS",
            "users",
            admin_id,
            json!({"stable_seq": i}),
        )
        .await
        .unwrap();
    }

    let (start, end) = current_year_dates();
    let page1_url = format!(
        "/api/v1/audit-logs/reports?report_type=access_logs&start_date={}&end_date={}&limit=2&offset=0&action_type=LOGIN_SUCCESS",
        start, end
    );
    let req = Request::builder()
        .method("GET")
        .uri(page1_url)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let page1: Value = serde_json::from_slice(&bytes).unwrap();
    let snapshot_at = page1["snapshot_at"]
        .as_str()
        .expect("snapshot_at should be returned");
    let page1_rows = page1["rows"]["data"].as_array().unwrap();
    assert_eq!(page1_rows.len(), 2);

    xynergy_backend::services::log_audit(
        &pool,
        Some(admin_id),
        "LOGIN_SUCCESS",
        "users",
        admin_id,
        json!({"reason": "after_snapshot"}),
    )
    .await
    .unwrap();

    let page2_url = format!(
        "/api/v1/audit-logs/reports?report_type=access_logs&start_date={}&end_date={}&limit=2&offset=2&action_type=LOGIN_SUCCESS&snapshot_at={}",
        start, end, snapshot_at
    );
    let req = Request::builder()
        .method("GET")
        .uri(page2_url)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let page2: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(page2["snapshot_at"], snapshot_at);
    let page2_rows = page2["rows"]["data"].as_array().unwrap();
    assert!(
        page2_rows
            .iter()
            .all(|row| row["reason"] != Value::String("after_snapshot".to_string())),
        "rows inserted after page 1 must be excluded by the original snapshot"
    );

    let page1_ids: Vec<&str> = page1_rows
        .iter()
        .map(|row| row["audit_id"].as_str().unwrap())
        .collect();
    for row in page2_rows {
        let audit_id = row["audit_id"].as_str().unwrap();
        assert!(
            !page1_ids.contains(&audit_id),
            "snapshot pagination duplicated audit id {}",
            audit_id
        );
    }
}

// G9 (P1): limit clamps into the safe 1..=200 range.
#[sqlx::test(migrations = "../../migrations")]
async fn limit_clamps_to_safe_range(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let (start, end) = current_year_dates();

    let url = format!(
        "/api/v1/audit-logs/reports?report_type=access_logs&start_date={}&end_date={}&limit=0",
        start, end
    );
    let req = Request::builder()
        .method("GET")
        .uri(url)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["limit"], 1);

    let url = format!(
        "/api/v1/audit-logs/reports?report_type=access_logs&start_date={}&end_date={}&limit=9999",
        start, end
    );
    let req = Request::builder()
        .method("GET")
        .uri(url)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["limit"], 200);
}

// G10 (P1): watermark text format guarantees every auditor-required field is present.
#[sqlx::test(migrations = "../../migrations")]
async fn watermark_text_includes_all_required_fields(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let (start, end) = current_year_dates();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "report_type": "assignment_history",
                "start_date": start,
                "end_date": end,
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();

    let watermark_text = body["watermark"]["text"].as_str().unwrap();
    assert!(watermark_text.starts_with("Xynergy audit export"));
    assert!(watermark_text.contains(&format!("requested_by={}", finance_id)));
    assert!(watermark_text.contains("requested_at="));
    assert!(watermark_text.contains("snapshot_at="));
    assert!(watermark_text.contains("report_type=assignment_history"));
    assert!(watermark_text.contains(&format!("window={}..{}", start, end)));
    let export_id = body["export_id"].as_str().unwrap();
    assert!(watermark_text.contains(&format!("export_id={}", export_id)));
}

#[sqlx::test(migrations = "../../migrations")]
async fn access_log_export_watermark_includes_filters(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let (start, end) = current_year_dates();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "report_type": "access_logs",
                "start_date": start,
                "end_date": end,
                "user_id": finance_id,
                "action_type": "LOGIN_SUCCESS",
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let watermark_text = body["watermark"]["text"].as_str().unwrap();
    assert!(watermark_text.contains(&format!("filter_user_id={}", finance_id)));
    assert!(watermark_text.contains("filter_action_type=LOGIN_SUCCESS"));
}

// G11 (P1): partial export payload — report_type without dates must be rejected.
#[sqlx::test(migrations = "../../migrations")]
async fn export_with_report_type_but_missing_dates_returns_400(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"report_type": "ctc_change_log"}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// G12 (P1): export with inverted date range must be rejected before any side-effect persists.
#[sqlx::test(migrations = "../../migrations")]
async fn export_with_inverted_date_range_returns_400(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "report_type": "ctc_change_log",
                "start_date": "2026-12-01",
                "end_date": "2026-01-01",
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_export_requests WHERE requested_by = $1")
            .bind(finance_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count, 0,
        "no export request should persist on validation failure"
    );
}

// G13 (P2): empty window returns empty rows array, not null/error.
#[sqlx::test(migrations = "../../migrations")]
async fn empty_window_returns_empty_rows_array(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    // Far-past window guaranteed to contain no rows.
    let (status, body) = reports_request(
        &app,
        &token,
        "assignment_history",
        "1990-01-01",
        "1990-01-31",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["rows"]["data"]
        .as_array()
        .expect("data must be an array");
    assert!(rows.is_empty(), "expected zero rows for empty window");
    assert_eq!(body["has_more"], Value::Bool(false));
}

// G14 (P2): same-day boundary range is accepted.
#[sqlx::test(migrations = "../../migrations")]
async fn same_day_range_is_accepted(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let today = chrono::Utc::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let (status, body) = reports_request(&app, &token, "access_logs", &today, &today, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["start_date"], today);
    assert_eq!(body["end_date"], today);
}

// G15 (P2): export request with invalid report_type must be rejected.
#[sqlx::test(migrations = "../../migrations")]
async fn export_with_invalid_report_type_returns_400(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let _ = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let (start, end) = current_year_dates();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "report_type": "not_a_real_type",
                "start_date": start,
                "end_date": end,
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn export_rejects_non_access_log_user_action_filters(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let (start, end) = current_year_dates();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "report_type": "ctc_change_log",
                "start_date": start,
                "end_date": end,
                "user_id": finance_id,
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_export_requests WHERE requested_by = $1")
            .bind(finance_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0, "invalid export metadata must not persist");
}

#[sqlx::test(migrations = "../../migrations")]
async fn export_rejects_partial_filter_metadata_without_report_type(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let (start, end) = current_year_dates();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "start_date": start,
                "end_date": end,
                "snapshot_at": chrono::Utc::now(),
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_export_requests WHERE requested_by = $1")
            .bind(finance_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count, 0,
        "partial metadata must not persist without report_type"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn export_rejects_unknown_json_fields(pool: PgPool) {
    let app = build_app(&pool).await;
    let finance_email = test_email();
    let finance_id = create_user(&pool, &finance_email, "finance", None).await;
    let token = get_token(&app, &finance_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(json!({"reportType": "access_logs"}).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_export_requests WHERE requested_by = $1")
            .bind(finance_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0, "unknown export fields must not persist");
}

#[sqlx::test(migrations = "../../migrations")]
async fn same_day_range_uses_jakarta_business_day_boundaries(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let day = chrono::Utc::now().date_naive() - chrono::Duration::days(1);
    let start_timestamp = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
        (day - chrono::Duration::days(1))
            .and_hms_milli_opt(17, 0, 0, 0)
            .unwrap(),
        chrono::Utc,
    );
    let end_timestamp = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
        day.and_hms_milli_opt(16, 59, 59, 500).unwrap(),
        chrono::Utc,
    );
    let excluded_timestamp = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
        day.and_hms_milli_opt(17, 0, 0, 0).unwrap(),
        chrono::Utc,
    );

    sqlx::query(
        "INSERT INTO audit_logs (user_id, action, entity_type, entity_id, changes, created_at)
         VALUES
           ($1, 'LOGIN_SUCCESS', 'users', $1, $2, $3),
           ($1, 'LOGIN_SUCCESS', 'users', $1, $4, $5),
           ($1, 'LOGIN_SUCCESS', 'users', $1, $6, $7)",
    )
    .bind(admin_id)
    .bind(json!({"reason": "jakarta_start_boundary"}))
    .bind(start_timestamp)
    .bind(json!({"reason": "jakarta_fractional_end_boundary"}))
    .bind(end_timestamp)
    .bind(json!({"reason": "next_jakarta_day"}))
    .bind(excluded_timestamp)
    .execute(&pool)
    .await
    .unwrap();

    let date = day.format("%Y-%m-%d").to_string();
    let (status, body) = reports_request(
        &app,
        &token,
        "access_logs",
        &date,
        &date,
        "&action_type=LOGIN_SUCCESS",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["rows"]["data"].as_array().expect("access log rows");
    assert!(
        rows.iter()
            .any(|row| row["reason"] == Value::String("jakarta_start_boundary".to_string())),
        "event at 00:00:00.000 Asia/Jakarta must be included in same-day range"
    );
    assert!(
        rows.iter().any(
            |row| row["reason"] == Value::String("jakarta_fractional_end_boundary".to_string())
        ),
        "event at 23:59:59.500 Asia/Jakarta must be included in same-day range"
    );
    assert!(
        rows.iter()
            .all(|row| row["reason"] != Value::String("next_jakarta_day".to_string())),
        "event at 00:00:00.000 on the next Asia/Jakarta day must be excluded"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn ctc_decryption_failure_surfaces_redacted_report_row(pool: PgPool) {
    let app = build_app(&pool).await;
    let dept = create_department(&pool, "Audit Security").await;
    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr", Some(dept)).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let hr_token = get_token(&app, &hr_email).await;
    let admin_token = get_token(&app, &admin_email).await;

    let resource_id = create_resource(&pool, "Corrupt Cipher", dept).await;
    create_ctc_via_api(&app, &hr_token, resource_id).await;

    sqlx::query(
        "UPDATE ctc_revisions
         SET encrypted_components = 'not-valid-ciphertext'
         WHERE resource_id = $1",
    )
    .bind(resource_id)
    .execute(&pool)
    .await
    .unwrap();

    let (start, end) = current_year_dates();
    let (status, body) =
        reports_request(&app, &admin_token, "ctc_change_log", &start, &end, "").await;
    assert_eq!(status, StatusCode::OK);

    let rows = body["rows"]["data"].as_array().expect("ctc rows");
    let redacted = rows
        .iter()
        .find(|row| row["field"] == "ctc_components")
        .expect("redacted CTC row should be visible");
    assert_eq!(redacted["new_value"]["redacted"], Value::Bool(true));
    assert_eq!(redacted["new_value"]["reason"], "decryption_failed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn ctc_daily_rate_decryption_failure_surfaces_redacted_report_row(pool: PgPool) {
    let app = build_app(&pool).await;
    let dept = create_department(&pool, "Audit Security Daily").await;
    let hr_email = test_email();
    let _ = create_user(&pool, &hr_email, "hr", Some(dept)).await;
    let admin_email = test_email();
    let _ = create_user(&pool, &admin_email, "admin", None).await;
    let hr_token = get_token(&app, &hr_email).await;
    let admin_token = get_token(&app, &admin_email).await;

    let resource_id = create_resource(&pool, "Corrupt Daily Cipher", dept).await;
    create_ctc_via_api(&app, &hr_token, resource_id).await;
    let components = build_components(&app, &hr_token, resource_id).await;
    update_ctc_via_api(
        &app,
        &hr_token,
        resource_id,
        &components,
        "Daily adjustment",
    )
    .await;

    sqlx::query(
        "UPDATE ctc_revisions
         SET encrypted_daily_rate = 'not-valid-daily-rate-ciphertext'
         WHERE resource_id = $1",
    )
    .bind(resource_id)
    .execute(&pool)
    .await
    .unwrap();

    let (start, end) = current_year_dates();
    let (status, body) =
        reports_request(&app, &admin_token, "ctc_change_log", &start, &end, "").await;
    assert_eq!(status, StatusCode::OK);

    let rows = body["rows"]["data"].as_array().expect("ctc rows");
    let redacted = rows
        .iter()
        .find(|row| row["field"] == "ctc_components")
        .expect("redacted CTC row should be visible");
    assert_eq!(redacted["new_value"]["redacted"], Value::Bool(true));
    assert_eq!(redacted["new_value"]["reason"], "decryption_failed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn assignment_history_uses_event_payload_resource_and_project_ids(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let dept = create_department(&pool, "History").await;
    let live_resource_id = create_resource(&pool, "Current Resource", dept).await;
    let event_resource_id = create_resource(&pool, "Event Resource", dept).await;
    let live_project_id = create_project(&pool, "Current Project", admin_id).await;
    let event_project_id = create_project(&pool, "Event Project", admin_id).await;

    let allocation_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO allocations (resource_id, project_id, allocation_percentage, start_date, end_date)
         VALUES ($1, $2, 50, CURRENT_DATE, CURRENT_DATE + INTERVAL '30 days')
         RETURNING id",
    )
    .bind(live_resource_id)
    .bind(live_project_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    xynergy_backend::services::log_audit(
        &pool,
        Some(admin_id),
        "update",
        "allocation",
        allocation_id,
        json!({
            "before": {
                "resource_id": live_resource_id,
                "project_id": live_project_id,
            },
            "after": {
                "resource_id": event_resource_id,
                "project_id": event_project_id,
            }
        }),
    )
    .await
    .unwrap();

    let (start, end) = current_year_dates();
    let (status, body) =
        reports_request(&app, &token, "assignment_history", &start, &end, "").await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["rows"]["data"].as_array().expect("assignment rows");
    let row = rows
        .iter()
        .find(|row| row["audit_id"].is_string())
        .expect("assignment row should exist");
    assert_eq!(
        row["resource_id"],
        Value::String(event_resource_id.to_string())
    );
    assert_eq!(row["resource_name"], "Event Resource");
    assert_eq!(
        row["project_id"],
        Value::String(event_project_id.to_string())
    );
    assert_eq!(row["project_name"], "Event Project");
}

#[sqlx::test(migrations = "../../migrations")]
async fn assignment_history_uses_top_level_event_payload_ids(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    let dept = create_department(&pool, "Top Level History").await;
    let resource_id = create_resource(&pool, "Top Level Resource", dept).await;
    let project_id = create_project(&pool, "Top Level Project", admin_id).await;
    let allocation_id = Uuid::new_v4();

    xynergy_backend::services::log_audit(
        &pool,
        Some(admin_id),
        "overallocation_confirmed",
        "allocation",
        allocation_id,
        json!({
            "resource_id": resource_id,
            "project_id": project_id,
            "reason_code": "manual_overallocation_confirmation"
        }),
    )
    .await
    .unwrap();

    let (start, end) = current_year_dates();
    let (status, body) =
        reports_request(&app, &token, "assignment_history", &start, &end, "").await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["rows"]["data"].as_array().expect("assignment rows");
    let row = rows
        .iter()
        .find(|row| row["audit_id"].is_string())
        .expect("assignment row should exist");
    assert_eq!(row["resource_id"], Value::String(resource_id.to_string()));
    assert_eq!(row["project_id"], Value::String(project_id.to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn assignment_history_excludes_access_denied_rows(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;
    let allocation_id = Uuid::new_v4();

    xynergy_backend::services::log_audit(
        &pool,
        Some(admin_id),
        "ACCESS_DENIED",
        "allocation",
        allocation_id,
        json!({
            "reason": "insufficient_role",
            "resource_id": Uuid::new_v4(),
            "project_id": Uuid::new_v4(),
        }),
    )
    .await
    .unwrap();

    let (start, end) = current_year_dates();
    let (status, body) =
        reports_request(&app, &token, "assignment_history", &start, &end, "").await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["rows"]["data"].as_array().expect("assignment rows");
    assert!(
        rows.iter().all(|row| row["action"] != "ACCESS_DENIED"),
        "assignment history must not present denied attempts as assignment changes"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn assignment_history_excludes_budget_preview_rows(pool: PgPool) {
    let app = build_app(&pool).await;
    let admin_email = test_email();
    let admin_id = create_user(&pool, &admin_email, "admin", None).await;
    let token = get_token(&app, &admin_email).await;

    xynergy_backend::services::log_audit(
        &pool,
        Some(admin_id),
        "budget_preview_critical",
        "allocation",
        Uuid::new_v4(),
        json!({
            "resource_id": Uuid::new_v4(),
            "projected_committed_idr": 200_000_000,
            "department_budget_total_idr": 150_000_000,
        }),
    )
    .await
    .unwrap();

    let (start, end) = current_year_dates();
    let (status, body) =
        reports_request(&app, &token, "assignment_history", &start, &end, "").await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["rows"]["data"].as_array().expect("assignment rows");
    assert!(
        rows.iter()
            .all(|row| row["action"] != "budget_preview_critical"),
        "assignment history must not present budget preview warnings as assignment changes"
    );
}
