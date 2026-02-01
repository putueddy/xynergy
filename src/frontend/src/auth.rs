use leptos::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub is_authenticated: Memo<bool>,
}

/// Provide authentication context to the application
pub fn provide_auth_context() {
    let user = create_rw_signal(None);
    let token = create_rw_signal(None);

    // Check localStorage on mount
    if let Ok(storage) = web_sys::window().unwrap().local_storage() {
        if let Some(storage) = storage {
            if let Ok(Some(saved_token)) = storage.get_item("auth_token") {
                token.set(Some(saved_token));
            }
        }
    }

    let is_authenticated = create_memo(move |_| {
        // Check both user and token - authenticated if either exists
        user.get().is_some() || token.get().is_some()
    });

    provide_context(AuthContext {
        user,
        token,
        is_authenticated,
    });
}

/// Use the authentication context
pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>().expect("AuthContext not found")
}

/// Login response from API
#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
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

        // Save token to localStorage
        if let Ok(storage) = web_sys::window().unwrap().local_storage() {
            if let Some(storage) = storage {
                let _ = storage.set_item("auth_token", &login_response.token);
            }
        }

        Ok(login_response)
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Login failed: {}", error_text))
    }
}

/// Logout user
pub fn logout_user(auth: &AuthContext) {
    auth.user.set(None);
    auth.token.set(None);

    // Remove token from localStorage
    if let Ok(storage) = web_sys::window().unwrap().local_storage() {
        if let Some(storage) = storage {
            let _ = storage.remove_item("auth_token");
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
