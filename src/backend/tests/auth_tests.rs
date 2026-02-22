//! Integration tests for authentication flows.

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_email() -> String {
    format!("auth-{}@example.com", Uuid::new_v4())
}

async fn create_test_user(pool: &PgPool, email: &str, password: &str) -> Uuid {
    let password_hash = xynergy_backend::routes::auth::hash_password(password)
        .expect("password hashing should succeed in tests");

    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, first_name, last_name, role)
         VALUES ($1, $2, 'Test', 'User', 'admin')
         RETURNING id",
    )
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
    .expect("test user should be created")
}

async fn parse_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should be valid json")
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_refresh_and_me_flow(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");

    let email = test_email();
    let _user_id = create_test_user(&pool, &email, "CorrectHorseBatteryStaple!").await;

    let app = xynergy_backend::create_app(pool.clone());

    let login_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": "CorrectHorseBatteryStaple!"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let login_response = app
        .clone()
        .oneshot(login_request)
        .await
        .expect("login should return response");
    assert_eq!(login_response.status(), StatusCode::OK);
    let login_json = parse_json(login_response).await;

    let token = login_json["token"]
        .as_str()
        .expect("login token should be present")
        .to_string();
    let refresh_token = login_json["refresh_token"]
        .as_str()
        .expect("login refresh token should be present")
        .to_string();

    let me_request = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/me")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("request should be built");

    let me_response = app
        .clone()
        .oneshot(me_request)
        .await
        .expect("me should return response");
    assert_eq!(me_response.status(), StatusCode::OK);
    let me_json = parse_json(me_response).await;
    assert_eq!(
        me_json["email"].as_str(),
        Some(login_json["user"]["email"].as_str().unwrap_or(""))
    );

    let refresh_request = Request::builder()
        .method("PUT")
        .uri("/api/v1/auth/refresh")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "refresh_token": refresh_token }).to_string(),
        ))
        .expect("request should be built");

    let refresh_response = app
        .clone()
        .oneshot(refresh_request)
        .await
        .expect("refresh should return response");
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_json = parse_json(refresh_response).await;

    let rotated_refresh = refresh_json["refresh_token"]
        .as_str()
        .expect("rotated refresh token should be present")
        .to_string();
    assert_ne!(rotated_refresh, refresh_token);

    let replay_request = Request::builder()
        .method("PUT")
        .uri("/api/v1/auth/refresh")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "refresh_token": refresh_token }).to_string(),
        ))
        .expect("request should be built");

    let replay_response = app
        .clone()
        .oneshot(replay_request)
        .await
        .expect("replay should return response");
    assert_eq!(replay_response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn lockout_uses_generic_error_message(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");

    let email = test_email();
    let user_id = create_test_user(&pool, &email, "CorrectHorseBatteryStaple!").await;

    let app = xynergy_backend::create_app(pool.clone());

    for _ in 0..5 {
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "email": email,
                    "password": "wrong-password"
                })
                .to_string(),
            ))
            .expect("request should be built");

        let response = app
            .clone()
            .oneshot(req)
            .await
            .expect("response should be returned");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = parse_json(response).await;
        let message = body["error"]["message"].as_str().unwrap_or_default();
        assert!(message.contains("Invalid credentials"));
        assert!(!message.contains("locked"));
    }

    let locked_until = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT locked_until FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("locked_until should be queryable");

    assert!(locked_until.is_some());

    let blocked_login_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": "CorrectHorseBatteryStaple!"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let blocked_response = app
        .clone()
        .oneshot(blocked_login_request)
        .await
        .expect("response should be returned");
    assert_eq!(blocked_response.status(), StatusCode::UNAUTHORIZED);
    let blocked_body = parse_json(blocked_response).await;
    let blocked_message = blocked_body["error"]["message"]
        .as_str()
        .unwrap_or_default();
    assert!(blocked_message.contains("Invalid credentials"));
    assert!(!blocked_message.contains("locked"));
}
