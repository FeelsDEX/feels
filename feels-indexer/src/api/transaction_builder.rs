//! Transaction building for swaps
//!
//! Provides endpoints to build swap transactions ready for signing and execution

use super::ApiState;
use axum::{
    extract::State,
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    transaction::Transaction,
    message::Message,
    system_program,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::str::FromStr;
use tracing::{info, error};
use base64::Engine;

/// Request to build a swap transaction
#[derive(Debug, Deserialize)]
pub struct BuildSwapTransactionRequest {
    /// User's wallet address
    pub wallet: String,
    /// Market address to swap on
    pub market_address: String,
    /// Amount of input tokens
    pub amount_in: String,
    /// Minimum amount of output tokens (with slippage)
    pub min_amount_out: String,
    /// Whether swapping token0 to token1
    pub is_token_0_to_1: bool,
    /// User's token account for input token
    pub user_token_in: String,
    /// User's token account for output token
    pub user_token_out: String,
    /// Optional referrer for fees
    pub referrer: Option<String>,
    /// Priority fee in microlamports
    pub priority_fee_microlamports: Option<u64>,
}

/// Response with built transaction
#[derive(Debug, Serialize)]
pub struct BuildSwapTransactionResponse {
    /// Base64 encoded transaction ready for signing
    pub transaction: String,
    /// Estimated compute units
    pub compute_units: u32,
    /// Priority fee included
    pub priority_fee: u64,
    /// Transaction expiry (slot)
    pub expires_at: u64,
    /// Instructions included in transaction
    pub instructions_summary: Vec<String>,
    /// Accounts that need to be signed
    pub signers: Vec<String>,
}

/// Build a swap transaction
pub async fn build_swap_transaction(
    State(state): State<ApiState>,
    Json(params): Json<BuildSwapTransactionRequest>,
) -> impl IntoResponse {
    info!("Build swap transaction request: {:?}", params);
    
    // Parse addresses
    let wallet = match Pubkey::from_str(&params.wallet) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid wallet address"
            }))).into_response();
        }
    };
    
    let market_address = match Pubkey::from_str(&params.market_address) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid market address"
            }))).into_response();
        }
    };
    
    let user_token_in = match Pubkey::from_str(&params.user_token_in) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid user_token_in address"
            }))).into_response();
        }
    };
    
    let user_token_out = match Pubkey::from_str(&params.user_token_out) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid user_token_out address"
            }))).into_response();
        }
    };
    
    // Parse amounts
    let amount_in: u64 = match params.amount_in.parse() {
        Ok(amt) => amt,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid amount_in"
            }))).into_response();
        }
    };
    
    let min_amount_out: u64 = match params.min_amount_out.parse() {
        Ok(amt) => amt,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid min_amount_out"
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
    
    // Create RPC client
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8899".to_string());
    let client = RpcClient::new(rpc_url);
    
    // Get recent blockhash
    let recent_blockhash = match client.get_latest_blockhash().await {
        Ok(blockhash) => blockhash,
        Err(e) => {
            error!("Failed to get blockhash: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get recent blockhash"
            }))).into_response();
        }
    };
    
    // Build swap instruction
    let swap_instruction = match build_swap_instruction(
        &market,
        &wallet,
        amount_in,
        min_amount_out,
        params.is_token_0_to_1,
        &user_token_in,
        &user_token_out,
    ).await {
        Ok(ix) => ix,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to build swap instruction: {}", e)
            }))).into_response();
        }
    };
    
    // Add compute budget instruction
    let compute_units = 400_000u32; // Estimated for swap
    let priority_fee = params.priority_fee_microlamports.unwrap_or(5_000);
    
    let compute_budget_ix = solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(compute_units);
    let priority_fee_ix = solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(priority_fee);
    
    // Build transaction
    let message = Message::new(
        &[compute_budget_ix, priority_fee_ix, swap_instruction],
        Some(&wallet),
    );
    
    let mut transaction = Transaction::new_unsigned(message);
    transaction.message.recent_blockhash = recent_blockhash;
    
    // Serialize transaction
    let serialized = bincode::serialize(&transaction)
        .map_err(|e| {
            error!("Failed to serialize transaction: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to serialize transaction"
            })))
        })?;
    
    let base64_tx = base64::engine::general_purpose::STANDARD.encode(&serialized);
    
    // Get current slot for expiry
    let current_slot = match client.get_slot().await {
        Ok(slot) => slot,
        Err(_) => 0,
    };
    
    let response = BuildSwapTransactionResponse {
        transaction: base64_tx,
        compute_units,
        priority_fee,
        expires_at: current_slot + 150, // ~1 minute expiry
        instructions_summary: vec![
            "Set compute budget".to_string(),
            "Set priority fee".to_string(),
            format!("Swap {} for min {} on market {}", amount_in, min_amount_out, market_address),
        ],
        signers: vec![wallet.to_string()],
    };
    
    (StatusCode::OK, Json(response)).into_response()
}

/// Build swap instruction
async fn build_swap_instruction(
    market: &crate::database::Market,
    user: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    is_token_0_to_1: bool,
    user_token_in: &Pubkey,
    user_token_out: &Pubkey,
) -> Result<Instruction, String> {
    // Parse market data
    let market_pubkey = Pubkey::from_str(&market.address)
        .map_err(|_| "Invalid market address")?;
    let token_0 = Pubkey::from_str(&market.token_0)
        .map_err(|_| "Invalid token_0")?;
    let token_1 = Pubkey::from_str(&market.token_1)
        .map_err(|_| "Invalid token_1")?;
    
    // Derive PDAs
    // Use the Feels program ID from environment or default
    let program_id = Pubkey::from_str("FEELs1FW9tXEKPxMECvKhgxCcDQ9Q3pYd44piyHUxJbV")
        .unwrap_or_else(|_| Pubkey::from_str("11111111111111111111111111111112").unwrap());
    let (market_authority, _) = Pubkey::find_program_address(
        &[b"market_authority", market_pubkey.as_ref()],
        &program_id,
    );
    
    let (vault_0, _) = Pubkey::find_program_address(
        &[b"vault", market_pubkey.as_ref(), token_0.as_ref()],
        &program_id,
    );
    
    let (vault_1, _) = Pubkey::find_program_address(
        &[b"vault", market_pubkey.as_ref(), token_1.as_ref()],
        &program_id,
    );
    
    let (oracle, _) = Pubkey::find_program_address(
        &[b"oracle", market_pubkey.as_ref()],
        &program_id,
    );
    
    let (buffer, _) = Pubkey::find_program_address(
        &[b"buffer", market_pubkey.as_ref()],
        &program_id,
    );
    
    let (protocol_config, _) = Pubkey::find_program_address(
        &[b"protocol_config"],
        &program_id,
    );
    
    // Treasury would need to be fetched from protocol_config
    // For now, use a mock treasury
    let treasury = derive_mock_treasury_ata(if is_token_0_to_1 { &token_1 } else { &token_0 });
    
    // Build instruction data
    let ix_data = build_swap_instruction_data(amount_in, min_amount_out)?;
    
    // Build accounts
    let accounts = vec![
        AccountMeta::new(*user, true), // user (signer)
        AccountMeta::new(market_pubkey, false), // market
        AccountMeta::new(vault_0, false), // vault_0
        AccountMeta::new(vault_1, false), // vault_1
        AccountMeta::new_readonly(market_authority, false), // market_authority
        AccountMeta::new(buffer, false), // buffer
        AccountMeta::new(oracle, false), // oracle
        AccountMeta::new(*user_token_in, false), // user_token_in
        AccountMeta::new(*user_token_out, false), // user_token_out
        AccountMeta::new_readonly(spl_token::id(), false), // token_program
        AccountMeta::new_readonly(system_program::id(), false), // system_program
        AccountMeta::new_readonly(protocol_config, false), // protocol_config
        AccountMeta::new(treasury, false), // protocol_treasury
        AccountMeta::new_readonly(Pubkey::default(), false), // protocol_token (optional)
        AccountMeta::new_readonly(Pubkey::default(), false), // creator_token_account (optional)
        AccountMeta::new_readonly(spl_associated_token_account::id(), false), // associated_token_program
    ];
    
    Ok(Instruction {
        program_id,
        accounts,
        data: ix_data,
    })
}

/// Build swap instruction data
fn build_swap_instruction_data(amount_in: u64, min_amount_out: u64) -> Result<Vec<u8>, String> {
    // Swap instruction discriminator
    // This would need to match the actual instruction discriminator
    let mut data = vec![248, 198, 158, 145, 225, 117, 135, 200]; // Mock discriminator
    
    // Add amount_in (u64 little-endian)
    data.extend_from_slice(&amount_in.to_le_bytes());
    
    // Add min_amount_out (u64 little-endian)
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    
    Ok(data)
}

/// Derive mock treasury ATA for testing
fn derive_mock_treasury_ata(mint: &Pubkey) -> Pubkey {
    // In production, this would fetch the actual treasury from protocol_config
    let mock_treasury = Pubkey::from_str("FEELtreasury111111111111111111111111111111").unwrap();
    spl_associated_token_account::get_associated_token_address(&mock_treasury, mint)
}

/// Request to simulate a transaction
#[derive(Debug, Deserialize)]
pub struct SimulateTransactionRequest {
    /// Base64 encoded transaction
    pub transaction: String,
    /// Whether to include logs in response
    pub include_logs: Option<bool>,
}

/// Response from transaction simulation
#[derive(Debug, Serialize)]
pub struct SimulateTransactionResponse {
    /// Whether simulation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Logs from simulation
    pub logs: Option<Vec<String>>,
    /// Compute units consumed
    pub units_consumed: Option<u64>,
    /// Accounts accessed
    pub accounts: Vec<String>,
}

/// Simulate a transaction before execution
pub async fn simulate_transaction(
    State(_state): State<ApiState>,
    Json(params): Json<SimulateTransactionRequest>,
) -> impl IntoResponse {
    info!("Simulate transaction request");
    
    // Decode transaction
    let tx_bytes = match base64::engine::general_purpose::STANDARD.decode(&params.transaction) {
        Ok(bytes) => bytes,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid base64 transaction"
            }))).into_response();
        }
    };
    
    let transaction: Transaction = match bincode::deserialize(&tx_bytes) {
        Ok(tx) => tx,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid transaction format"
            }))).into_response();
        }
    };
    
    // Create RPC client
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8899".to_string());
    let client = RpcClient::new(rpc_url);
    
    // Simulate transaction
    match client.simulate_transaction(&transaction).await {
        Ok(result) => {
            let response = SimulateTransactionResponse {
                success: result.value.err.is_none(),
                error: result.value.err.map(|e| format!("{:?}", e)),
                logs: if params.include_logs.unwrap_or(false) {
                    result.value.logs
                } else {
                    None
                },
                units_consumed: result.value.units_consumed,
                accounts: result.value.accounts.unwrap_or_default()
                    .iter()
                    .filter_map(|a| a.as_ref().map(|acc| acc.owner.to_string()))
                    .collect(),
            };
            
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            error!("Failed to simulate transaction: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to simulate transaction"
            }))).into_response()
        }
    }
}