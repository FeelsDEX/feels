use anyhow::Result;
use axum::{
    extract::State,
    response::{Json, sse::{Event, Sse}},
    routing::get,
    Router,
};
use clap::Parser;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt as _};
use tower_http::cors::CorsLayer;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamUpdate {
    pub slot: u64,
    pub update_type: String,
    pub data: serde_json::Value,
}

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long, default_value = "http://localhost:8899")]
    rpc_url: String,

    #[clap(short, long, default_value = "10000")]
    port: u16,

    #[clap(short = 'p', long)]
    program_id: Option<String>,
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<StreamUpdate>,
}

#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<serde_json::Value>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let args = Args::parse();
    info!("Starting Minimal Streaming Adapter");
    info!("RPC: {}", args.rpc_url);
    info!("Port: {}", args.port);

    let (tx, _) = broadcast::channel::<StreamUpdate>(1000);

    // Start RPC poller
    let poller_tx = tx.clone();
    let rpc_url = args.rpc_url.clone();
    let program_id = args.program_id.clone();
    
    tokio::spawn(async move {
        poll_rpc(rpc_url, poller_tx, program_id).await;
    });

    // Create HTTP server
    let app = Router::new()
        .route("/", get(root))
        .route("/stream", get(sse_handler))
        .route("/health", get(|| async { "OK" }))
        .route("/status", get(status))
        .with_state(AppState { tx })
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port)).await?;
    info!("Server running on http://0.0.0.0:{}", args.port);
    
    axum::serve(listener, app).await?;
    Ok(())
}

async fn root() -> &'static str {
    "Minimal Streaming Adapter\n\nEndpoints:\n- /stream - SSE stream of blockchain updates\n- /status - Current status\n- /health - Health check"
}

async fn status() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "running",
        "version": "0.1.0",
        "endpoints": {
            "stream": "/stream",
            "health": "/health"
        }
    }))
}

async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, anyhow::Error>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).map(|msg| {
        match msg {
            Ok(update) => {
                let data = serde_json::to_string(&update).unwrap_or_default();
                Ok(Event::default().data(data))
            }
            Err(_) => Ok(Event::default().data("error")),
        }
    });

    Sse::new(stream)
}

async fn poll_rpc(rpc_url: String, tx: broadcast::Sender<StreamUpdate>, program_id: Option<String>) {
    let client = reqwest::Client::new();
    let mut last_slot = 0u64;
    let mut request_id = 1u64;

    loop {
        // Poll slot
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: "getSlot".to_string(),
            params: vec![],
        };
        request_id += 1;

        if let Ok(response) = client.post(&rpc_url)
            .json(&request)
            .send()
            .await
        {
            if let Ok(rpc_response) = response.json::<RpcResponse<u64>>().await {
                if let Some(slot) = rpc_response.result {
                    if slot > last_slot {
                        last_slot = slot;
                        let update = StreamUpdate {
                            slot,
                            update_type: "slot".to_string(),
                            data: serde_json::json!({
                                "parent": slot.saturating_sub(1),
                                "status": "confirmed"
                            }),
                        };
                        let _ = tx.send(update);
                        info!("Slot: {}", slot);
                    }
                }
            }
        }

        // If program ID provided, poll accounts (simplified)
        if let Some(ref program_id) = program_id {
            let request = RpcRequest {
                jsonrpc: "2.0".to_string(),
                id: request_id,
                method: "getProgramAccounts".to_string(),
                params: vec![
                    serde_json::Value::String(program_id.clone()),
                    serde_json::json!({
                        "encoding": "base64",
                        "dataSlice": {
                            "offset": 0,
                            "length": 0
                        }
                    })
                ],
            };
            request_id += 1;

            if let Ok(response) = client.post(&rpc_url)
                .json(&request)
                .send()
                .await
            {
                if let Ok(rpc_response) = response.json::<RpcResponse<Vec<serde_json::Value>>>().await {
                    if let Some(accounts) = rpc_response.result {
                        for (idx, account) in accounts.iter().take(5).enumerate() {
                            if let Some(pubkey) = account.get("pubkey").and_then(|p| p.as_str()) {
                                let update = StreamUpdate {
                                    slot: last_slot,
                                    update_type: "account".to_string(),
                                    data: serde_json::json!({
                                        "pubkey": pubkey,
                                        "program": program_id,
                                        "index": idx
                                    }),
                                };
                                let _ = tx.send(update);
                            }
                        }
                    }
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}