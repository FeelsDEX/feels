//! Example: Complete token launch flow
//!
//! This example demonstrates the full process of:
//! 1. Minting a new protocol token
//! 2. Initializing a market with the token
//! 3. Deploying liquidity using the stair pattern
//! 4. Performing the initial buy (if requested)

use anchor_lang::prelude::*;
use feels::{
    instructions::*,
    state::*,
};

/// Complete token launch flow
pub async fn launch_token_with_stair_pattern() {
    // Step 1: Mint a new protocol token
    // All 1 billion tokens go to the protocol buffer
    let mint_params = MintTokenParams {
        token_name: "Feels Test Token".to_string(),
        token_symbol: "FTEST".to_string(),
        token_uri: "https://example.com/token.json".to_string(),
    };
    
    // The mint_token instruction creates:
    // - New SPL token with 1B supply
    // - All tokens sent to buffer
    // - Protocol token registry entry
    // - Token metadata
    
    println!("Token minted successfully!");
    println!("✓ 1,000,000,000 tokens in buffer");
    println!("✓ Creator registered as market launcher");
    
    // Step 2: Initialize market
    // Only the token creator can launch a market for their token
    let initial_sqrt_price = (1u128 << 64) * 10; // Initial price = 10 FeelsSOL per token
    
    let liquidity_commitment = InitialLiquidityCommitment {
        token_0_amount: 0, // Not used - protocol controls deployment
        token_1_amount: 0, // Not used - protocol controls deployment
        deployer: creator_pubkey,
        deploy_by: current_timestamp + 3600, // 1 hour deadline
        position_commitments: vec![], // Protocol will create positions
    };
    
    let market_params = InitializeMarketParams {
        base_fee_bps: 30, // 0.3% fee
        tick_spacing: 10,
        initial_sqrt_price,
        liquidity_commitment,
        initial_buy_feelssol_amount: 0, // No initial buy during initialization
    };
    
    println!("\nMarket initialized!");
    println!("✓ Initial price: 10 FeelsSOL per FTEST");
    println!("✓ Market ready for liquidity deployment");
    
    // Step 3: Deploy protocol liquidity in stair pattern with optional initial buy
    // This uses 80% of the buffer tokens (800M tokens)
    // Creator can include FeelsSOL to be the first buyer at the best price
    let deploy_params = DeployInitialLiquidityParams {
        tick_step_size: 100, // 100 ticks between each step (~1% price intervals)
        initial_buy_feelssol_amount: 100_000_000_000, // Creator buys 100 FeelsSOL worth
    };
    
    println!("\nDeploying stair pattern liquidity:");
    println!("✓ 800,000,000 FTEST tokens (80% of buffer)");
    println!("✓ Corresponding FeelsSOL from buffer");
    println!("✓ Creator includes 100 FeelsSOL for initial buy");
    println!("✓ 10 liquidity positions in escalating pattern:");
    
    // The stair pattern creates 10 positions with declining allocations:
    // Position 1: 20% allocation at current_tick to current_tick+100
    // Position 2: 18% allocation at current_tick+100 to current_tick+200
    // Position 3: 16% allocation at current_tick+200 to current_tick+300
    // ... and so on ...
    // Position 10: 2% allocation (or remaining) at highest range
    
    for i in 0..10 {
        let tick_lower = current_tick + (i * 100);
        let tick_upper = tick_lower + 100;
        let allocation = if i == 9 { "remaining" } else { &format!("{}%", 20 - i * 2) };
        println!("  Position {}: {} allocation, ticks [{}, {}]", i + 1, allocation, tick_lower, tick_upper);
    }
    
    println!("\n✓ Liquidity deployed successfully!");
    println!("✓ Market is now live and tradeable!");
    
    // The initial buy happens as part of the deployment transaction:
    // - Creator's 100 FeelsSOL is swapped against the lowest price liquidity
    // - Creator gets the best price as the first buyer
    println!("\n✓ Initial buy executed atomically:");
    println!("  Creator spent: 100 FeelsSOL");
    println!("  Creator received: ~10 FTEST");
    println!("  Guaranteed best price as first buyer");
    
    // Summary of final state:
    println!("\n=== Final Token Distribution ===");
    println!("Buffer (protocol): 200,000,000 FTEST (20%)");
    println!("In liquidity pools: ~799,999,990 FTEST");
    println!("Creator: ~10 FTEST (from initial buy)");
    println!("Total: 1,000,000,000 FTEST");
}

/// Example without initial buy
pub async fn launch_token_without_initial_buy() {
    // Steps 1-2 are the same (mint token, initialize market)
    
    // Step 3: Deploy liquidity without initial buy
    let deploy_params = DeployInitialLiquidityParams {
        tick_step_size: 100,
        initial_buy_feelssol_amount: 0, // No initial buy
    };
    
    println!("\nDeploying stair pattern liquidity without initial buy:");
    println!("✓ 800,000,000 FTEST tokens deployed");
    println!("✓ Market goes live");
    println!("✓ Anyone can now trade on the market");
}

fn main() {
    println!("=== Feels Protocol Token Launch Example ===\n");
    println!("Token launch process:");
    println!("1. Mint token - All supply goes to protocol buffer");
    println!("2. Initialize market - Creator sets initial price");
    println!("3. Deploy liquidity - Protocol deploys 80% in stair pattern");
    println!("\nOptional: Creator can include FeelsSOL for initial buy");
    println!("         to guarantee best price as first buyer");
}