//! Jupiter API integration for entry/exit quotes
//!
//! Handles external swaps for entering/exiting the Feels ecosystem

use super::ApiState;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use reqwest;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tracing::{info, error};

/// Jupiter quote API endpoint
const JUPITER_QUOTE_API: &str = "https://quote-api.jup.ag/v6/quote";
const JUPITER_SWAP_API: &str = "https://quote-api.jup.ag/v6/swap";

/// Known token mints
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const JITOSOL_MINT: &str = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";
const FEELSSOL_MINT: &str = "FEELsso1VoSkqwJQsYq3h3mBGsVZcKbXgssbKdZrmMad";

/// Entry quote request - convert SOL/JitoSOL to FeelsSOL
#[derive(Debug, Deserialize)]
pub struct EntryQuoteRequest {
    /// Input token mint (SOL or JitoSOL)
    pub input_mint: String,
    /// Amount of input tokens
    pub amount: String,
    /// Slippage in basis points
    pub slippage_bps: Option<u16>,
}

/// Exit quote request - convert FeelsSOL to SOL/JitoSOL
#[derive(Debug, Deserialize)]
pub struct ExitQuoteRequest {
    /// Output token mint (SOL or JitoSOL)
    pub output_mint: String,
    /// Amount of FeelsSOL to exit
    pub amount: String,
    /// Slippage in basis points
    pub slippage_bps: Option<u16>,
}

/// Entry/Exit quote response
#[derive(Debug, Serialize)]
pub struct EntryExitQuoteResponse {
    /// Input token
    pub input_mint: String,
    /// Output token
    pub output_mint: String,
    /// Input amount
    pub in_amount: String,
    /// Expected output amount
    pub out_amount: String,
    /// Minimum output amount with slippage
    pub min_out_amount: String,
    /// Price impact in basis points
    pub price_impact_bps: u16,
    /// Execution price
    pub price: f64,
    /// Route details
    pub route: Vec<RouteStep>,
    /// Whether this uses Jupiter
    pub uses_jupiter: bool,
}

/// Single step in the route
#[derive(Debug, Serialize)]
pub struct RouteStep {
    /// Source token
    pub input_mint: String,
    /// Destination token
    pub output_mint: String,
    /// DEX/AMM name
    pub amm: String,
    /// Input amount for this step
    pub in_amount: String,
    /// Output amount for this step
    pub out_amount: String,
}

/// Jupiter quote response (simplified)
#[derive(Debug, Deserialize)]
struct JupiterQuoteResponse {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    #[serde(rename = "inAmount")]
    in_amount: String,
    #[serde(rename = "outAmount")]
    out_amount: String,
    #[serde(rename = "priceImpactPct")]
    price_impact_pct: String,
    #[serde(rename = "routePlan")]
    route_plan: Vec<JupiterRoutePlan>,
}

#[derive(Debug, Deserialize)]
struct JupiterRoutePlan {
    #[serde(rename = "swapInfo")]
    swap_info: JupiterSwapInfo,
}

#[derive(Debug, Deserialize)]
struct JupiterSwapInfo {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    #[serde(rename = "inAmount")]
    in_amount: String,
    #[serde(rename = "outAmount")]
    out_amount: String,
    #[serde(rename = "ammKey")]
    amm_key: String,
    label: Option<String>,
}

/// Get entry quote (SOL/JitoSOL -> FeelsSOL)
pub async fn get_entry_quote(
    State(state): State<ApiState>,
    Query(params): Query<EntryQuoteRequest>,
) -> impl IntoResponse {
    info!("Entry quote request: {:?}", params);
    
    // Validate input mint
    let input_mint = match Pubkey::from_str(&params.input_mint) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid input mint"
            }))).into_response();
        }
    };
    
    // Check if input is SOL or JitoSOL
    let is_sol = input_mint.to_string() == SOL_MINT;
    let is_jitosol = input_mint.to_string() == JITOSOL_MINT;
    
    if !is_sol && !is_jitosol {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Input must be SOL or JitoSOL"
        }))).into_response();
    }
    
    let amount: u64 = match params.amount.parse() {
        Ok(amt) => amt,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid amount"
            }))).into_response();
        }
    };
    
    // If input is SOL, we need Jupiter to convert SOL -> JitoSOL first
    if is_sol {
        // Get Jupiter quote for SOL -> JitoSOL
        match get_jupiter_quote(&params.input_mint, JITOSOL_MINT, &params.amount).await {
            Ok(jupiter_quote) => {
                // Then internally convert JitoSOL -> FeelsSOL (1:1)
                let jitosol_amount = jupiter_quote.out_amount.parse::<u64>().unwrap_or(0);
                let slippage_bps = params.slippage_bps.unwrap_or(100);
                let min_out = jitosol_amount * (10000 - slippage_bps) as u64 / 10000;
                
                let response = EntryExitQuoteResponse {
                    input_mint: params.input_mint.clone(),
                    output_mint: FEELSSOL_MINT.to_string(),
                    in_amount: params.amount.clone(),
                    out_amount: jitosol_amount.to_string(),
                    min_out_amount: min_out.to_string(),
                    price_impact_bps: (jupiter_quote.price_impact_pct.parse::<f64>().unwrap_or(0.0) * 100.0) as u16,
                    price: amount as f64 / jitosol_amount as f64,
                    route: vec![
                        RouteStep {
                            input_mint: SOL_MINT.to_string(),
                            output_mint: JITOSOL_MINT.to_string(),
                            amm: jupiter_quote.route_plan.first()
                                .and_then(|r| r.swap_info.label.clone())
                                .unwrap_or_else(|| "Jupiter".to_string()),
                            in_amount: params.amount.clone(),
                            out_amount: jupiter_quote.out_amount.clone(),
                        },
                        RouteStep {
                            input_mint: JITOSOL_MINT.to_string(),
                            output_mint: FEELSSOL_MINT.to_string(),
                            amm: "Feels".to_string(),
                            in_amount: jupiter_quote.out_amount.clone(),
                            out_amount: jupiter_quote.out_amount.clone(), // 1:1 conversion
                        },
                    ],
                    uses_jupiter: true,
                };
                
                (StatusCode::OK, Json(response)).into_response()
            }
            Err(e) => {
                error!("Jupiter quote failed: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": "Failed to get Jupiter quote"
                }))).into_response()
            }
        }
    } else {
        // Direct JitoSOL -> FeelsSOL conversion (1:1)
        let slippage_bps = params.slippage_bps.unwrap_or(100);
        let min_out = amount * (10000 - slippage_bps) as u64 / 10000;
        
        let response = EntryExitQuoteResponse {
            input_mint: params.input_mint.clone(),
            output_mint: FEELSSOL_MINT.to_string(),
            in_amount: params.amount.clone(),
            out_amount: params.amount.clone(), // 1:1
            min_out_amount: min_out.to_string(),
            price_impact_bps: 0,
            price: 1.0,
            route: vec![
                RouteStep {
                    input_mint: JITOSOL_MINT.to_string(),
                    output_mint: FEELSSOL_MINT.to_string(),
                    amm: "Feels".to_string(),
                    in_amount: params.amount.clone(),
                    out_amount: params.amount.clone(),
                },
            ],
            uses_jupiter: false,
        };
        
        (StatusCode::OK, Json(response)).into_response()
    }
}

/// Get exit quote (FeelsSOL -> SOL/JitoSOL)
pub async fn get_exit_quote(
    State(state): State<ApiState>,
    Query(params): Query<ExitQuoteRequest>,
) -> impl IntoResponse {
    info!("Exit quote request: {:?}", params);
    
    // Validate output mint
    let output_mint = match Pubkey::from_str(&params.output_mint) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid output mint"
            }))).into_response();
        }
    };
    
    // Check if output is SOL or JitoSOL
    let is_sol = output_mint.to_string() == SOL_MINT;
    let is_jitosol = output_mint.to_string() == JITOSOL_MINT;
    
    if !is_sol && !is_jitosol {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Output must be SOL or JitoSOL"
        }))).into_response();
    }
    
    let amount: u64 = match params.amount.parse() {
        Ok(amt) => amt,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid amount"
            }))).into_response();
        }
    };
    
    // If output is SOL, we need Jupiter to convert JitoSOL -> SOL after
    if is_sol {
        // First convert FeelsSOL -> JitoSOL (1:1)
        // Then get Jupiter quote for JitoSOL -> SOL
        match get_jupiter_quote(JITOSOL_MINT, &params.output_mint, &params.amount).await {
            Ok(jupiter_quote) => {
                let sol_amount = jupiter_quote.out_amount.parse::<u64>().unwrap_or(0);
                let slippage_bps = params.slippage_bps.unwrap_or(100);
                let min_out = sol_amount * (10000 - slippage_bps) as u64 / 10000;
                
                let response = EntryExitQuoteResponse {
                    input_mint: FEELSSOL_MINT.to_string(),
                    output_mint: params.output_mint.clone(),
                    in_amount: params.amount.clone(),
                    out_amount: sol_amount.to_string(),
                    min_out_amount: min_out.to_string(),
                    price_impact_bps: (jupiter_quote.price_impact_pct.parse::<f64>().unwrap_or(0.0) * 100.0) as u16,
                    price: amount as f64 / sol_amount as f64,
                    route: vec![
                        RouteStep {
                            input_mint: FEELSSOL_MINT.to_string(),
                            output_mint: JITOSOL_MINT.to_string(),
                            amm: "Feels".to_string(),
                            in_amount: params.amount.clone(),
                            out_amount: params.amount.clone(), // 1:1 conversion
                        },
                        RouteStep {
                            input_mint: JITOSOL_MINT.to_string(),
                            output_mint: SOL_MINT.to_string(),
                            amm: jupiter_quote.route_plan.first()
                                .and_then(|r| r.swap_info.label.clone())
                                .unwrap_or_else(|| "Jupiter".to_string()),
                            in_amount: params.amount.clone(),
                            out_amount: jupiter_quote.out_amount.clone(),
                        },
                    ],
                    uses_jupiter: true,
                };
                
                (StatusCode::OK, Json(response)).into_response()
            }
            Err(e) => {
                error!("Jupiter quote failed: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": "Failed to get Jupiter quote"
                }))).into_response()
            }
        }
    } else {
        // Direct FeelsSOL -> JitoSOL conversion (1:1)
        let slippage_bps = params.slippage_bps.unwrap_or(100);
        let min_out = amount * (10000 - slippage_bps) as u64 / 10000;
        
        let response = EntryExitQuoteResponse {
            input_mint: FEELSSOL_MINT.to_string(),
            output_mint: params.output_mint.clone(),
            in_amount: params.amount.clone(),
            out_amount: params.amount.clone(), // 1:1
            min_out_amount: min_out.to_string(),
            price_impact_bps: 0,
            price: 1.0,
            route: vec![
                RouteStep {
                    input_mint: FEELSSOL_MINT.to_string(),
                    output_mint: JITOSOL_MINT.to_string(),
                    amm: "Feels".to_string(),
                    in_amount: params.amount.clone(),
                    out_amount: params.amount.clone(),
                },
            ],
            uses_jupiter: false,
        };
        
        (StatusCode::OK, Json(response)).into_response()
    }
}

/// Get quote from Jupiter API
async fn get_jupiter_quote(
    input_mint: &str,
    output_mint: &str,
    amount: &str,
) -> Result<JupiterQuoteResponse, String> {
    let client = reqwest::Client::new();
    
    let url = format!(
        "{}?inputMint={}&outputMint={}&amount={}&slippageBps=50",
        JUPITER_QUOTE_API, input_mint, output_mint, amount
    );
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Jupiter API error: {}", response.status()));
    }
    
    response
        .json::<JupiterQuoteResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Create Jupiter integration routes
pub fn create_jupiter_routes() -> axum::Router<ApiState> {
    axum::Router::new()
        .route("/entry/quote", axum::routing::get(get_entry_quote))
        .route("/exit/quote", axum::routing::get(get_exit_quote))
}