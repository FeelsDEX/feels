//! Complete flow example for Feels Protocol SDK
//! 
//! This example demonstrates:
//! 1. Setting up the SDK client
//! 2. Entering FeelsSOL (depositing JitoSOL)
//! 3. Using the hub-constrained router
//! 4. Performing swaps
//! 5. Managing liquidity positions

use feels_sdk::{
    FeelsClient, SdkConfig, HubRouter, PoolInfo,
    find_market_address, find_buffer_address,
    sort_tokens, derive_pool,
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Feels Protocol SDK Complete Flow Example ===\n");

    // 1. Setup
    let payer = Keypair::new();
    let payer_pubkey = payer.pubkey();
    println!("Created payer: {}", payer_pubkey);
    
    // Configure SDK for localnet
    let config = SdkConfig::localnet(payer)
        .with_commitment("confirmed".to_string());
    
    println!("SDK Configuration:");
    println!("  RPC URL: {}", config.rpc_url);
    println!("  Program ID: {}", config.program_id);
    println!("  Commitment: {}", config.commitment);
    
    // Create client
    let client = FeelsClient::new(config)?;
    println!("\nClient initialized successfully!");

    // 2. Token Setup (using example pubkeys)
    let jitosol_mint = Pubkey::from_str("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn")?;
    let feelssol_mint = Pubkey::from_str("FEELSjMmBW8cB9SsoNXdQiKtFYbNVUe2tTEKKZmu6E1")?;
    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;
    let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
    
    println!("\nToken Configuration:");
    println!("  JitoSOL: {}", jitosol_mint);
    println!("  FeelsSOL: {}", feelssol_mint);
    println!("  USDC: {}", usdc_mint);
    println!("  SOL: {}", sol_mint);

    // 3. PDA Derivation Examples
    println!("\nPDA Derivation Examples:");
    
    // Sort tokens for consistent ordering
    let (token_0, token_1) = sort_tokens(usdc_mint, feelssol_mint);
    println!("  Sorted tokens: {} < {}", token_0, token_1);
    
    // Derive market address
    let (market_address, market_bump) = find_market_address(&token_0, &token_1);
    println!("  Market PDA: {} (bump: {})", market_address, market_bump);
    
    // Derive buffer address
    let (buffer_address, buffer_bump) = find_buffer_address(&market_address);
    println!("  Buffer PDA: {} (bump: {})", buffer_address, buffer_bump);
    
    // Derive pool using fee rate
    let fee_rate = 30u16; // 0.3%
    let (pool_address, pool_bump) = derive_pool(&token_0, &token_1, fee_rate, &client.config.program_id);
    println!("  Pool PDA: {} (bump: {}) @ {}bps", pool_address, pool_bump, fee_rate);

    // 4. Hub-Constrained Router Setup
    println!("\nSetting up Hub-Constrained Router:");
    let mut router = HubRouter::new(feelssol_mint);
    
    // Add pools (all must include FeelsSOL as hub)
    let pools = vec![
        PoolInfo {
            address: Pubkey::new_unique(),
            token_0: usdc_mint,
            token_1: feelssol_mint,
            fee_rate: 30, // 0.3%
        },
        PoolInfo {
            address: Pubkey::new_unique(),
            token_0: sol_mint,
            token_1: feelssol_mint,
            fee_rate: 25, // 0.25%
        },
        PoolInfo {
            address: Pubkey::new_unique(),
            token_0: jitosol_mint,
            token_1: feelssol_mint,
            fee_rate: 10, // 0.1% for entry/exit
        },
    ];
    
    for pool in pools {
        router.add_pool(pool.clone())?;
        println!("  Added pool: {} <-> {} @ {}bps", 
            pool.token_0, pool.token_1, pool.fee_rate);
    }

    // 5. Route Finding Examples
    println!("\nRoute Finding Examples:");
    
    // Example 1: Direct route (USDC -> FeelsSOL)
    let route1 = router.find_route(&usdc_mint, &feelssol_mint)?;
    println!("\n  USDC → FeelsSOL:");
    println!("    Route: {}", router.get_route_summary(&route1));
    println!("    Hops: {}", route1.hop_count());
    println!("    Total fee: {}bps", router.calculate_route_fee(&route1));
    
    // Example 2: Two-hop route (USDC -> SOL via FeelsSOL)
    let route2 = router.find_route(&usdc_mint, &sol_mint)?;
    println!("\n  USDC → SOL:");
    println!("    Route: {}", router.get_route_summary(&route2));
    println!("    Hops: {}", route2.hop_count());
    println!("    Total fee: {}bps", router.calculate_route_fee(&route2));
    
    // Example 3: Entry route (JitoSOL -> FeelsSOL)
    let route3 = router.find_route(&jitosol_mint, &feelssol_mint)?;
    println!("\n  JitoSOL → FeelsSOL (Entry):");
    println!("    Route: {}", router.get_route_summary(&route3));
    println!("    Hops: {}", route3.hop_count());
    println!("    Total fee: {}bps", router.calculate_route_fee(&route3));

    // 6. Transaction Examples (would execute on real cluster)
    println!("\nTransaction Examples (dry run):");
    
    // Example: Enter FeelsSOL
    println!("\n  Enter FeelsSOL:");
    println!("    Amount: 1.0 JitoSOL");
    println!("    Expected: 1.0 FeelsSOL (1:1 backing)");
    
    // In real usage:
    // let sig = client.enter_feelssol(
    //     &user_jitosol_account,
    //     &user_feelssol_account,
    //     &jitosol_mint,
    //     &feelssol_mint,
    //     1_000_000_000, // 1 JitoSOL (9 decimals)
    // ).await?;
    // println!("    Transaction: {}", sig);
    
    // Example: Quote swap
    println!("\n  Quote Swap:");
    let quote = client.quote_swap(&usdc_mint, &sol_mint, 1_000_000)?;
    println!("    Input: {} USDC", quote.amount_in as f64 / 1_000_000.0);
    println!("    Output: {} SOL", quote.amount_out as f64 / 1_000_000_000.0);
    println!("    Fee: {} USDC ({}bps)", 
        quote.fee_amount as f64 / 1_000_000.0, 
        quote.fee_bps
    );
    println!("    Route: {} hop(s)", quote.route.hop_count());

    // 7. Market Operations (would require initialized market)
    println!("\nMarket Operations:");
    
    // Example: Get market info
    println!("  Get Market Info:");
    // let market_info = client.get_market_info(&market_address)?;
    println!("    Market: {}", market_address);
    println!("    Status: Would fetch actual market data");
    
    // Example: Get buffer info
    println!("\n  Get Buffer Info:");
    // let buffer_info = client.get_buffer_info(&buffer_address)?;
    println!("    Buffer: {}", buffer_address);
    println!("    Status: Would fetch actual buffer data");

    // 8. Advanced Features
    println!("\nAdvanced Features:");
    
    // Calculate required tick arrays for swap
    println!("  Tick Arrays for Swap:");
    println!("    For concentrated liquidity, would derive tick arrays");
    println!("    based on current price and swap direction");
    
    // Position management
    println!("\n  Position Management:");
    println!("    Open Position: Provide liquidity in tick range");
    println!("    Close Position: Remove liquidity and collect fees");
    println!("    Collect Fees: Harvest accumulated trading fees");

    println!("\nExample completed successfully!");
    println!("\nNote: This is a dry run. On a real cluster, transactions would be submitted.");

    Ok(())
}

// Helper function to demonstrate error handling
#[allow(dead_code)]
fn demonstrate_error_handling() {
    println!("\nError Handling Examples:");
    
    let feelssol_mint = Pubkey::new_unique();
    let mut router = HubRouter::new(feelssol_mint);
    
    // Try to add invalid pool (no hub token)
    let invalid_pool = PoolInfo {
        address: Pubkey::new_unique(),
        token_0: Pubkey::new_unique(),
        token_1: Pubkey::new_unique(),
        fee_rate: 30,
    };
    
    match router.add_pool(invalid_pool) {
        Err(e) => println!("  Expected error: {}", e),
        Ok(_) => println!("  Unexpected success!"),
    }
    
    // Try to route same token
    let token = Pubkey::new_unique();
    match router.find_route(&token, &token) {
        Err(e) => println!("  Expected error: {}", e),
        Ok(_) => println!("  Unexpected success!"),
    }
}