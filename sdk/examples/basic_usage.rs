//! Basic usage example for Feels Protocol SDK
//!
//! This example demonstrates the core SDK functionality

use feels_sdk::{
    find_buffer_address, find_market_address, sort_tokens, HubRouter, PoolInfo, SdkConfig,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

fn main() {
    println!("=== Feels Protocol SDK Basic Usage ===\n");

    // 1. Create configuration
    let payer = Keypair::new();
    let config = SdkConfig::localnet(payer);

    println!("SDK Configuration:");
    println!("  Network: Localnet");
    println!("  RPC URL: {}", config.rpc_url);
    println!("  Program ID: {}", config.program_id);

    // 2. Initialize client (async in real usage)
    // let client = FeelsClient::new(config)?;
    println!("\nClient configuration ready");

    // 3. PDA Derivation
    println!("\nPDA Examples:");

    let token_0 = Pubkey::new_unique();
    let token_1 = Pubkey::new_unique();

    // Sort tokens
    let (token_0, token_1) = sort_tokens(token_0, token_1);

    // Derive market PDA
    let (market, _) = find_market_address(&token_0, &token_1);
    println!("  Market PDA: {}", market);

    // Derive buffer PDA
    let (buffer, _) = find_buffer_address(&market);
    println!("  Buffer PDA: {}", buffer);

    // 4. Hub Router
    println!("\nHub Router Example:");

    let feelssol = Pubkey::new_unique();
    let mut router = HubRouter::new(feelssol);

    // Add a pool
    let pool = PoolInfo {
        address: Pubkey::new_unique(),
        token_0,
        token_1: feelssol,
        fee_rate: 30, // 0.3%
    };

    match router.add_pool(pool) {
        Ok(_) => println!("  Pool added successfully"),
        Err(e) => println!("  Error: {}", e),
    }

    // Find route
    match router.find_route(&token_0, &feelssol) {
        Ok(route) => {
            println!("  Route found: {} hop(s)", route.hop_count());
        }
        Err(e) => println!("  No route: {}", e),
    }

    println!("\nExample completed!");
}
