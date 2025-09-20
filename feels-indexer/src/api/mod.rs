//! REST API for querying indexed Feels Protocol data

mod handlers;
mod routes;
mod responses;

pub use routes::*;

use crate::config::ApiConfig;
use crate::database::DatabaseManager;
use anyhow::Result;
use axum::{
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

/// Start the API server
pub async fn start_server(
    db_manager: Arc<DatabaseManager>,
    config: &ApiConfig,
) -> Result<tokio::task::JoinHandle<()>> {
    let app = create_app(db_manager).await?;
    
    let listener = TcpListener::bind(&config.bind_address).await?;
    info!("API server listening on {}", config.bind_address);
    
    let handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("API server error: {}", e);
        }
    });
    
    Ok(handle)
}

/// Start the metrics server
pub async fn start_metrics_server(port: u16) -> Result<tokio::task::JoinHandle<()>> {
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler));
    
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    info!("Metrics server listening on {}", addr);
    
    let handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("Metrics server error: {}", e);
        }
    });
    
    Ok(handle)
}

/// Create the main API application
async fn create_app(db_manager: Arc<DatabaseManager>) -> Result<Router> {
    let api_state = ApiState::new(db_manager);
    
    let app = Router::new()
        .merge(create_market_routes())
        .merge(create_swap_routes())
        .merge(create_position_routes())
        .merge(create_protocol_routes())
        .route("/health", get(health_handler))
        .with_state(api_state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
        );
    
    Ok(app)
}

/// Health check handler
async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().timestamp(),
        "service": "feels-indexer"
    }))
}

/// Metrics handler (placeholder)
async fn metrics_handler() -> Result<String, StatusCode> {
    // In a real implementation, this would return Prometheus metrics
    Ok("# Feels Indexer Metrics\n# TODO: Implement metrics\n".to_string())
}

/// Shared API state
#[derive(Clone)]
pub struct ApiState {
    pub db_manager: Arc<DatabaseManager>,
}

impl ApiState {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self { db_manager }
    }
}
