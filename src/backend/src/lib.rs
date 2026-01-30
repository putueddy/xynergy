use axum::{
    routing::get,
    Router,
    response::{Html, Response},
    body::Body,
};
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

pub mod config;
pub mod db;
pub mod error;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;

pub use config::Config;
pub use db::Database;
pub use error::{AppError, Result};

/// Initialize logging
pub fn init_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set subscriber");
}

/// Serve the index.html file with Leptos
async fn serve_index() -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Xynergy - Resource Management</title>
    <link rel="stylesheet" href="/output.css">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
    <style>
        body {
            font-family: 'Inter', system-ui, sans-serif;
        }
        #loading {
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: #f9fafb;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            z-index: 9999;
        }
        .spinner {
            width: 40px;
            height: 40px;
            border: 4px solid #e5e7eb;
            border-top: 4px solid #3b82f6;
            border-radius: 50%;
            animation: spin 1s linear infinite;
            margin-bottom: 16px;
        }
        @keyframes spin {
            0% { transform: rotate(0deg); }
            100% { transform: rotate(360deg); }
        }
        #loading-text {
            color: #6b7280;
            font-size: 14px;
        }
        #error-message {
            display: none;
            color: #dc2626;
            font-size: 14px;
            text-align: center;
            max-width: 400px;
            padding: 20px;
        }
    </style>
</head>
<body>
    <div id="loading">
        <div class="spinner"></div>
        <div id="loading-text">Loading Xynergy...</div>
        <div id="error-message"></div>
    </div>
    <div id="root"></div>
    
    <script>
        // Debug: Check body content after mount
        setTimeout(() => {
            console.log('Body content:', document.body.innerHTML.substring(0, 500));
            console.log('Body children count:', document.body.children.length);
        }, 1000);
    </script>
    
    <script type="module">
        console.log('Starting Xynergy app...');
        
        const loadingText = document.getElementById('loading-text');
        const errorMessage = document.getElementById('error-message');
        
        // Import the WASM module
        import('/pkg/xynergy_frontend.js')
            .then(module => {
                console.log('WASM module loaded:', module);
                
                // Initialize the WASM module
                return module.default();
            })
            .then(() => {
                console.log('WASM initialized, starting Leptos...');
                
                // The WASM is loaded but we need to make sure main() was called
                // Check if content was rendered
                setTimeout(() => {
                    const root = document.getElementById('root');
                    const hasContent = root && root.innerHTML.trim().length > 0;
                    console.log('Root has content:', hasContent);
                    console.log('Root content preview:', root ? root.innerHTML.substring(0, 200) : 'not found');
                    
                    const loading = document.getElementById('loading');
                    if (loading) {
                        loading.style.display = 'none';
                    }
                    console.log('Loading spinner hidden');
                    
                    if (!hasContent) {
                        console.error('ERROR: Leptos did not render any content!');
                        errorMessage.style.display = 'block';
                        errorMessage.innerHTML = `
                            <h3>Application Error</h3>
                            <p>The application failed to render.</p>
                            <p>Please check the console for details.</p>
                        `;
                    }
                }, 500);
            })
            .catch(err => {
                console.error('Failed to load Xynergy:', err);
                loadingText.style.display = 'none';
                errorMessage.style.display = 'block';
                errorMessage.innerHTML = `
                    <h3>Failed to load application</h3>
                    <p>Error: ${err.message}</p>
                    <p>Please check the browser console for details.</p>
                    <button onclick="location.reload()" style="margin-top: 10px; padding: 8px 16px; background: #3b82f6; color: white; border: none; border-radius: 4px; cursor: pointer;">
                        Refresh Page
                    </button>
                `;
            });
    </script>
</body>
</html>
"#;
    
    Html(html.to_string())
}

/// Serve static CSS file
async fn serve_css() -> Result<Response> {
    let project_root = std::env::current_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    
    let css_path = project_root.join("src/frontend/public/output.css");
    
    let css_content = tokio::fs::read_to_string(&css_path)
        .await
        .map_err(|e| {
            tracing::error!("Failed to read CSS file: {}", e);
            AppError::NotFound("CSS file not found".to_string())
        })?;
    
    Ok(Response::builder()
        .header("content-type", "text/css")
        .body(Body::from(css_content))
        .unwrap())
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// Hello world endpoint for testing
async fn hello_world() -> &'static str {
    "Hello from Xynergy Backend!"
}

/// Create the Axum application router
pub fn create_app(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/health", get(health_check))
        .route("/api/v1/hello", get(hello_world))
        .route("/output.css", get(serve_css))
        .nest("/api/v1", api_routes())
        .nest_service("/pkg", ServeDir::new("target/site/pkg"))
        // Serve index.html for all other routes (SPA fallback)
        .fallback(serve_index)
        .with_state(pool)
}

/// API routes
fn api_routes() -> Router<PgPool> {
    Router::new()
        .merge(routes::auth_routes())
        .merge(routes::department_routes())
        .merge(routes::user_routes())
        .merge(routes::resource_routes())
        .merge(routes::project_routes())
}

/// Run the server
pub async fn run_server(addr: SocketAddr) -> Result<()> {
    init_logging();
    
    // Initialize database
    let db = Database::new().await?;
    info!("Database connected successfully");
    
    let app = create_app(db.pool().clone());
    
    info!("Starting server on {}", addr);
    info!("Visit http://{} to see your app", addr);
    info!("API endpoints available at http://{}/api/v1/", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check() {
        // This is a placeholder test
        assert_eq!(2 + 2, 4);
    }
}
