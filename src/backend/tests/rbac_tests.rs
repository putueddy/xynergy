//! Integration tests for role-based access control.

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("rbac-{}@example.com", Uuid::new_v4())
}

async fn create_test_user_with_role(pool: &PgPool, email: &str, role: &str) -> Uuid {
    let password_hash = xynergy_backend::routes::auth::hash_password("Password123!")
        .expect("password hashing should succeed in tests");

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
    .expect("test user should be created")
}

async fn create_test_resource(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO resources (name, resource_type, capacity)
         VALUES ($1, 'human', 1.0)
         RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("test resource should be created")
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
        .expect("request should be built");

    let login_response = app
        .clone()
        .oneshot(login_request)
        .await
        .expect("login should return response");

    let bytes = to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_json: Value = serde_json::from_slice(&bytes).unwrap();

    login_json["token"].as_str().unwrap().to_string()
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_update_roles(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    // Create admin user
    let admin_email = test_email();
    let _admin_id = create_test_user_with_role(&pool, &admin_email, "admin").await;
    let admin_token = get_auth_token(&app, &admin_email).await;

    // Create target user
    let target_email = test_email();
    let target_id = create_test_user_with_role(&pool, &target_email, "hr").await;

    // Update target user's role to department_head
    let update_req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/users/{}", target_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", admin_token))
        .body(Body::from(
            json!({
                "role": "department_head"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app.clone().oneshot(update_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify role in database
    let updated_role = sqlx::query_scalar::<_, String>("SELECT role FROM users WHERE id = $1")
        .bind(target_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(updated_role, "department_head");

    // Verify audit log
    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM audit_logs WHERE entity_type = 'user' AND entity_id = $1 AND action = 'update'"
    )
        .bind(target_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audit_count, 1, "audit log should be created");
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_admin_cannot_update_roles(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    // Create HR user
    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    // Create another target user
    let target_email = test_email();
    let target_id = create_test_user_with_role(&pool, &target_email, "finance").await;

    // Try to update target user's role using HR token
    let update_req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/users/{}", target_id))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "role": "admin"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app.clone().oneshot(update_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    let msg = json["error"]["message"].as_str().unwrap();
    assert_eq!(msg, "Insufficient permissions");
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_admin_cannot_create_or_delete_user(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    // Create non-admin (project_manager)
    let pm_email = test_email();
    let _pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let pm_token = get_auth_token(&app, &pm_email).await;

    // Try to create user
    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/users")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", pm_token))
        .body(Body::from(
            json!({
                "email": "should-fail@example.com",
                "password": "Password123!",
                "first_name": "Should",
                "last_name": "Fail",
                "role": "finance"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let create_res = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_res.status(), StatusCode::FORBIDDEN);

    // Try to delete a user
    let delete_id = create_test_user_with_role(&pool, "todelete@ex.com", "hr").await;
    let delete_req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/users/{}", delete_id))
        .header("Authorization", format!("Bearer {}", pm_token))
        .body(Body::empty())
        .expect("request should be built");

    let delete_res = app.clone().oneshot(delete_req).await.unwrap();
    assert_eq!(delete_res.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_admin_cannot_view_users(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/users")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .expect("request should be built");

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn department_head_denied_ctc_components_and_audited(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    let dept_a = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO departments (name) VALUES ('Dept A') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let dept_b = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO departments (name) VALUES ('Dept B') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let dh_email = test_email();
    let password_hash = xynergy_backend::routes::auth::hash_password("Password123!")
        .expect("password hashing should succeed in tests");
    let dh_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, first_name, last_name, role, department_id)
         VALUES ($1, $2, 'Test', 'User', 'department_head', $3)
         RETURNING id",
    )
    .bind(&dh_email)
    .bind(password_hash)
    .bind(dept_a)
    .fetch_one(&pool)
    .await
    .unwrap();
    let dh_token = get_auth_token(&app, &dh_email).await;
    let resource_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO resources (name, resource_type, capacity, department_id)
         VALUES ('RBAC Test Resource', 'human', 1.0, $1)
         RETURNING id",
    )
    .bind(dept_b)
    .fetch_one(&pool)
    .await
    .unwrap();

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/{}/components", resource_id))
        .header("Authorization", format!("Bearer {}", dh_token))
        .body(Body::empty())
        .expect("request should be built");

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let denied_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*)
         FROM audit_logs
         WHERE user_id = $1
           AND action = 'ACCESS_DENIED'
           AND entity_type = 'ctc_components'
           AND entity_id = $2",
    )
    .bind(dh_id)
    .bind(resource_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(denied_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn project_manager_can_only_view_assigned_projects(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    // Create 2 Project Managers
    let pm1_email = test_email();
    let pm1_id = create_test_user_with_role(&pool, &pm1_email, "project_manager").await;
    let pm1_token = get_auth_token(&app, &pm1_email).await;

    let pm2_email = test_email();
    let pm2_id = create_test_user_with_role(&pool, &pm2_email, "project_manager").await;

    // Create Project 1 assigned to PM 1
    let proj1_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, start_date, end_date, project_manager_id)
         VALUES ('Project 1', '2026-01-01', '2026-12-31', $1)
         RETURNING id",
    )
    .bind(pm1_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Create Project 2 assigned to PM 2
    let proj2_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, start_date, end_date, project_manager_id)
         VALUES ('Project 2', '2026-01-01', '2026-12-31', $1)
         RETURNING id",
    )
    .bind(pm2_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Query projects as PM 1
    let get_req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects")
        .header("Authorization", format!("Bearer {}", pm1_token))
        .body(Body::empty())
        .expect("request should be built");

    let response = app.clone().oneshot(get_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    let projects = json.as_array().unwrap();

    // PM 1 should only see Project 1
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["id"].as_str().unwrap(), proj1_id.to_string());

    // Query specific Project 2 as PM 1 should return Forbidden
    let get_proj2_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}", proj2_id))
        .header("Authorization", format!("Bearer {}", pm1_token))
        .body(Body::empty())
        .expect("request should be built");

    let response2 = app.clone().oneshot(get_proj2_req).await.unwrap();
    assert_eq!(response2.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn project_manager_only_sees_allocations_for_assigned_projects(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    let pm1_email = test_email();
    let pm1_id = create_test_user_with_role(&pool, &pm1_email, "project_manager").await;
    let pm1_token = get_auth_token(&app, &pm1_email).await;

    let pm2_email = test_email();
    let pm2_id = create_test_user_with_role(&pool, &pm2_email, "project_manager").await;

    let project_1 = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, start_date, end_date, project_manager_id)
         VALUES ('PM1 Project', '2026-01-01', '2026-12-31', $1)
         RETURNING id",
    )
    .bind(pm1_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let project_2 = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, start_date, end_date, project_manager_id)
         VALUES ('PM2 Project', '2026-01-01', '2026-12-31', $1)
         RETURNING id",
    )
    .bind(pm2_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let resource_id = create_test_resource(&pool, "Shared Resource").await;

    sqlx::query(
        "INSERT INTO allocations (project_id, resource_id, start_date, end_date, allocation_percentage)
         VALUES ($1, $2, '2026-02-01', '2026-02-15', 50.0)",
    )
    .bind(project_1)
    .bind(resource_id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO allocations (project_id, resource_id, start_date, end_date, allocation_percentage)
         VALUES ($1, $2, '2026-02-16', '2026-02-28', 50.0)",
    )
    .bind(project_2)
    .bind(resource_id)
    .execute(&pool)
    .await
    .unwrap();

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/allocations/resource/{}", resource_id))
        .header("Authorization", format!("Bearer {}", pm1_token))
        .body(Body::empty())
        .expect("request should be built");

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    let allocations = json.as_array().unwrap();

    assert_eq!(allocations.len(), 1);
    assert_eq!(
        allocations[0]["project_id"].as_str().unwrap(),
        project_1.to_string()
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn hr_cannot_create_allocation(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let _hr_id = create_test_user_with_role(&pool, &hr_email, "hr").await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let pm_email = test_email();
    let pm_id = create_test_user_with_role(&pool, &pm_email, "project_manager").await;
    let project_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (name, start_date, end_date, project_manager_id)
         VALUES ('RBAC Allocation Project', '2026-01-01', '2026-12-31', $1)
         RETURNING id",
    )
    .bind(pm_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let resource_id = create_test_resource(&pool, "Allocation RBAC Resource").await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/allocations")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::from(
            json!({
                "project_id": project_id,
                "resource_id": resource_id,
                "start_date": "2026-03-01",
                "end_date": "2026-03-31",
                "allocation_percentage": 50.0,
                "include_weekend": false
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
