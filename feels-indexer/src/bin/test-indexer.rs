use axum::{Router, routing::get, response::Json};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

#[derive(Serialize)]
struct Status {
    current_slot: u64,
    status: &'static str,
}

#[tokio::main]
async fn main() {
    let slot = Arc::new(Mutex::new(0u64));
    let slot_clone = slot.clone();

    // Poll RPC in background
    tokio::spawn(async move {
        let client = solana_client::rpc_client::RpcClient::new("http://localhost:8899");
        loop {
            if let Ok(s) = client.get_slot() {
                *slot_clone.lock().await = s;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    let app = Router::new()
        .route("/status", get({
            let slot = slot.clone();
            move || async move {
                Json(Status {
                    current_slot: *slot.lock().await,
                    status: "running",
                })
            }
        }))
        .route("/health", get(|| async { "OK" }))
        .layer(CorsLayer::permissive());

    println!("Indexer API running on http://localhost:8080");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
