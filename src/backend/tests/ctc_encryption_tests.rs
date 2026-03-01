use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use tower::ServiceExt;
use uuid::Uuid;

use xynergy_backend::services::ctc_crypto::{
    backfill_plaintext_ctc_records, CtcCryptoService, DefaultCtcCryptoService, EncryptedPayload,
};
use xynergy_backend::services::key_provider::EnvKeyProvider;

fn test_email() -> String {
    format!("crypto-{}@example.com", Uuid::new_v4())
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
         VALUES ($1, $2, 'Crypto', 'Test', $3, $4)
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
async fn test_create_and_read_ctc_encrypts_data(pool: PgPool) {
    std::env::set_var("JWT_SECRET", "test-secret");
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );
    let app = xynergy_backend::create_app(pool.clone());

    let hr_email = test_email();
    let dept_id = create_test_department(&pool, "HR Dept").await;
    let _hr_id = create_test_user_with_role_and_dept(&pool, &hr_email, "hr", Some(dept_id)).await;
    let hr_token = get_auth_token(&app, &hr_email).await;

    let resource_id = create_test_resource_in_dept(&pool, "Encrypted Engineer", dept_id).await;

    // Create CTC (should be encrypted in DB)
    let payload = json!({
        "resource_id": resource_id.to_string(),
        "base_salary": 15000000,
        "hra_allowance": 2000000,
        "medical_allowance": 1000000,
        "transport_allowance": 500000,
        "meal_allowance": 500000,
        "working_days_per_month": 22,
        "risk_tier": 1
    });

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/ctc")
        .header("Authorization", format!("Bearer {}", hr_token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let create_res = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_res.status(), StatusCode::OK);

    // Assert raw DB state contains NO plaintext
    let row = sqlx::query(
        "SELECT components, encrypted_components, encrypted_daily_rate, daily_rate, key_version FROM ctc_records WHERE resource_id = $1"
    )
    .bind(resource_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let components: serde_json::Value = row.try_get("components").unwrap();
    assert_eq!(components, json!({})); // Empty plaintext JSON body

    let encrypted_components: String = row.try_get("encrypted_components").unwrap();
    assert!(!encrypted_components.is_empty());
    assert!(!encrypted_components.contains("15000000")); // Shouldn't be raw string either

    let encrypted_daily_rate: String = row.try_get("encrypted_daily_rate").unwrap();
    assert!(!encrypted_daily_rate.is_empty());

    let daily_rate: sqlx::types::BigDecimal = row.try_get("daily_rate").unwrap();
    assert_eq!(daily_rate.to_string(), "0");

    let key_version: String = row.try_get("key_version").unwrap();
    assert_eq!(key_version, "v1");

    // Retrieve via API (should decrypt successfully for HR)
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ctc/{}/components", resource_id))
        .header("Authorization", format!("Bearer {}", hr_token))
        .body(Body::empty())
        .unwrap();

    let get_res = app.clone().oneshot(get_req).await.unwrap();
    assert_eq!(get_res.status(), StatusCode::OK);

    let get_body = to_bytes(get_res.into_body(), usize::MAX).await.unwrap();
    let get_json: Value = serde_json::from_slice(&get_body).unwrap();

    assert_eq!(get_json["components"]["base_salary"], 15000000);
}

#[tokio::test]
async fn test_key_version_metadata_and_rotation_compatibility() {
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V2",
        "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=",
    );

    let v1_service = DefaultCtcCryptoService::new(EnvKeyProvider {
        active_version: "v1".to_string(),
    });
    let v2_service = DefaultCtcCryptoService::new(EnvKeyProvider {
        active_version: "v2".to_string(),
    });

    let original = json!({"base_salary": 12345678, "meal_allowance": 250000});

    let payload_v1 = v1_service.encrypt_components(&original).await.unwrap();
    assert_eq!(payload_v1.key_version, "v1");

    let payload_v2 = v2_service.encrypt_components(&original).await.unwrap();
    assert_eq!(payload_v2.key_version, "v2");

    // Simulate post-rotation runtime: decrypt old payload using the newer provider.
    let decrypted_old = v2_service.decrypt_components(&payload_v1).await.unwrap();
    assert_eq!(decrypted_old, original);

    let decrypted_new = v2_service.decrypt_components(&payload_v2).await.unwrap();
    assert_eq!(decrypted_new, original);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_plaintext_backfill(pool: PgPool) {
    std::env::set_var("CTC_ACTIVE_KEY_VERSION", "v1");
    std::env::set_var(
        "CTC_ENCRYPTION_KEY_V1",
        "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=",
    );
    let dept_id = create_test_department(&pool, "Legacy Dept").await;
    let resource_id = create_test_resource_in_dept(&pool, "Legacy Engineer", dept_id).await;
    let hr_email = test_email();
    let hr_id = create_test_user_with_role_and_dept(&pool, &hr_email, "hr", Some(dept_id)).await;

    // 1. Manually insert a "legacy" plaintext record
    sqlx::query(
        "INSERT INTO ctc_records (
            resource_id, components, created_by, updated_by, reason, 
             daily_rate, working_days_per_month, effective_date, status, created_at
         ) VALUES (
             $1, $2, $3, $3, 'Legacy insert', 
             654321.12, 22, CURRENT_DATE, 'Active', CURRENT_TIMESTAMP
         )",
    )
    .bind(resource_id)
    .bind(json!({
        "base_salary": 9999999
    }))
    .bind(hr_id)
    .execute(&pool)
    .await
    .unwrap();

    // Verify it's unencrypted
    let pre = sqlx::query(
        "SELECT encrypted_components, components FROM ctc_records WHERE resource_id = $1",
    )
    .bind(resource_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let pre_enc: Option<String> = pre.try_get("encrypted_components").unwrap();
    assert!(pre_enc.is_none());

    let pre_comp: serde_json::Value = pre.try_get("components").unwrap();
    assert_eq!(pre_comp["base_salary"], 9999999);

    // 2. Run Backfill
    let crypto_svc = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let backfilled_count = backfill_plaintext_ctc_records(&pool, &crypto_svc)
        .await
        .unwrap();
    assert_eq!(backfilled_count, 1);

    // 3. Verify it is now encrypted
    let post = sqlx::query("SELECT encrypted_components, encrypted_daily_rate, components, daily_rate FROM ctc_records WHERE resource_id = $1")
        .bind(resource_id)
        .fetch_one(&pool).await.unwrap();

    let post_enc: String = post.try_get("encrypted_components").unwrap();
    assert!(!post_enc.is_empty());

    let post_comp: serde_json::Value = post.try_get("components").unwrap();
    assert_eq!(post_comp, json!({}));

    let post_daily_rate: sqlx::types::BigDecimal = post.try_get("daily_rate").unwrap();
    assert_eq!(post_daily_rate.to_string(), "0");

    let post_enc_daily_rate: String = post.try_get("encrypted_daily_rate").unwrap();
    assert!(!post_enc_daily_rate.is_empty());

    // 4. Decrypt via service to verify equivalence
    let key_version: String = post
        .try_get("key_version")
        .unwrap_or_else(|_| "v1".to_string());
    let decrypted = crypto_svc
        .decrypt_components(&EncryptedPayload {
            ciphertext: post_enc,
            key_version,
            encryption_version: "v1".to_string(),
            algorithm: "AES-256-GCM".to_string(),
            encrypted_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    assert_eq!(decrypted["base_salary"], 9999999);

    let decrypted_daily_rate = crypto_svc
        .decrypt_components(&EncryptedPayload {
            ciphertext: post_enc_daily_rate,
            key_version: "v1".to_string(),
            encryption_version: "v1".to_string(),
            algorithm: "AES-256-GCM".to_string(),
            encrypted_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    let decrypted_daily_rate_str = decrypted_daily_rate["daily_rate"]
        .as_str()
        .unwrap_or_default();
    assert!(decrypted_daily_rate_str.starts_with("654321.12"));
}
