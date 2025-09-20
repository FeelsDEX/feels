use anyhow::Result;
use axum::{
    extract::State,
    response::Json,
    routing::get,
    Router,
};
use clap::Parser;
use feels_indexer::streaming_client::{StreamingClient, StreamingUpdate};
use serde::Serialize;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

#[derive(Parser, Debug)]
#[clap(name = "feels-indexer-streaming")]
#[clap(about = "Feels indexer with streaming support", long_about = None)]
struct Args {
    /// Streaming endpoint (for local dev, use localhost:10000)
    #[clap(short, long, default_value = "http://localhost:10000")]
    streaming_endpoint: String,

    /// HTTP API port
    #[clap(short, long, default_value = "8080")]
    port: u16,

    /// Feels program ID to monitor
    #[clap(short = 'p', long)]
    program_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct IndexerStatus {
    current_slot: u64,
    accounts_indexed: usize,
    transactions_indexed: usize,
    status: String,
}

#[derive(Clone)]
struct AppState {
    current_slot: Arc<Mutex<u64>>,
    account_count: Arc<Mutex<usize>>,
    transaction_count: Arc<Mutex<usize>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("feels_indexer=info,feels_indexer_streaming=info")
        .init();

    let args = Args::parse();
    info!("Starting Feels Indexer with Streaming");
    info!("Streaming endpoint: {}", args.streaming_endpoint);
    info!("API port: {}", args.port);

    // Parse program ID
    let program_id = if let Some(id) = args.program_id {
        Some(Pubkey::from_str(&id)?)
    } else {
        // Default to a test program ID
        None
    };

    // Create shared state
    let state = AppState {
        current_slot: Arc::new(Mutex::new(0)),
        account_count: Arc::new(Mutex::new(0)),
        transaction_count: Arc::new(Mutex::new(0)),
    };

    // Create channel for streaming updates
    let (tx, mut rx) = mpsc::channel::<StreamingUpdate>(1000);

    // Spawn streaming client
    let streaming_client = StreamingClient::new(args.streaming_endpoint);
    let streaming_client = if let Some(program_id) = program_id {
        info!("Filtering for program: {}", program_id);
        streaming_client.with_program_filter(program_id)
    } else {
        streaming_client
    };

    let stream_handle = tokio::spawn(async move {
        if let Err(e) = streaming_client.connect_and_stream(tx).await {
            error!("Streaming error: {}", e);
        }
    });

    // Spawn update processor
    let processor_state = state.clone();
    let processor_handle = tokio::spawn(async move {
        while let Some(update) = rx.recv().await {
            match update {
                StreamingUpdate::Slot(slot_update) => {
                    info!("New slot: {}", slot_update.slot);
                    *processor_state.current_slot.lock().await = slot_update.slot;
                }
                StreamingUpdate::Account(account_update) => {
                    info!(
                        "Account update: {} at slot {}",
                        account_update.pubkey, account_update.slot
                    );
                    *processor_state.account_count.lock().await += 1;
                }
                StreamingUpdate::Transaction(tx_update) => {
                    info!(
                        "Transaction: {} at slot {}",
                        tx_update.signature, tx_update.slot
                    );
                    *processor_state.transaction_count.lock().await += 1;
                }
            }
        }
    });

    // Create HTTP API
    let app = Router::new()
        .route("/", get(root))
        .route("/status", get(get_status))
        .route("/health", get(health))
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        );

    let addr = format!("0.0.0.0:{}", args.port);
    info!("Starting HTTP API on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    stream_handle.await?;
    processor_handle.await?;

    Ok(())
}

async fn root() -> &'static str {
    "Feels Indexer API v1.0"
}

async fn get_status(State(state): State<AppState>) -> Json<IndexerStatus> {
    let current_slot = *state.current_slot.lock().await;
    let accounts_indexed = *state.account_count.lock().await;
    let transactions_indexed = *state.transaction_count.lock().await;

    Json(IndexerStatus {
        current_slot,
        accounts_indexed,
        transactions_indexed,
        status: "streaming".to_string(),
    })
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}