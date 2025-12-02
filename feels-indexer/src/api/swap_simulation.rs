//! Swap simulation and quote endpoints
//!
//! Provides real-time swap quotes and simulations using the SDK's SwapSimulator

use super::ApiState;
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use feels_sdk::{
    jupiter::{SwapSimulator, MarketState, TickArrayLoader, ParsedTickArray, TickArrayFormat},
    protocol::sqrt_price_to_price,
};
use tracing::{info, error};

/// Request for swap quote
#[derive(Debug, Deserialize)]
pub struct SwapQuoteRequest {
    /// Amount of input token
    pub amount_in: String,
    /// Input token mint address
    pub token_in: String,
    /// Output token mint address  
    pub token_out: String,
    /// Slippage tolerance in basis points (optional)
    pub slippage_bps: Option<u16>,
}

/// Response for swap quote
#[derive(Debug, Serialize)]
pub struct SwapQuoteResponse {
    /// Input amount
    pub amount_in: String,
    /// Expected output amount
    pub amount_out: String,
    /// Expected output with slippage
    pub min_amount_out: String,
    /// Total fees paid
    pub fee_amount: String,
    /// Price impact in basis points
    pub price_impact_bps: u16,
    /// Execution price
    pub execution_price: f64,
    /// Route through FeelsSOL hub
    pub route: Vec<SwapRoute>,
    /// Current market price
    pub market_price: f64,
    /// Slippage warning if high
    pub slippage_warning: Option<String>,
}

/// Single hop in a swap route
#[derive(Debug, Serialize)]
pub struct SwapRoute {
    /// Source token
    pub from_token: String,
    /// Destination token
    pub to_token: String,
    /// Market address for this hop
    pub market_address: String,
    /// Protocol used (Feels or Jupiter)
    pub protocol: String,
    /// Amount in for this hop
    pub amount_in: String,
    /// Amount out for this hop
    pub amount_out: String,
}

/// Detailed swap simulation request
#[derive(Debug, Deserialize)]
pub struct SwapSimulationRequest {
    /// Market address to simulate on
    pub market_address: String,
    /// Amount of input token
    pub amount_in: String,
    /// Whether swapping token0 to token1
    pub is_token_0_to_1: bool,
    /// Include tick array data in response
    pub include_tick_data: Option<bool>,
}

/// Detailed swap simulation response
#[derive(Debug, Serialize)]
pub struct SwapSimulationResponse {
    /// Input amount
    pub amount_in: String,
    /// Expected output amount
    pub amount_out: String,
    /// Fee paid
    pub fee_paid: String,
    /// Price before swap
    pub price_before: f64,
    /// Price after swap
    pub price_after: f64,
    /// Price impact percentage
    pub price_impact_percent: f64,
    /// Start tick
    pub start_tick: i32,
    /// End tick
    pub end_tick: i32,
    /// Number of ticks crossed
    pub ticks_crossed: u8,
    /// Current liquidity at end
    pub end_liquidity: String,
}

/// Get swap quote with automatic routing
pub async fn get_swap_quote(
    State(state): State<ApiState>,
    Query(params): Query<SwapQuoteRequest>,
) -> impl IntoResponse {
    info!("Swap quote request: {:?}", params);
    
    // Parse token addresses
    let token_in = match Pubkey::from_str(&params.token_in) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid token_in address"
            }))).into_response();
        }
    };
    
    let token_out = match Pubkey::from_str(&params.token_out) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid token_out address"
            }))).into_response();
        }
    };
    
    // Parse amount
    let amount_in: u64 = match params.amount_in.parse() {
        Ok(amt) => amt,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid amount_in"
            }))).into_response();
        }
    };
    
    // Get FeelsSOL mint from environment or use default
    let feelssol_mint = Pubkey::from_str("FEELsso1VoSkqwJQsYq3h3mBGsVZcKbXgssbKdZrmMad")
        .unwrap_or_else(|_| Pubkey::from_str("11111111111111111111111111111112").unwrap());
    
    // Determine route through hub-and-spoke architecture
    let mut route = Vec::new();
    let mut current_amount = amount_in;
    
    // Case 1: Direct swap if one token is FeelsSOL
    if token_in == feelssol_mint || token_out == feelssol_mint {
        // Find the market
        let market = match find_market(&state, &token_in, &token_out).await {
            Ok(market) => market,
            Err(e) => {
                return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                    "error": format!("Market not found: {}", e)
                }))).into_response();
            }
        };
        
        // Simulate the swap
        match simulate_swap_on_market(&state, &market, current_amount, token_in == market.token_0).await {
            Ok(simulation) => {
                route.push(SwapRoute {
                    from_token: token_in.to_string(),
                    to_token: token_out.to_string(),
                    market_address: market.address.to_string(),
                    protocol: "Feels".to_string(),
                    amount_in: current_amount.to_string(),
                    amount_out: simulation.amount_out.to_string(),
                });
                current_amount = simulation.amount_out;
            }
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": format!("Simulation failed: {}", e)
                }))).into_response();
            }
        }
    } else {
        // Case 2: Two-hop swap through FeelsSOL hub
        // First hop: token_in -> FeelsSOL
        let first_market = match find_market(&state, &token_in, &feelssol_mint).await {
            Ok(market) => market,
            Err(_) => {
                // Try Jupiter for external tokens (e.g., SOL -> JitoSOL)
                route.push(SwapRoute {
                    from_token: token_in.to_string(),
                    to_token: feelssol_mint.to_string(),
                    market_address: "jupiter".to_string(),
                    protocol: "Jupiter".to_string(),
                    amount_in: current_amount.to_string(),
                    amount_out: current_amount.to_string(), // Mock 1:1 for now
                });
                current_amount // Keep same amount for mock
            }
        };
        
        // Second hop: FeelsSOL -> token_out
        let second_market = match find_market(&state, &feelssol_mint, &token_out).await {
            Ok(market) => market,
            Err(e) => {
                return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                    "error": format!("No route available: {}", e)
                }))).into_response();
            }
        };
        
        // Simulate second hop
        match simulate_swap_on_market(&state, &second_market, current_amount, feelssol_mint == second_market.token_0).await {
            Ok(simulation) => {
                route.push(SwapRoute {
                    from_token: feelssol_mint.to_string(),
                    to_token: token_out.to_string(),
                    market_address: second_market.address.to_string(),
                    protocol: "Feels".to_string(),
                    amount_in: current_amount.to_string(),
                    amount_out: simulation.amount_out.to_string(),
                });
                current_amount = simulation.amount_out;
            }
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": format!("Simulation failed: {}", e)
                }))).into_response();
            }
        }
    }
    
    // Calculate totals
    let total_fee: u64 = route.iter()
        .map(|r| r.amount_in.parse::<u64>().unwrap_or(0) - r.amount_out.parse::<u64>().unwrap_or(0))
        .sum();
    
    // Calculate slippage
    let slippage_bps = params.slippage_bps.unwrap_or(100); // Default 1%
    let min_amount_out = current_amount * (10000 - slippage_bps) as u64 / 10000;
    
    // Price impact calculation (simplified)
    let price_impact_bps = if amount_in > 0 {
        ((amount_in as f64 - current_amount as f64) / amount_in as f64 * 10000.0) as u16
    } else {
        0
    };
    
    let response = SwapQuoteResponse {
        amount_in: amount_in.to_string(),
        amount_out: current_amount.to_string(),
        min_amount_out: min_amount_out.to_string(),
        fee_amount: total_fee.to_string(),
        price_impact_bps,
        execution_price: if current_amount > 0 { amount_in as f64 / current_amount as f64 } else { 0.0 },
        route,
        market_price: 1.0, // TODO: Get from oracle
        slippage_warning: if price_impact_bps > 500 {
            Some(format!("High price impact: {:.2}%", price_impact_bps as f64 / 100.0))
        } else {
            None
        },
    };
    
    (StatusCode::OK, Json(response)).into_response()
}

/// Simulate swap on specific market
pub async fn simulate_swap(
    State(state): State<ApiState>,
    Json(params): Json<SwapSimulationRequest>,
) -> impl IntoResponse {
    info!("Swap simulation request: {:?}", params);
    
    // Parse market address
    let market_address = match Pubkey::from_str(&params.market_address) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid market address"
            }))).into_response();
        }
    };
    
    // Parse amount
    let amount_in: u64 = match params.amount_in.parse() {
        Ok(amt) => amt,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid amount_in"
            }))).into_response();
        }
    };
    
    // Get market from database
    let market = match state.db.get_market(&market_address).await {
        Ok(Some(market)) => market,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Market not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Database error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to fetch market"
            }))).into_response();
        }
    };
    
    // Create market state for simulation
    let token_0 = Pubkey::from_str(&market.token_0)
        .map_err(|_| "Invalid token_0")?;
    let token_1 = Pubkey::from_str(&market.token_1)
        .map_err(|_| "Invalid token_1")?;
    
    // Convert decimal types to u128
    let sqrt_price: u128 = market.sqrt_price.to_string().parse()
        .map_err(|_| "Failed to parse sqrt_price")?;
    let liquidity: u128 = market.liquidity.to_string().parse()
        .map_err(|_| "Failed to parse liquidity")?;
    
    let market_state = MarketState {
        market_key: market_address,
        token_0,
        token_1,
        sqrt_price,
        current_tick: market.current_tick,
        liquidity,
        fee_bps: market.fee_bps as u16,
        tick_spacing: market.tick_spacing as u16,
        global_lower_tick: -887272, // Default bounds
        global_upper_tick: 887272,
        fee_growth_global_0: 0,
        fee_growth_global_1: 0,
    };
    
    // For now, use empty tick arrays (no initialized ticks)
    // In production, would fetch from RPC or cache
    let tick_arrays = TickArrayLoader::new();
    
    // Run simulation
    let simulator = SwapSimulator::new(&market_state, &tick_arrays);
    let result = match simulator.simulate_swap(amount_in, params.is_token_0_to_1) {
        Ok(result) => result,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Simulation failed: {}", e)
            }))).into_response();
        }
    };
    
    // Calculate prices
    let price_before = sqrt_price_to_price(market_state.sqrt_price);
    let price_after = sqrt_price_to_price(result.end_sqrt_price);
    let price_impact = ((price_after - price_before) / price_before * 100.0).abs();
    
    let response = SwapSimulationResponse {
        amount_in: result.amount_in.to_string(),
        amount_out: result.amount_out.to_string(),
        fee_paid: result.fee_paid.to_string(),
        price_before,
        price_after,
        price_impact_percent: price_impact,
        start_tick: market_state.current_tick,
        end_tick: result.end_tick,
        ticks_crossed: result.ticks_crossed,
        end_liquidity: market_state.liquidity.to_string(), // Simplified
    };
    
    (StatusCode::OK, Json(response)).into_response()
}

// Helper functions

async fn find_market(
    state: &ApiState, 
    token_a: &Pubkey, 
    token_b: &Pubkey
) -> Result<crate::database::Market, String> {
    // Markets always have FeelsSOL as token_0
    let feelssol_mint = Pubkey::from_str("FEELsso1VoSkqwJQsYq3h3mBGsVZcKbXgssbKdZrmMad")
        .unwrap_or_else(|_| Pubkey::from_str("11111111111111111111111111111112").unwrap());
    
    // Determine correct token ordering
    let (token_0, token_1) = if *token_a == feelssol_mint {
        (*token_a, *token_b)
    } else if *token_b == feelssol_mint {
        (*token_b, *token_a)
    } else {
        return Err("Neither token is FeelsSOL".to_string());
    };
    
    // Query database for market
    match state.db.find_market_by_tokens(&token_0, &token_1).await {
        Ok(Some(market)) => Ok(market),
        Ok(None) => Err("Market not found".to_string()),
        Err(e) => Err(format!("Database error: {}", e)),
    }
}

async fn simulate_swap_on_market(
    _state: &ApiState,
    market: &crate::database::Market,
    amount_in: u64,
    is_token_0_to_1: bool,
) -> Result<String, String> {  // Temporarily return String instead of feels_sdk::SwapSimulation
    // Create market state
    
    let market_address = Pubkey::from_str(&market.address)
        .map_err(|e| format!("Invalid market address: {}", e))?;
    let token_0 = Pubkey::from_str(&market.token_0)
        .map_err(|e| format!("Invalid token_0: {}", e))?;
    let token_1 = Pubkey::from_str(&market.token_1)
        .map_err(|e| format!("Invalid token_1: {}", e))?;
    
    // Convert decimal types to u128
    let sqrt_price: u128 = market.sqrt_price.to_string().parse()
        .map_err(|_| "Failed to parse sqrt_price")?;
    let liquidity: u128 = market.liquidity.to_string().parse()
        .map_err(|_| "Failed to parse liquidity")?;
    
    let market_state = MarketState {
        market_key: market_address,
        token_0,
        token_1,
        sqrt_price,
        current_tick: market.current_tick,
        liquidity,
        fee_bps: market.fee_bps as u16,
        tick_spacing: market.tick_spacing as u16,
        global_lower_tick: -887272,
        global_upper_tick: 887272,
        fee_growth_global_0: 0,
        fee_growth_global_1: 0,
    };
    
    // Empty tick arrays for now
    let tick_arrays = TickArrayLoader::new();
    
    // Run simulation
    let simulator = SwapSimulator::new(&market_state, &tick_arrays);
    simulator.simulate_swap(amount_in, is_token_0_to_1)
        .map_err(|e| format!("Simulation error: {}", e))
}