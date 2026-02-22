//! Integration tests for RLS (Row-Level Security)
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("rls-{}@example.com", Uuid::new_v4())
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
async fn dept_head_sees_only_own_department_resources(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    let dept1_id = create_test_department(&pool, "Engineering").await;
    let dept2_id = create_test_department(&pool, "Sales").await;

    // Create dept head in Engineering
    let head_email = test_email();
    let _head_id =
        create_test_user_with_role_and_dept(&pool, &head_email, "department_head", Some(dept1_id))
            .await;
    let head_token = get_auth_token(&app, &head_email).await;

    // Create resources
    let res1 = create_test_resource_in_dept(&pool, "Eng Resource 1", dept1_id).await;
    let res2 = create_test_resource_in_dept(&pool, "Sales Resource 1", dept2_id).await;

    // Fetch team list
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/resources")
        .header("Authorization", format!("Bearer {}", head_token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    let resources = json.as_array().unwrap();

    // Verify
    let found_res1 = resources
        .iter()
        .any(|r| r["id"].as_str().unwrap() == res1.to_string());
    let found_res2 = resources
        .iter()
        .any(|r| r["id"].as_str().unwrap() == res2.to_string());

    assert!(found_res1, "Should see own department resource");
    assert!(
        !found_res2,
        "Should NOT see other department resource via RLS"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn dept_head_denied_other_dept_ctc_and_audited(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    let dept1_id = create_test_department(&pool, "Engineering").await;
    let dept2_id = create_test_department(&pool, "Sales").await;

    let head_email = test_email();
    let head_id =
        create_test_user_with_role_and_dept(&pool, &head_email, "department_head", Some(dept1_id))
            .await;
    let head_token = get_auth_token(&app, &head_email).await;

    // Resource in Sales
    let res_sales = create_test_resource_in_dept(&pool, "Sales Resource", dept2_id).await;

    // Attempt direct URL CTC access
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/{}/components", res_sales))
        .header("Authorization", format!("Bearer {}", head_token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    // Because generic security error, getting 403
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Verify audit log
    let denied_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM audit_logs 
         WHERE user_id = $1 AND action = 'ACCESS_DENIED' AND entity_type = 'ctc_components' AND entity_id = $2"
    )
    .bind(head_id)
    .bind(res_sales)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        denied_count, 1,
        "unauthorized cross-dept CTC access should be audited"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_can_read_all_ctc_and_is_audited(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    let dept_eng_id = create_test_department(&pool, "Engineering").await;
    let dept_hr_id = create_test_department(&pool, "HR Dept").await;

    // Create HR user
    let hr_email = test_email();
    let hr_id = create_test_user_with_role_and_dept(&pool, &hr_email, "hr", Some(dept_hr_id)).await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    // Resource in Engineering
    let res_eng = create_test_resource_in_dept(&pool, "Eng Resource", dept_eng_id).await;

    // Fetch CTC for Eng Resource
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/{}/components", res_eng))
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify audit log for cross dept view
    let hr_audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM audit_logs 
         WHERE user_id = $1 AND action = 'CTC_VIEW_CROSS_DEPT' AND entity_id = $2",
    )
    .bind(hr_id)
    .bind(res_eng)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        hr_audit_count, 1,
        "HR cross-department CTC view should be audited"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn resources_table_has_forced_rls_and_required_policies(pool: PgPool) {
    let flags = sqlx::query!(
        "SELECT c.relrowsecurity, c.relforcerowsecurity
         FROM pg_class c
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = 'public' AND c.relname = 'resources'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(flags.relrowsecurity);
    assert!(flags.relforcerowsecurity);

    let policy_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*)
         FROM pg_policies
         WHERE schemaname = 'public'
           AND tablename = 'resources'
           AND policyname IN ('admin_all_policy', 'hr_read_policy', 'dept_head_policy', 'standard_roles_policy')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(policy_count, 4);
}
