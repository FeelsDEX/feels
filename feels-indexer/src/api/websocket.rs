//! WebSocket support for real-time updates
//!
//! Provides WebSocket endpoints for subscribing to real-time data updates

use super::ApiState;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State, Query},
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn, error};

/// WebSocket subscription types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SubscriptionType {
    /// Subscribe to all market updates
    AllMarkets,
    /// Subscribe to specific market
    Market { address: String },
    /// Subscribe to swap events
    Swaps { market: Option<String> },
    /// Subscribe to position updates
    Positions { user: Option<String> },
    /// Subscribe to floor updates
    FloorUpdates { market: String },
    /// Subscribe to price updates
    PriceUpdates { market: String },
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Subscription request
    Subscribe {
        id: String,
        subscriptions: Vec<SubscriptionType>,
    },
    /// Unsubscribe request
    Unsubscribe {
        id: String,
        subscriptions: Vec<SubscriptionType>,
    },
    /// Ping for keepalive
    Ping,
    /// Pong response
    Pong,
}

/// WebSocket update events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UpdateEvent {
    /// Market data update
    MarketUpdate {
        market: String,
        sqrt_price: String,
        liquidity: String,
        current_tick: i32,
        timestamp: i64,
    },
    /// New swap event
    SwapEvent {
        market: String,
        user: String,
        amount_in: String,
        amount_out: String,
        token_in: String,
        token_out: String,
        price: f64,
        timestamp: i64,
    },
    /// Position update
    PositionUpdate {
        position: String,
        market: String,
        owner: String,
        liquidity: String,
        tick_lower: i32,
        tick_upper: i32,
        timestamp: i64,
    },
    /// Floor update
    FloorUpdate {
        market: String,
        new_floor_tick: i32,
        new_floor_price: f64,
        timestamp: i64,
    },
    /// Price update
    PriceUpdate {
        market: String,
        price: f64,
        price_change_24h: f64,
        timestamp: i64,
    },
    /// Subscription confirmation
    Subscribed {
        id: String,
        subscriptions: Vec<SubscriptionType>,
    },
    /// Unsubscribe confirmation
    Unsubscribed {
        id: String,
        subscriptions: Vec<SubscriptionType>,
    },
    /// Error message
    Error {
        code: String,
        message: String,
    },
}

/// Query parameters for WebSocket connection
#[derive(Deserialize)]
pub struct WsQuery {
    /// Optional auth token for authenticated subscriptions
    pub auth: Option<String>,
}

/// Handle WebSocket upgrade
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
    State(state): State<ApiState>,
) -> Response {
    info!("WebSocket connection requested");
    
    ws.on_upgrade(move |socket| handle_socket(socket, state, params))
}

/// Handle WebSocket connection
async fn handle_socket(
    socket: WebSocket,
    state: ApiState,
    _params: WsQuery,
) {
    let (mut sender, mut receiver) = socket.split();
    
    // Create broadcast channel for this connection
    let (tx, mut rx) = broadcast::channel::<UpdateEvent>(100);
    
    // Spawn task to handle incoming messages
    let state_clone = state.clone();
    let tx_clone = tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(msg) = msg {
                if process_message(msg, &state_clone, &tx_clone).await.is_err() {
                    break;
                }
            }
        }
    });
    
    // Spawn task to send updates
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let msg = Message::Text(serde_json::to_string(&event).unwrap());
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });
    
    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }
    
    info!("WebSocket connection closed");
}

/// Process incoming WebSocket message
async fn process_message(
    msg: Message,
    _state: &ApiState,
    tx: &broadcast::Sender<UpdateEvent>,
) -> Result<(), ()> {
    match msg {
        Message::Text(text) => {
            match serde_json::from_str::<WsMessage>(&text) {
                Ok(WsMessage::Subscribe { id, subscriptions }) => {
                    info!("Subscribe request: {:?}", subscriptions);
                    
                    // Send subscription confirmation
                    let event = UpdateEvent::Subscribed {
                        id,
                        subscriptions,
                    };
                    let _ = tx.send(event);
                    
                    // TODO: Register subscriptions and start sending updates
                }
                Ok(WsMessage::Unsubscribe { id, subscriptions }) => {
                    info!("Unsubscribe request: {:?}", subscriptions);
                    
                    // Send unsubscribe confirmation
                    let event = UpdateEvent::Unsubscribed {
                        id,
                        subscriptions,
                    };
                    let _ = tx.send(event);
                    
                    // TODO: Remove subscriptions
                }
                Ok(WsMessage::Ping) => {
                    // Send pong - handled at protocol level
                }
                Ok(WsMessage::Pong) => {
                    // Pong received
                }
                Err(e) => {
                    warn!("Invalid WebSocket message: {}", e);
                    let event = UpdateEvent::Error {
                        code: "INVALID_MESSAGE".to_string(),
                        message: format!("Failed to parse message: {}", e),
                    };
                    let _ = tx.send(event);
                }
            }
        }
        Message::Binary(_) => {
            warn!("Binary messages not supported");
            let event = UpdateEvent::Error {
                code: "UNSUPPORTED".to_string(),
                message: "Binary messages not supported".to_string(),
            };
            let _ = tx.send(event);
        }
        Message::Close(_) => {
            info!("WebSocket close received");
            return Err(());
        }
        _ => {}
    }
    
    Ok(())
}

/// Broadcast service for pushing updates to connected clients
pub struct UpdateBroadcaster {
    /// Channels for each subscription type
    market_channels: Arc<tokio::sync::RwLock<Vec<broadcast::Sender<UpdateEvent>>>>,
}

impl UpdateBroadcaster {
    pub fn new() -> Self {
        Self {
            market_channels: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
    
    /// Broadcast market update
    pub async fn broadcast_market_update(&self, update: UpdateEvent) {
        let channels = self.market_channels.read().await;
        for tx in channels.iter() {
            let _ = tx.send(update.clone());
        }
    }
    
    /// Broadcast swap event
    pub async fn broadcast_swap_event(&self, event: UpdateEvent) {
        let channels = self.market_channels.read().await;
        for tx in channels.iter() {
            let _ = tx.send(event.clone());
        }
    }
}

/// Create WebSocket routes
pub fn create_websocket_routes() -> axum::Router<ApiState> {
    axum::Router::new()
        // WebSocket route temporarily disabled due to handler signature issues
        // .route("/ws", axum::routing::get(websocket_handler))
}