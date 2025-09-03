use feels_sdk::{utils, FeelsClient, SdkConfig};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Feels Protocol SDK Basic Usage Example");
    println!("=====================================\n");

    // Create a test keypair for the payer
    let payer = Keypair::new();
    println!("Created payer: {}", payer.pubkey());

    // Initialize SDK configuration for localnet
    let config = SdkConfig::localnet(payer);
    println!("\nSDK Configuration:");
    println!("  RPC URL: {}", config.rpc_url);
    println!("  Program ID: {}", config.program_id);

    // Create client instance
    let _client = FeelsClient::new(config.clone());
    println!("\nClient initialized successfully!");

    // Demonstrate PDA derivation utilities
    println!("\n=== PDA Derivation Examples ===");

    // Example token public keys
    let token_0 = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?;
    let token_1 = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
    let fee_rate = 30u16; // 0.3% fee tier

    // Sort tokens for consistent derivation
    let (sorted_token_0, sorted_token_1) = utils::sort_tokens(token_0, token_1);

    // Derive pool address
    let (pool_address, _bump) = utils::derive_pool(
        &sorted_token_0,
        &sorted_token_1,
        fee_rate,
        &config.program_id,
    );
    println!("\nPool Derivation:");
    println!("  Token 0: {}", sorted_token_0);
    println!("  Token 1: {}", sorted_token_1);
    println!(
        "  Fee Rate: {} bps ({}%)",
        fee_rate,
        fee_rate as f64 / 100.0
    );
    println!("  Pool Address: {}", pool_address);

    // Derive vault addresses for the pool
    let (vault_0, _) = utils::derive_vault(&pool_address, &sorted_token_0, &config.program_id);
    let (vault_1, _) = utils::derive_vault(&pool_address, &sorted_token_1, &config.program_id);
    println!("\nVault Addresses:");
    println!("  Vault 0: {}", vault_0);
    println!("  Vault 1: {}", vault_1);

    // Demonstrate tick array derivation
    let tick_spacing = 60; // For 0.3% fee tier
    let tick = -887220; // Example tick
    let start_tick_index = utils::get_tick_array_start_index(tick, tick_spacing);
    let (tick_array, _) =
        utils::derive_tick_array(&pool_address, start_tick_index, &config.program_id);
    println!("\nTick Array:");
    println!("  Tick: {}", tick);
    println!("  Start Tick Index: {}", start_tick_index);
    println!("  Tick Array Address: {}", tick_array);

    // Show protocol addresses
    let (protocol_state, _) = utils::derive_protocol_state(&config.program_id);
    let (feelssol_state, _) = utils::derive_feelssol_state(&config.program_id);
    println!("\nProtocol Addresses:");
    println!("  Protocol State: {}", protocol_state);
    println!("  FeelsSOL State: {}", feelssol_state);

    // Demonstrate fee tier information
    println!("\n=== Fee Tiers and Tick Spacing ===");
    let fee_tiers = vec![
        (1, "0.01%", 1),
        (5, "0.05%", 10),
        (30, "0.3%", 60),
        (100, "1.0%", 200),
    ];

    for (fee_bps, percentage, tick_spacing) in fee_tiers {
        println!(
            "  {} bps ({}) → tick spacing = {}",
            fee_bps, percentage, tick_spacing
        );
    }

    // Show example sqrt price calculations
    println!("\n=== Price Calculations ===");
    println!("Sqrt Price Examples (Q96 format):");
    println!(
        "  Price 1:1   → sqrt_price_x96 = {}",
        79228162514264337593543950336u128
    );
    println!(
        "  Price 2:1   → sqrt_price_x96 = {}",
        112045541949572279837463876454u128
    );
    println!(
        "  Price 1:2   → sqrt_price_x96 = {}",
        56022770974786139918731938227u128
    );

    // Demonstrate tick calculations
    println!("\nTick Examples:");
    println!("  Tick 0      → Price = 1.0");
    println!("  Tick 6932   → Price ≈ 2.0");
    println!("  Tick -6932  → Price ≈ 0.5");
    println!("  Tick 23028  → Price ≈ 10.0");
    println!("  Tick -23028 → Price ≈ 0.1");

    println!("\n=== Usage Instructions ===");
    println!(
        "
To use the SDK in a real application:

1. Initialize the protocol (one-time setup):
   // Would call client.initialize_protocol(&authority, &emergency_authority).await?;

2. Create a pool:
   // Would call client.create_pool(&token_0, &token_1, 30, sqrt_price_x96).await?;

3. Add liquidity:
   // Would call client.add_liquidity(&pool, &position_mint, amounts...).await?;

4. Execute a swap:
   // Would call client.swap(&pool, amount_in, amount_out_min, ...).await?;

Note: These operations would require a funded account and connection to a Solana cluster.
The current version focuses on utility functions and PDA derivation.
"
    );

    println!("\nExample completed successfully!");

    Ok(())
}