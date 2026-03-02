use leptos::*;
use reqwest::{Method, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
}

/// Authentication context
#[derive(Debug, Clone, Copy)]
pub struct AuthContext {
    pub user: RwSignal<Option<User>>,
    pub token: RwSignal<Option<String>>,
    pub refresh_token: RwSignal<Option<String>>,
    pub is_authenticated: Memo<bool>,
}

/// Provide authentication context to the application
pub fn provide_auth_context() {
    let user = create_rw_signal(None);
    let token = create_rw_signal(None);
    let refresh_token = create_rw_signal(None);

    // Check localStorage on mount
    if let Ok(storage) = web_sys::window().unwrap().local_storage() {
        if let Some(storage) = storage {
            if let Ok(Some(saved_token)) = storage.get_item("auth_token") {
                token.set(Some(saved_token));
            }
            if let Ok(Some(saved_refresh)) = storage.get_item("auth_refresh_token") {
                refresh_token.set(Some(saved_refresh));
            }
        }
    }

    let is_authenticated = create_memo(move |_| {
        // Check both user and token - authenticated if either exists
        user.get().is_some() || token.get().is_some()
    });

    // Set up automatic token refresh
    setup_token_refresh(token, refresh_token);

    provide_context(AuthContext {
        user,
        token,
        refresh_token,
        is_authenticated,
    });
}

/// Set up automatic token refresh before expiry (every 14 minutes)
fn setup_token_refresh(
    token_signal: RwSignal<Option<String>>,
    refresh_token_signal: RwSignal<Option<String>>,
) {
    spawn_local(async move {
        // Refresh token every 14 minutes (token expires at 15 minutes)
        let refresh_interval = 14 * 60 * 1000; // 14 minutes in milliseconds

        loop {
            // Wait for the refresh interval
            gloo_timers::future::TimeoutFuture::new(refresh_interval).await;

            // Check if we have both tokens
            let current_refresh = refresh_token_signal.get();

            if let Some(refresh) = current_refresh {
                // Attempt to refresh the token
                match refresh_access_token(&refresh).await {
                    Ok((new_token, new_refresh)) => {
                        // Update signals
                        token_signal.set(Some(new_token.clone()));
                        refresh_token_signal.set(Some(new_refresh.clone()));

                        // Update localStorage
                        if let Ok(storage) = web_sys::window().unwrap().local_storage() {
                            if let Some(storage) = storage {
                                let _ = storage.set_item("auth_token", &new_token);
                                let _ = storage.set_item("auth_refresh_token", &new_refresh);
                            }
                        }
                    }
                    Err(e) => {
                        // Refresh failed - clear tokens (user needs to re-login)
                        web_sys::console::error_1(&format!("Token refresh failed: {}", e).into());
                        token_signal.set(None);
                        refresh_token_signal.set(None);

                        if let Ok(storage) = web_sys::window().unwrap().local_storage() {
                            if let Some(storage) = storage {
                                let _ = storage.remove_item("auth_token");
                                let _ = storage.remove_item("auth_refresh_token");
                            }
                        }
                    }
                }
            }
        }
    });
}

fn read_local_storage(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(key).ok().flatten())
}

fn write_local_storage(key: &str, value: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(key, value);
    }
}

pub fn clear_auth_storage() {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.remove_item("auth_token");
        let _ = storage.remove_item("auth_refresh_token");
    }
}

pub fn auth_token() -> Result<String, String> {
    read_local_storage("auth_token").ok_or_else(|| "SESSION_EXPIRED".to_string())
}

fn force_relogin() {
    clear_auth_storage();
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_href("/login");
    }
}

pub async fn refresh_access_token_from_storage() -> Result<String, String> {
    let refresh =
        read_local_storage("auth_refresh_token").ok_or_else(|| "SESSION_EXPIRED".to_string())?;

    match refresh_access_token(&refresh).await {
        Ok((new_token, new_refresh)) => {
            write_local_storage("auth_token", &new_token);
            write_local_storage("auth_refresh_token", &new_refresh);
            Ok(new_token)
        }
        Err(_) => {
            force_relogin();
            Err("SESSION_EXPIRED".to_string())
        }
    }
}

async fn authenticated_request<T: Serialize + ?Sized>(
    method: Method,
    url: &str,
    payload: Option<&T>,
) -> Result<Response, String> {
    let client = reqwest::Client::new();
    let token = auth_token()?;

    let mut builder = client
        .request(method.clone(), url)
        .header("Authorization", format!("Bearer {}", token));
    if let Some(body) = payload {
        builder = builder.json(body);
    }
    let mut response = builder
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        let refreshed = refresh_access_token_from_storage().await?;
        let mut retry_builder = client
            .request(method, url)
            .header("Authorization", format!("Bearer {}", refreshed));
        if let Some(body) = payload {
            retry_builder = retry_builder.json(body);
        }
        response = retry_builder
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            force_relogin();
            return Err("SESSION_EXPIRED".to_string());
        }
    }

    Ok(response)
}

pub async fn authenticated_get(url: &str) -> Result<Response, String> {
    authenticated_request::<serde_json::Value>(Method::GET, url, None).await
}

pub async fn authenticated_post_json<T: Serialize + ?Sized>(
    url: &str,
    payload: &T,
) -> Result<Response, String> {
    authenticated_request(Method::POST, url, Some(payload)).await
}

pub async fn authenticated_put_json<T: Serialize + ?Sized>(
    url: &str,
    payload: &T,
) -> Result<Response, String> {
    authenticated_request(Method::PUT, url, Some(payload)).await
}

pub async fn authenticated_delete(url: &str) -> Result<Response, String> {
    authenticated_request::<serde_json::Value>(Method::DELETE, url, None).await
}

/// Use the authentication context
pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>().expect("AuthContext not found")
}

/// Login response from API
#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: User,
}

/// Refresh token response from API
#[derive(Debug, Deserialize)]
pub struct RefreshTokenResponse {
    pub token: String,
    pub refresh_token: String,
}

/// Login request to API
#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Perform login
pub async fn login_user(email: String, password: String) -> Result<LoginResponse, String> {
    let request = LoginRequest { email, password };

    let response = reqwest::Client::new()
        .post("http://localhost:3000/api/v1/auth/login")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let login_response: LoginResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Save tokens to localStorage
        if let Ok(storage) = web_sys::window().unwrap().local_storage() {
            if let Some(storage) = storage {
                let _ = storage.set_item("auth_token", &login_response.token);
                let _ = storage.set_item("auth_refresh_token", &login_response.refresh_token);
            }
        }

        Ok(login_response)
    } else {
        // Return generic error message (no user enumeration)
        let error_text = response.text().await.unwrap_or_default();
        // Log actual error to console for debugging
        web_sys::console::error_1(&format!("Login error: {}", error_text).into());
        Err("Invalid credentials. Please check your email and password.".to_string())
    }
}

/// Refresh access token using refresh token
async fn refresh_access_token(refresh_token: &str) -> Result<(String, String), String> {
    #[derive(Debug, Serialize)]
    struct RefreshRequest {
        refresh_token: String,
    }

    let request = RefreshRequest {
        refresh_token: refresh_token.to_string(),
    };

    let response = reqwest::Client::new()
        .put("http://localhost:3000/api/v1/auth/refresh")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let refresh_response: RefreshTokenResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok((refresh_response.token, refresh_response.refresh_token))
    } else {
        Err("Token refresh failed".to_string())
    }
}

/// Logout user
pub fn logout_user(auth: &AuthContext) {
    auth.user.set(None);
    auth.token.set(None);
    auth.refresh_token.set(None);

    // Remove tokens from localStorage
    if let Ok(storage) = web_sys::window().unwrap().local_storage() {
        if let Some(storage) = storage {
            let _ = storage.remove_item("auth_token");
            let _ = storage.remove_item("auth_refresh_token");
        }
    }
}

/// Validate token and fetch user data
pub async fn validate_token(token: String) -> Result<User, String> {
    let response = reqwest::Client::new()
        .get("http://localhost:3000/api/v1/auth/me")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let user: User = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(user)
    } else {
        Err("Invalid token".to_string())
    }
}
