//! API route definitions

use super::{ApiState, handlers::*};
use axum::{
    routing::{get, post},
    Router,
};

/// Create market-related routes
pub fn create_market_routes() -> Router<ApiState> {
    Router::new()
        .route("/markets", get(list_markets))
        .route("/markets/:address", get(get_market))
        .route("/markets/:address/stats", get(get_market_stats))
        .route("/markets/:address/swaps", get(get_market_swaps))
        .route("/markets/:address/positions", get(get_market_positions))
        .route("/markets/:address/floor", get(get_market_floor))
        .route("/markets/:address/ohlcv", get(get_market_ohlcv))
}

/// Create swap-related routes
pub fn create_swap_routes() -> Router<ApiState> {
    Router::new()
        .route("/swaps", get(list_swaps))
        .route("/swaps/:signature", get(get_swap))
        .route("/users/:address/swaps", get(get_user_swaps))
        // Temporarily disabled - need dependencies
        // .route("/swap/quote", get(crate::api::swap_simulation::get_swap_quote))
        // .route("/swap/simulate", post(crate::api::swap_simulation::simulate_swap))
        // .route("/swap/build", post(crate::api::transaction_builder::build_swap_transaction))
        // .route("/tx/simulate", post(crate::api::transaction_builder::simulate_transaction))
}

/// Create position-related routes
pub fn create_position_routes() -> Router<ApiState> {
    Router::new()
        .route("/positions", get(list_positions))
        .route("/positions/:address", get(get_position))
        .route("/users/:address/positions", get(get_user_positions))
}

/// Create protocol-level routes
pub fn create_protocol_routes() -> Router<ApiState> {
    Router::new()
        .route("/protocol/stats", get(get_protocol_stats))
        .route("/protocol/markets", get(get_protocol_markets))
        .route("/protocol/volume", get(get_protocol_volume))
}

// Token-related routes (temporarily disabled)
// pub fn create_token_routes() -> Router<ApiState> {
//     Router::new()
//         .route("/tokens/:mint/balance/:wallet", get(crate::api::token_balance::get_token_balance))
//         .route("/wallets/:wallet/balances", get(crate::api::token_balance::get_wallet_balances))
// }
