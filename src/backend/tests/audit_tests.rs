//! Integration tests for the Audit Logging System (Story 1.4)
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use sqlx::Row;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("audit-{}@example.com", Uuid::new_v4())
}

async fn create_test_department(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("INSERT INTO departments (name) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn create_test_user_with_role_and_dept(
    pool: &PgPool,
    email: &str,
    role: &str,
    dept_id: Option<Uuid>,
) -> Uuid {
    let password_hash = xynergy_backend::routes::auth::hash_password("Password123!").unwrap();

    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, first_name, last_name, role, department_id)
         VALUES ($1, $2, 'Test', 'User', $3, $4)
         RETURNING id",
    )
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .bind(dept_id)
    .fetch_one(pool)
    .await
    .unwrap()
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
    .unwrap()
}

async fn get_auth_token(app: &axum::Router, email: &str) -> String {
    let login_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": "Password123!"
            })
            .to_string(),
        ))
        .unwrap();

    let login_response = app.clone().oneshot(login_request).await.unwrap();
    let bytes = to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_json: Value = serde_json::from_slice(&bytes).unwrap();

    login_json["token"].as_str().unwrap().to_string()
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_valid_deterministic_hash_chain(pool: PgPool) {
    // Generate valid manual log inserts via log_audit helper
    let email = test_email();
    let user_id = create_test_user_with_role_and_dept(&pool, &email, "admin", None).await;
    let entity_id = Uuid::new_v4();

    // Insert first log
    xynergy_backend::services::log_audit(
        &pool,
        Some(user_id),
        "TEST_ACTION_1",
        "test_entity",
        entity_id,
        json!({"field": "val1"}),
    )
    .await
    .expect("Log 1 should succeed");

    // Insert second log
    xynergy_backend::services::log_audit(
        &pool,
        Some(user_id),
        "TEST_ACTION_2",
        "test_entity",
        entity_id,
        json!({"field": "val2"}),
    )
    .await
    .expect("Log 2 should succeed");

    let logs =
        sqlx::query("SELECT previous_hash, entry_hash FROM audit_logs ORDER BY created_at ASC")
            .fetch_all(&pool)
            .await
            .unwrap();

    assert!(logs.len() >= 2);

    // Verify chain linkage
    let log1 = &logs[logs.len() - 2];
    let log2 = &logs[logs.len() - 1];

    let log1_entry_hash: Option<String> = log1.try_get("entry_hash").unwrap();
    let log2_previous_hash: Option<String> = log2.try_get("previous_hash").unwrap();

    assert_eq!(
        log2_previous_hash.as_deref().unwrap_or(""),
        log1_entry_hash.as_deref().unwrap_or("")
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_ctc_view_and_mutation_audit(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var("CTC_ENCRYPTION_KEY_V1", "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=");
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let dept_id = create_test_department(&pool, "Finance").await;
    let _hr_id =
        create_test_user_with_role_and_dept(&pool, &hr_email, "hr", Some(dept_id))
            .await;
    let hr_token = get_auth_token(&app, &hr_email).await;
    let resource_id = create_test_resource_in_dept(&pool, "Fin Resource", dept_id).await;

    // View resource
    let req_view = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/{}/components", resource_id))
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();

    let res_view = app.clone().oneshot(req_view).await.unwrap();
    assert_eq!(res_view.status(), StatusCode::OK);

    // Update resource
    let req_update = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/ctc/{}/components", resource_id))
        .header("Authorization", format!("Bearer {}", hr_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "components": { "salary": 5000 },
                "reason": "Annual Adjustment"
            })
            .to_string(),
        ))
        .unwrap();

    let res_update = app.clone().oneshot(req_update).await.unwrap();
    assert_eq!(res_update.status(), StatusCode::OK);

    // Verify 2 logs generated
    let logs = sqlx::query!(
        "SELECT action, changes FROM audit_logs WHERE entity_id = $1 ORDER BY created_at ASC",
        resource_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].action, "VIEW");
    assert_eq!(logs[1].action, "UPDATE");
    let update_changes = logs[1].changes.clone().unwrap_or_else(|| json!({}));
    assert_eq!(update_changes["action"], "update_ctc");
    assert_eq!(update_changes["reason"], "Annual Adjustment");
    assert_eq!(update_changes["status"], "encrypted");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_audit_report_access(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role_and_dept(&pool, &admin_email, "admin", None).await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-logs?limit=5")
        .header("Authorization", format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_export_request_persists_pending_approval(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    let finance_email = test_email();
    let finance_id =
        create_test_user_with_role_and_dept(&pool, &finance_email, "finance", None).await;
    let finance_token = get_auth_token(&app, &finance_email).await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/audit-logs/export")
        .header("Authorization", format!("Bearer {}", finance_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let export_id = Uuid::parse_str(json["export_id"].as_str().unwrap()).unwrap();

    let row = sqlx::query("SELECT requested_by, status FROM audit_export_requests WHERE id = $1")
        .bind(export_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    let requested_by: Uuid = row.try_get("requested_by").unwrap();
    let status: String = row.try_get("status").unwrap();

    assert_eq!(requested_by, finance_id);
    assert_eq!(status, "pending_approval");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_verify_endpoint_detects_tampering(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    let admin_email = test_email();
    let _admin_id = create_test_user_with_role_and_dept(&pool, &admin_email, "admin", None).await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    let actor_email = test_email();
    let actor = create_test_user_with_role_and_dept(&pool, &actor_email, "admin", None).await;
    let entity = Uuid::new_v4();
    xynergy_backend::services::log_audit(
        &pool,
        Some(actor),
        "A1",
        "ctc_components",
        entity,
        json!({"x":1}),
    )
    .await
    .unwrap();
    xynergy_backend::services::log_audit(
        &pool,
        Some(actor),
        "A2",
        "ctc_components",
        entity,
        json!({"x":2}),
    )
    .await
    .unwrap();

    sqlx::query("ALTER TABLE audit_logs DISABLE TRIGGER audit_logs_append_only")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE audit_logs SET changes = $1 WHERE action = 'A2'")
        .bind(json!({"x":999}))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE audit_logs ENABLE TRIGGER audit_logs_append_only")
        .execute(&pool)
        .await
        .unwrap();

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-logs/verify")
        .header("Authorization", format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["is_valid"], Value::Bool(false));
}
