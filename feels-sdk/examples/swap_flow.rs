//! Complete swap flow example
//!
//! Demonstrates a full swap execution from start to finish

use feels_sdk::FeelsClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = FeelsClient::new("https://api.devnet.solana.com").await?;

    // Load user keypair (in production, load from file)
    let user = Keypair::new();

    // Token addresses
    let token_a: Pubkey = "TokenA11111111111111111111111111111111111".parse()?;
    let token_b: Pubkey = "TokenB22222222222222222222222222222222222".parse()?;

    // User token accounts
    let user_token_a: Pubkey = "UserTokenA333333333333333333333333333333".parse()?;
    let user_token_b: Pubkey = "UserTokenB444444444444444444444444444444".parse()?;

    println!("Executing swap from {} to {}", token_a, token_b);

    // Step 1: Find the market
    let market_info = match client.market.get_market_by_tokens(&token_a, &token_b).await {
        Ok(info) => info,
        Err(_) => {
            println!("Market not found. Need to create one first.");
            // In production, you might initialize the market here
            return Ok(());
        }
    };

    println!("Using market: {}", market_info.address);
    println!("  Base fee: {} bps", market_info.base_fee_bps);
    println!("  Current tick: {}", market_info.current_tick);

    // Step 2: Simulate the swap
    let amount_in = 1_000_000; // 1 USDC (assuming 6 decimals)
    let simulation = client
        .swap
        .simulate_swap(market_info.address, amount_in, true)
        .await?;

    println!("\nSimulation results:");
    println!("  Input: {}", simulation.amount_in);
    println!("  Output: {}", simulation.amount_out);
    println!("  Fee: {}", simulation.fee_paid);
    println!("  Ticks crossed: {}", simulation.ticks_crossed);

    // Step 3: Execute the swap with slippage tolerance
    let min_amount_out = (simulation.amount_out as f64 * 0.99) as u64; // 1% slippage

    println!("\nExecuting swap...");
    match client
        .swap
        .swap_exact_in(
            &user,
            market_info.address,
            user_token_a,
            user_token_b,
            amount_in,
            min_amount_out,
            Some(100), // 1% max slippage in bps
        )
        .await
    {
        Ok(result) => {
            println!("Swap successful!");
            println!("  Transaction: {}", result.signature);
            println!("  Amount in: {}", result.amount_in);
            println!("  Amount out: {}", result.amount_out_estimate);
            println!("  Fee paid: {}", result.fee_paid_estimate);
        }
        Err(e) => {
            println!("Swap failed: {}", e);
        }
    }

    Ok(())
}
