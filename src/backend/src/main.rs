use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    xynergy_backend::init_logging();
    
    info!("Starting Xynergy server...");
    
    // Initialize database
    let db = xynergy_backend::Database::new().await?;
    info!("Database connected successfully");
    
    // Create the application
    let app = xynergy_backend::create_app(db.pool().clone());
    
    // Bind to address
    let addr = "127.0.0.1:3000";
    let listener = TcpListener::bind(addr).await?;
    
    info!("Server running on http://{}", addr);
    info!("API endpoints available at http://{}/api/v1/", addr);
    
    // Run the server
    axum::serve(listener, app).await?;
    
    Ok(())
}
