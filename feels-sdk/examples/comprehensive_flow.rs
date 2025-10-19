use anchor_lang::prelude::*;
use feels_sdk::FeelsClient;
use solana_sdk::signature::Keypair;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = FeelsClient::new("https://api.devnet.solana.com").await?;

    // Example: Protocol initialization (one-time setup)
    let _authority = Keypair::new();
    let _treasury = Keypair::new();

    // Note: In production, these would be actual protocol operations
    println!("Protocol initialization would happen here");

    // Example: Initialize FeelsSOL hub
    let _jitosol_mint = Pubkey::new_unique(); // Would be actual JitoSOL mint
    println!("Hub initialization would happen here");

    // Example: Initialize a market
    let feelssol_mint = Pubkey::new_unique(); // Would be FeelsSOL mint
    let other_token = Pubkey::new_unique();
    let deployer = Keypair::new();

    // Market initialization
    let market_result = client
        .liquidity
        .initialize_market(
            &deployer,
            feelssol_mint,                     // token_0 must be FeelsSOL
            other_token,                       // token_1
            30,                                // base_fee_bps
            10,                                // tick_spacing
            79228162514264337593543950336u128, // initial_sqrt_price (1:1)
            1000000000,                        // initial_buy_feelssol_amount
        )
        .await?;

    println!("Market initialized: {:?}", market_result.market);

    // Example: Enter FeelsSOL (wrap JitoSOL)
    let user = Keypair::new();
    let user_jitosol = Pubkey::new_unique();
    let user_feelssol = Pubkey::new_unique();

    let enter_sig = client
        .liquidity
        .enter_feelssol(
            &user,
            user_jitosol,
            user_feelssol,
            1000000000, // 1 JitoSOL worth
        )
        .await?;

    println!("Enter FeelsSOL transaction: {:?}", enter_sig);

    // Example: Perform a swap using SwapService
    let swap_result = client
        .swap
        .swap_exact_in(
            &user,
            market_result.market,
            user_feelssol,
            other_token,
            1000000,   // amount_in
            900000,    // minimum_amount_out
            Some(100), // max_slippage_bps (1%)
        )
        .await?;

    println!("Swap executed: {:?}", swap_result.signature);

    // Example: Open a liquidity position
    let position_result = client
        .liquidity
        .open_position(
            &user,
            market_result.market,
            -1000,          // tick_lower
            1000,           // tick_upper
            1000000000u128, // liquidity
        )
        .await?;

    println!("Position opened: {:?}", position_result.position);

    // Example: Exit FeelsSOL (unwrap to JitoSOL)
    let exit_sig = client
        .liquidity
        .exit_feelssol(
            &user,
            user_jitosol,
            user_feelssol,
            500000000, // 0.5 JitoSOL worth
        )
        .await?;

    println!("Exit FeelsSOL transaction: {:?}", exit_sig);

    println!("\nAll examples completed successfully!");
    println!("SDK services demonstrated:");
    println!("- Market operations");
    println!("- Swap execution");
    println!("- Liquidity management");
    println!("- Enter/Exit FeelsSOL");

    Ok(())
}
