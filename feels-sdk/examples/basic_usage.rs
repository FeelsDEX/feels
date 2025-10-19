//! Basic usage example for the Feels Protocol SDK
//!
//! This example demonstrates:
//! - Creating a client
//! - Finding markets
//! - Executing swaps
//! - Managing liquidity

use feels_sdk::{FeelsClient, Route};
use solana_sdk::signature::{Keypair, Signer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = FeelsClient::new("https://api.devnet.solana.com").await?;

    // Create a test keypair (in production, load from file)
    let user = Keypair::new();
    println!("User pubkey: {}", user.pubkey());

    // Example token pubkeys (replace with actual tokens)
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".parse()?;
    let custom_token = "11111111111111111111111111111111".parse()?;

    // Get market info
    match client
        .market
        .get_market_by_tokens(&usdc_mint, &custom_token)
        .await
    {
        Ok(market) => {
            println!("Found market: {}", market.address);
            println!("  Token 0: {}", market.token_0);
            println!("  Token 1: {}", market.token_1);
            println!("  Current price: {}", market.sqrt_price);
            println!("  Liquidity: {}", market.liquidity);
        }
        Err(e) => {
            println!("Market not found: {}", e);
        }
    }

    // Find route between tokens
    let route = client.swap.find_route(&usdc_mint, &custom_token).await?;
    match route {
        Route::Direct { from, to } => {
            println!("Direct route: {} -> {}", from, to);
        }
        Route::TwoHop {
            from,
            intermediate,
            to,
        } => {
            println!("Two-hop route: {} -> {} -> {}", from, intermediate, to);
        }
    }

    // Estimate swap fees
    let market_address = "YourMarketAddressHere".parse()?;
    match client.swap.estimate_fees(&market_address, 1_000_000).await {
        Ok(fees) => {
            println!("Estimated fees:");
            println!("  Base fee: {}", fees.base_fee);
            println!("  Impact fee: {}", fees.impact_fee);
            println!("  Total fee: {} ({} bps)", fees.total_fee, fees.fee_bps);
        }
        Err(e) => {
            println!("Failed to estimate fees: {}", e);
        }
    }

    // Simulate a swap (without executing)
    match client
        .swap
        .simulate_swap(market_address, 1_000_000, true)
        .await
    {
        Ok(simulation) => {
            println!("Swap simulation:");
            println!("  Amount in: {}", simulation.amount_in);
            println!("  Amount out: {}", simulation.amount_out);
            println!("  Fee paid: {}", simulation.fee_paid);
            println!("  End price: {}", simulation.end_sqrt_price);
        }
        Err(e) => {
            println!("Simulation failed: {}", e);
        }
    }

    println!("\nSDK Version: {}", feels_sdk::VERSION);

    Ok(())
}
