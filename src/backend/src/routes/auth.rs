use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Json, State},
    http::HeaderMap,
    routing::{get, post, put},
    Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::User;
use crate::services::audit_log::{log_audit, user_id_from_headers};

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID
    pub email: String,
    pub role: String,
    pub department_id: Option<Uuid>, // User's department for efficient scoping
    pub exp: usize, // Expiration time
    pub iat: usize, // Issued at
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserInfo,
}

/// User info for login response
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Refresh token response
#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub token: String,
    pub refresh_token: String,
}

/// Hash password using Argon2
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut thread_rng());
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Password hashing failed: {}", e)))?
        .to_string();

    Ok(password_hash)
}

/// Hash refresh token using Argon2
fn hash_refresh_token(token: &str) -> Result<String> {
    let pepper = std::env::var("REFRESH_TOKEN_PEPPER")
        .or_else(|_| std::env::var("JWT_SECRET"))
        .map_err(|_| {
            AppError::Internal("REFRESH_TOKEN_PEPPER or JWT_SECRET not set".to_string())
        })?;

    let mut hasher = Sha256::new();
    hasher.update(pepper.as_bytes());
    hasher.update(b":");
    hasher.update(token.as_bytes());

    Ok(format!("{:x}", hasher.finalize()))
}

/// Verify password using Argon2
fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash: {}", e)))?;

    let argon2 = Argon2::default();

    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Generate JWT access token (15-minute expiry)
fn generate_access_token(user: &User) -> Result<String> {
    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| AppError::Internal("JWT_SECRET not set".to_string()))?;

    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(15))
        .ok_or_else(|| {
            AppError::Internal("Failed to build token expiration timestamp".to_string())
        })?
        .timestamp() as usize;

    let claims = Claims {
        sub: user.id.to_string(),
        email: user.email.clone(),
        role: user.role.clone(),
        department_id: user.department_id,
        exp: expiration,
        iat: Utc::now().timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Authentication(format!("Token generation failed: {}", e)))
}

/// Generate random refresh token
fn generate_refresh_token() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

/// Check if account is locked
fn is_account_locked(locked_until: Option<chrono::DateTime<Utc>>) -> bool {
    match locked_until {
        Some(lock_time) if lock_time > Utc::now() => true,
        _ => false,
    }
}

/// Login handler
async fn login(
    State(pool): State<PgPool>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    // Find user by email
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&req.email)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // If user not found, return generic error (prevent user enumeration)
    let user = match user {
        Some(u) => u,
        None => {
            // Log failed login attempt (user not found)
            log_audit(
                &pool,
                None,
                "LOGIN_FAILED",
                "user",
                Uuid::nil(),
                json!({
                    "reason": "user_not_found",
                    "email_attempted": req.email,
                    "ip_address": "unknown" // Would extract from request in production
                }),
            )
            .await
            .ok(); // Don't fail login if audit logging fails

            return Err(AppError::Authentication("Invalid credentials".to_string()));
        }
    };

    // Check if account is locked
    if is_account_locked(user.locked_until) {
        let locked_until = user.locked_until.unwrap();
        let remaining_minutes = (locked_until - Utc::now()).num_minutes();

        log_audit(
            &pool,
            Some(user.id),
            "LOGIN_BLOCKED",
            "user",
            user.id,
            json!({
                "reason": "account_locked",
                "locked_until": locked_until,
                "remaining_minutes": remaining_minutes
            }),
        )
        .await
        .ok();

        return Err(AppError::Authentication("Invalid credentials".to_string()));
    }

    // Verify password
    let is_valid = verify_password(&req.password, &user.password_hash)?;

    if !is_valid {
        // Increment login attempts
        let new_attempts = user.login_attempts.unwrap_or(0) + 1;
        let max_attempts = 5;

        if new_attempts >= max_attempts {
            // Lock account for 30 minutes
            let locked_until = Utc::now() + Duration::minutes(30);

            sqlx::query("UPDATE users SET login_attempts = $1, locked_until = $2 WHERE id = $3")
                .bind(new_attempts)
                .bind(Some(locked_until))
                .bind(user.id)
                .execute(&pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

            // Log account lockout
            log_audit(
                &pool,
                Some(user.id),
                "ACCOUNT_LOCKED",
                "user",
                user.id,
                json!({
                    "reason": "too_many_failed_attempts",
                    "failed_attempts": new_attempts,
                    "locked_until": locked_until,
                    "lock_duration_minutes": 30
                }),
            )
            .await
            .ok();

            return Err(AppError::Authentication("Invalid credentials".to_string()));
        } else {
            // Just increment the counter
            sqlx::query("UPDATE users SET login_attempts = $1 WHERE id = $2")
                .bind(new_attempts)
                .bind(user.id)
                .execute(&pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

            // Log failed login attempt
            log_audit(
                &pool,
                Some(user.id),
                "LOGIN_FAILED",
                "user",
                user.id,
                json!({
                    "reason": "invalid_password",
                    "attempt_number": new_attempts,
                    "max_attempts": max_attempts
                }),
            )
            .await
            .ok();
        }

        return Err(AppError::Authentication("Invalid credentials".to_string()));
    }

    // Password is valid - reset login attempts and generate tokens
    let refresh_token = generate_refresh_token();
    let refresh_token_hash = hash_refresh_token(&refresh_token)?;

    sqlx::query(
        "UPDATE users SET login_attempts = 0, locked_until = NULL, refresh_token_hash = $1, last_login_at = $2 WHERE id = $3",
    )
    .bind(Some(refresh_token_hash))
    .bind(Some(Utc::now()))
    .bind(user.id)
    .execute(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Generate access token
    let token = generate_access_token(&user)?;

    // Log successful login
    log_audit(
        &pool,
        Some(user.id),
        "LOGIN_SUCCESS",
        "user",
        user.id,
        json!({
            "timestamp": Utc::now(),
            "login_method": "password"
        }),
    )
    .await
    .ok();

    Ok(Json(LoginResponse {
        token,
        refresh_token,
        user: UserInfo {
            id: user.id,
            email: user.email,
            first_name: user.first_name,
            last_name: user.last_name,
            role: user.role,
        },
    }))
}

/// Refresh token handler
async fn refresh_token(
    State(pool): State<PgPool>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>> {
    let refresh_token_hash = hash_refresh_token(&req.refresh_token)?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE refresh_token_hash = $1")
        .bind(&refresh_token_hash)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::Authentication("Invalid refresh token".to_string()))?;

    // Generate new tokens
    let new_access_token = generate_access_token(&user)?;
    let new_refresh_token = generate_refresh_token();
    let new_refresh_token_hash = hash_refresh_token(&new_refresh_token)?;

    // Update user's refresh token (rotation)
    sqlx::query("UPDATE users SET refresh_token_hash = $1 WHERE id = $2")
        .bind(Some(new_refresh_token_hash))
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Log token refresh
    log_audit(
        &pool,
        Some(user.id),
        "TOKEN_REFRESH",
        "user",
        user.id,
        json!({
            "timestamp": Utc::now(),
            "reason": "access_token_expired"
        }),
    )
    .await
    .ok();

    Ok(Json(RefreshTokenResponse {
        token: new_access_token,
        refresh_token: new_refresh_token,
    }))
}

/// Get current authenticated user
async fn me(State(pool): State<PgPool>, headers: HeaderMap) -> Result<Json<UserInfo>> {
    let user_id = user_id_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Invalid credentials".to_string()))?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::Authentication("Invalid credentials".to_string()))?;

    Ok(Json(UserInfo {
        id: user.id,
        email: user.email,
        first_name: user.first_name,
        last_name: user.last_name,
        role: user.role,
    }))
}

/// Create auth routes
pub fn auth_routes() -> Router<PgPool> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/refresh", put(refresh_token))
        .route("/auth/me", get(me))
}
