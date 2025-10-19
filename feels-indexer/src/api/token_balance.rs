//! Token balance fetching endpoints
//!
//! Provides real-time token balance queries using RPC

use super::ApiState;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, account::Account};
use crate::rpc_client::LightRpcClient;
use spl_token::state::Account as TokenAccount;
use std::str::FromStr;
use tracing::{info, error};

/// Token balance response
#[derive(Debug, Serialize)]
pub struct TokenBalanceResponse {
    /// Token mint address
    pub mint: String,
    /// Token symbol (if known)
    pub symbol: Option<String>,
    /// User's balance
    pub balance: String,
    /// UI-formatted balance (with decimals)
    pub ui_balance: f64,
    /// Token decimals
    pub decimals: u8,
}

/// Multiple token balances response
#[derive(Debug, Serialize)]
pub struct TokenBalancesResponse {
    /// Wallet address
    pub wallet: String,
    /// List of token balances
    pub balances: Vec<TokenBalanceResponse>,
    /// Total count
    pub total_count: usize,
}

/// Get balance of a specific token for a wallet
pub async fn get_token_balance(
    State(_state): State<ApiState>,
    Path((mint, wallet)): Path<(String, String)>,
) -> impl IntoResponse {
    info!("Token balance request: mint={}, wallet={}", mint, wallet);
    
    // Parse addresses
    let mint_pubkey = match Pubkey::from_str(&mint) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid mint address"
            }))).into_response();
        }
    };
    
    let wallet_pubkey = match Pubkey::from_str(&wallet) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid wallet address"
            }))).into_response();
        }
    };
    
    // Create RPC client
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8899".to_string());
    let client = LightRpcClient::new(rpc_url);
    
    // Get associated token account
    let ata = spl_associated_token_account::get_associated_token_address(
        &wallet_pubkey,
        &mint_pubkey
    );
    
    // Fetch account data
    match client.get_account(&ata).await {
        Ok(account) => {
            match parse_token_account(&account) {
                Ok(token_account) => {
                    // Get token info (mock for now)
                    let (symbol, decimals) = get_token_info(&mint_pubkey);
                    
                    let balance = token_account.amount;
                    let ui_balance = balance as f64 / 10_f64.powi(decimals as i32);
                    
                    let response = TokenBalanceResponse {
                        mint: mint_pubkey.to_string(),
                        symbol: Some(symbol),
                        balance: balance.to_string(),
                        ui_balance,
                        decimals,
                    };
                    
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    error!("Failed to parse token account: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                        "error": "Failed to parse token account"
                    }))).into_response()
                }
            }
        }
        Err(_) => {
            // Account doesn't exist - return zero balance
            let (symbol, decimals) = get_token_info(&mint_pubkey);
            
            let response = TokenBalanceResponse {
                mint: mint_pubkey.to_string(),
                symbol: Some(symbol),
                balance: "0".to_string(),
                ui_balance: 0.0,
                decimals,
            };
            
            (StatusCode::OK, Json(response)).into_response()
        }
    }
}

/// Get all token balances for a wallet
pub async fn get_wallet_balances(
    State(_state): State<ApiState>,
    Path(wallet): Path<String>,
) -> impl IntoResponse {
    info!("Wallet balances request: wallet={}", wallet);
    
    // Parse wallet address
    let wallet_pubkey = match Pubkey::from_str(&wallet) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid wallet address"
            }))).into_response();
        }
    };
    
    // Create RPC client
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8899".to_string());
    let client = LightRpcClient::new(rpc_url);
    
    // Get all token accounts for wallet
    match client.get_token_accounts_by_owner(
        &wallet_pubkey,
        None, // Get all token accounts, not filtered by mint
    ).await {
        Ok(accounts) => {
            let mut balances = Vec::new();
            
            for (pubkey, account) in accounts {
                if let Ok(token_account) = parse_token_account(&account) {
                    if token_account.amount > 0 {
                        let mint = token_account.mint;
                        let (symbol, decimals) = get_token_info(&mint);
                        let ui_balance = token_account.amount as f64 / 10_f64.powi(decimals as i32);
                        
                        balances.push(TokenBalanceResponse {
                            mint: mint.to_string(),
                            symbol: Some(symbol),
                            balance: token_account.amount.to_string(),
                            ui_balance,
                            decimals,
                        });
                    }
                }
            }
            
            // Sort by UI balance descending
            balances.sort_by(|a, b| b.ui_balance.partial_cmp(&a.ui_balance).unwrap_or(std::cmp::Ordering::Equal));
            
            let response = TokenBalancesResponse {
                wallet: wallet_pubkey.to_string(),
                total_count: balances.len(),
                balances,
            };
            
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            error!("Failed to fetch token accounts: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to fetch token accounts"
            }))).into_response()
        }
    }
}

// Helper functions

fn parse_token_account(account: &Account) -> Result<TokenAccount, Box<dyn std::error::Error>> {
    use solana_program::program_pack::Pack;
    TokenAccount::unpack(&account.data).map_err(|e| e.into())
}

fn get_token_info(mint: &Pubkey) -> (String, u8) {
    // Mock token info - in production would query token registry
    match mint.to_string().as_str() {
        "So11111111111111111111111111111111111111112" => ("SOL".to_string(), 9),
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => ("USDC".to_string(), 6),
        "FEELsso1VoSkqwJQsYq3h3mBGsVZcKbXgssbKdZrmMad" => ("FeelsSOL".to_string(), 9),
        "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn" => ("JitoSOL".to_string(), 9),
        _ => ("Unknown".to_string(), 9), // Default to 9 decimals
    }
}