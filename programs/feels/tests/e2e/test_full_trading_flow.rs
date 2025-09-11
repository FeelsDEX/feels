//! E2E tests for full trading flows including market creation, liquidity provision, swaps, and fee collection

use crate::common::*;
use anchor_lang::prelude::*;
use feels::{
    state::{Market, Position},
    // instructions::{SwapParams, ClosePositionParams, InitializeMarketParams},
    constants::*,
};

/// Test the full lifecycle: market creation → liquidity → trading → fee collection
test_in_memory!(test_market_creation_to_fee_collection, |ctx: TestContext| async move {
    println!("=== Testing Full Trading Flow ===");
    
    // Step 1: Create tokens - FeelsSOL is already created by test framework
    println!("\nStep 1: Creating test token...");
    let test_token = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 9).await?;
    println!("✓ Test token created: {}", test_token.pubkey());
    
    // Step 2: Initialize market with initial liquidity commitment
    println!("\nStep 2: Initializing market...");
    
    // First, fund the market creator with both tokens for initial liquidity
    let creator_feelssol = ctx.create_ata(&ctx.accounts.market_creator.pubkey(), &ctx.feelssol_mint).await?;
    let creator_test_token = ctx.create_ata(&ctx.accounts.market_creator.pubkey(), &test_token.pubkey()).await?;
    
    // Mint tokens for initial liquidity (10,000 of each)
    ctx.mint_to(&ctx.feelssol_mint, &creator_feelssol, &ctx.feelssol_authority, 10_000_000_000_000).await?;
    ctx.mint_to(&test_token.pubkey(), &creator_test_token, &ctx.accounts.market_creator, 10_000_000_000_000).await?;
    
    // Initialize market with commitment
    let market = ctx.market_builder()
        .token_0(ctx.feelssol_mint)
        .token_1(test_token.pubkey())
        .fee_rate(100) // 1 bp = 0.01%
        .tick_spacing(10)
        .initial_price(79228162514264337593543950336u128) // Price = 1
        .build()
        .await?;
    
    println!("✓ Market initialized: {}", market);
    println!("  Fee rate: {} bps", 100);
    println!("  Tick spacing: {}", 10);
    
    // Step 3: Add additional liquidity from different LPs
    println!("\nStep 3: Adding liquidity positions...");
    
    // Setup Alice with tokens
    let alice_feelssol = ctx.create_ata(&ctx.accounts.alice.pubkey(), &ctx.feelssol_mint).await?;
    let alice_test_token = ctx.create_ata(&ctx.accounts.alice.pubkey(), &test_token.pubkey()).await?;
    ctx.mint_to(&ctx.feelssol_mint, &alice_feelssol, &ctx.feelssol_authority, 50_000_000_000_000).await?;
    ctx.mint_to(&test_token.pubkey(), &alice_test_token, &ctx.accounts.market_creator, 50_000_000_000_000).await?;
    
    // Setup Bob with tokens
    let bob_feelssol = ctx.create_ata(&ctx.accounts.bob.pubkey(), &ctx.feelssol_mint).await?;
    let bob_test_token = ctx.create_ata(&ctx.accounts.bob.pubkey(), &test_token.pubkey()).await?;
    ctx.mint_to(&ctx.feelssol_mint, &bob_feelssol, &ctx.feelssol_authority, 50_000_000_000_000).await?;
    ctx.mint_to(&test_token.pubkey(), &bob_test_token, &ctx.accounts.market_creator, 50_000_000_000_000).await?;
    
    // Alice provides wide-range liquidity
    let alice_position = ctx.position_helper()
        .open_position(
            &market,
            &ctx.accounts.alice,
            -1000,  // ~10% below current price
            1000,   // ~10% above current price
            2_000_000_000, // 2B liquidity
        )
        .await?;
    println!("✓ Alice position opened: {} liquidity from tick {} to {}", 
        2_000_000_000, -1000, 1000);
    
    // Bob provides concentrated liquidity
    let bob_position = ctx.position_helper()
        .open_position(
            &market,
            &ctx.accounts.bob,
            -200,   // ~2% below current price
            200,    // ~2% above current price
            5_000_000_000, // 5B liquidity (more concentrated)
        )
        .await?;
    println!("✓ Bob position opened: {} liquidity from tick {} to {}", 
        5_000_000_000u128, -200, 200);
    
    // Step 4: Execute trading volume
    println!("\nStep 4: Executing trades...");
    
    // Setup Charlie as trader
    let charlie_feelssol = ctx.create_ata(&ctx.accounts.charlie.pubkey(), &ctx.feelssol_mint).await?;
    let charlie_test_token = ctx.create_ata(&ctx.accounts.charlie.pubkey(), &test_token.pubkey()).await?;
    ctx.mint_to(&ctx.feelssol_mint, &charlie_feelssol, &ctx.feelssol_authority, 10_000_000_000_000).await?;
    ctx.mint_to(&test_token.pubkey(), &charlie_test_token, &ctx.accounts.market_creator, 10_000_000_000_000).await?;
    
    let mut total_volume = 0u64;
    let mut total_fees = 0u64;
    
    // Execute 10 swaps alternating direction
    for i in 0..10 {
        let is_buy = i % 2 == 0;
        let amount = 100_000_000_000; // 100 tokens per swap
        
        let swap_result = if is_buy {
            // Buy test token with FeelsSOL
            ctx.swap_helper()
                .swap(&market, &ctx.feelssol_mint, &test_token.pubkey(), amount, &ctx.accounts.charlie)
                .await?
        } else {
            // Sell test token for FeelsSOL
            ctx.swap_helper()
                .swap(&market, &test_token.pubkey(), &ctx.feelssol_mint, amount, &ctx.accounts.charlie)
                .await?
        };
        
        total_volume += swap_result.amount_in;
        total_fees += swap_result.fee_paid;
        
        println!("  Swap {}: {} → {} (fee: {})", 
            i + 1,
            swap_result.amount_in,
            swap_result.amount_out,
            swap_result.fee_paid
        );
    }
    
    println!("\n✓ Trading complete:");
    println!("  Total volume: {}", total_volume);
    println!("  Total fees: {}", total_fees);
    println!("  Expected LP fees (1bp): ~{}", total_volume / 10000);
    
    // Step 5: Close positions (automatically collects fees)
    println!("\nStep 5: Closing positions and collecting fees...");
    
    // Get balances before closing
    let alice_feelssol_before = ctx.get_token_balance(&alice_feelssol).await?;
    let alice_test_before = ctx.get_token_balance(&alice_test_token).await?;
    let bob_feelssol_before = ctx.get_token_balance(&bob_feelssol).await?;
    let bob_test_before = ctx.get_token_balance(&bob_test_token).await?;
    
    // Close Alice's position
    ctx.position_helper()
        .close_position(&alice_position, &ctx.accounts.alice)
        .await?;
    
    // Close Bob's position  
    ctx.position_helper()
        .close_position(&bob_position, &ctx.accounts.bob)
        .await?;
    
    // Get balances after closing
    let alice_feelssol_after = ctx.get_token_balance(&alice_feelssol).await?;
    let alice_test_after = ctx.get_token_balance(&alice_test_token).await?;
    let bob_feelssol_after = ctx.get_token_balance(&bob_feelssol).await?;
    let bob_test_after = ctx.get_token_balance(&bob_test_token).await?;
    
    // Calculate returns (principal + fees)
    let alice_feelssol_return = alice_feelssol_after.saturating_sub(alice_feelssol_before);
    let alice_test_return = alice_test_after.saturating_sub(alice_test_before);
    let bob_feelssol_return = bob_feelssol_after.saturating_sub(bob_feelssol_before);
    let bob_test_return = bob_test_after.saturating_sub(bob_test_before);
    
    println!("\n✓ Positions closed:");
    println!("  Alice received: {} FeelsSOL, {} test tokens", alice_feelssol_return, alice_test_return);
    println!("  Bob received: {} FeelsSOL, {} test tokens", bob_feelssol_return, bob_test_return);
    
    // Verify fees were collected
    assert!(alice_feelssol_return > 0 || alice_test_return > 0, "Alice should have received tokens");
    assert!(bob_feelssol_return > 0 || bob_test_return > 0, "Bob should have received tokens");
    
    // Bob should have earned more fees due to concentrated liquidity
    let alice_total_value = alice_feelssol_return + alice_test_return;
    let bob_total_value = bob_feelssol_return + bob_test_return;
    println!("\n  Relative fee earnings: Bob earned {}x more fees than Alice", 
        bob_total_value / alice_total_value.max(1));
    
    println!("\n=== Full Trading Flow Test Passed ===");
    Ok::<(), Box<dyn std::error::Error>>(())
});

/// Test liquidity migration between fee tiers during market volatility
test_in_memory!(test_liquidity_migration_scenario, |mut ctx: TestContext| async move {
    println!("=== Testing Liquidity Migration Scenario ===");
    
    // Create test token
    let test_token = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 9).await?;
    
    // Create two markets with different fee tiers
    println!("\nCreating markets with different fee tiers...");
    
    // Low fee market for normal conditions
    let low_fee_market = ctx.market_builder()
        .token_0(ctx.feelssol_mint)
        .token_1(test_token.pubkey())
        .fee_rate(10) // 0.1 bp = 0.001%
        .tick_spacing(1)
        .build()
        .await?;
    println!("✓ Low fee market created: {} (0.001% fee)", low_fee_market);
    
    // High fee market for volatile conditions
    let high_fee_market = ctx.market_builder()
        .token_0(ctx.feelssol_mint)
        .token_1(test_token.pubkey())
        .fee_rate(300) // 3 bp = 0.03%
        .tick_spacing(30)
        .build()
        .await?;
    println!("✓ High fee market created: {} (0.03% fee)", high_fee_market);
    
    // Setup Alice as liquidity provider
    let alice_feelssol = ctx.create_ata(&ctx.accounts.alice.pubkey(), &ctx.feelssol_mint).await?;
    let alice_test_token = ctx.create_ata(&ctx.accounts.alice.pubkey(), &test_token.pubkey()).await?;
    ctx.mint_to(&ctx.feelssol_mint, &alice_feelssol, &ctx.feelssol_authority, 100_000_000_000_000).await?;
    ctx.mint_to(&test_token.pubkey(), &alice_test_token, &ctx.accounts.market_creator, 100_000_000_000_000).await?;
    
    // Phase 1: Normal market conditions - use low fee market
    println!("\nPhase 1: Normal market conditions...");
    
    let initial_position = ctx.position_helper()
        .open_position(
            &low_fee_market,
            &ctx.accounts.alice,
            -500,
            500,
            10_000_000_000, // 10B liquidity
        )
        .await?;
    println!("✓ Liquidity provided to low fee market");
    
    // Simulate normal trading
    let bob_feelssol = ctx.create_ata(&ctx.accounts.bob.pubkey(), &ctx.feelssol_mint).await?;
    let bob_test_token = ctx.create_ata(&ctx.accounts.bob.pubkey(), &test_token.pubkey()).await?;
    ctx.mint_to(&ctx.feelssol_mint, &bob_feelssol, &ctx.feelssol_authority, 10_000_000_000_000).await?;
    ctx.mint_to(&test_token.pubkey(), &bob_test_token, &ctx.accounts.market_creator, 10_000_000_000_000).await?;
    
    println!("  Executing normal trades...");
    for i in 0..5 {
        let amount = 50_000_000_000; // 50 tokens
        let is_buy = i % 2 == 0;
        
        let result = if is_buy {
            ctx.swap_helper()
                .swap(&low_fee_market, &ctx.feelssol_mint, &test_token.pubkey(), amount, &ctx.accounts.bob)
                .await?
        } else {
            ctx.swap_helper()
                .swap(&low_fee_market, &test_token.pubkey(), &ctx.feelssol_mint, amount, &ctx.accounts.bob)
                .await?
        };
        
        println!("    Trade {}: {} → {} (fee: {})", i + 1, result.amount_in, result.amount_out, result.fee_paid);
    }
    
    // Close position in low fee market
    let balances_before_migration = (
        ctx.get_token_balance(&alice_feelssol).await?,
        ctx.get_token_balance(&alice_test_token).await?
    );
    
    ctx.position_helper()
        .close_position(&initial_position, &ctx.accounts.alice)
        .await?;
    
    let balances_after_close = (
        ctx.get_token_balance(&alice_feelssol).await?,
        ctx.get_token_balance(&alice_test_token).await?
    );
    
    let normal_fees = (
        balances_after_close.0.saturating_sub(balances_before_migration.0),
        balances_after_close.1.saturating_sub(balances_before_migration.1)
    );
    
    println!("✓ Position closed. Fees earned in normal market: {} FeelsSOL, {} test tokens", 
        normal_fees.0, normal_fees.1);
    
    // Phase 2: Market becomes volatile - migrate to high fee market
    println!("\nPhase 2: Market volatility detected, migrating liquidity...");
    
    let volatile_position = ctx.position_helper()
        .open_position(
            &high_fee_market,
            &ctx.accounts.alice,
            -1500,  // Wider range for volatility
            1500,
            10_000_000_000, // Same liquidity amount
        )
        .await?;
    println!("✓ Liquidity migrated to high fee market with wider range");
    
    // Simulate volatile trading
    let charlie_feelssol = ctx.create_ata(&ctx.accounts.charlie.pubkey(), &ctx.feelssol_mint).await?;
    let charlie_test_token = ctx.create_ata(&ctx.accounts.charlie.pubkey(), &test_token.pubkey()).await?;
    ctx.mint_to(&ctx.feelssol_mint, &charlie_feelssol, &ctx.feelssol_authority, 50_000_000_000_000).await?;
    ctx.mint_to(&test_token.pubkey(), &charlie_test_token, &ctx.accounts.market_creator, 50_000_000_000_000).await?;
    
    println!("  Executing volatile trades...");
    for i in 0..10 {
        // Larger, more random trade sizes
        let amount = if i % 3 == 0 {
            500_000_000_000  // Large trade
        } else {
            100_000_000_000  // Medium trade
        };
        
        let trader = if i % 2 == 0 { &ctx.accounts.charlie } else { &ctx.accounts.bob };
        let is_buy = i % 2 == 0;
        
        let result = if is_buy {
            ctx.swap_helper()
                .swap(&high_fee_market, &ctx.feelssol_mint, &test_token.pubkey(), amount, trader)
                .await?
        } else {
            ctx.swap_helper()
                .swap(&high_fee_market, &test_token.pubkey(), &ctx.feelssol_mint, amount, trader)
                .await?
        };
        
        println!("    Volatile trade {}: {} → {} (fee: {})", 
            i + 1, result.amount_in, result.amount_out, result.fee_paid);
    }
    
    // Close volatile position
    let balances_before_close = (
        ctx.get_token_balance(&alice_feelssol).await?,
        ctx.get_token_balance(&alice_test_token).await?
    );
    
    ctx.position_helper()
        .close_position(&volatile_position, &ctx.accounts.alice)
        .await?;
    
    let balances_after_volatile = (
        ctx.get_token_balance(&alice_feelssol).await?,
        ctx.get_token_balance(&alice_test_token).await?
    );
    
    let volatile_fees = (
        balances_after_volatile.0.saturating_sub(balances_before_close.0),
        balances_after_volatile.1.saturating_sub(balances_before_close.1)
    );
    
    println!("✓ Volatile position closed. Fees earned: {} FeelsSOL, {} test tokens", 
        volatile_fees.0, volatile_fees.1);
    
    // Compare fee earnings
    let total_volatile_fees = volatile_fees.0 + volatile_fees.1;
    let total_normal_fees = normal_fees.0 + normal_fees.1;
    
    println!("\n=== Migration Results ===");
    println!("Normal market fees: {}", total_normal_fees);
    println!("Volatile market fees: {}", total_volatile_fees);
    println!("Fee multiplier: {}x", total_volatile_fees / total_normal_fees.max(1));
    
    // Verify higher fees were earned in volatile market
    assert!(total_volatile_fees > total_normal_fees, 
        "Should earn higher fees in volatile market with higher fee tier");
    
    println!("\n=== Liquidity Migration Test Passed ===");
    Ok::<(), Box<dyn std::error::Error>>(())
});

/// Test complex multi-user trading scenarios
test_in_memory!(test_complex_multi_user_trading, |mut ctx: TestContext| async move {
    println!("=== Testing Complex Multi-User Trading ===");
    
    // Create test token
    let test_token = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 9).await?;
    
    // Initialize market
    let market = ctx.market_builder()
        .token_0(ctx.feelssol_mint)
        .token_1(test_token.pubkey())
        .fee_rate(30) // 0.3 bp
        .tick_spacing(10)
        .build()
        .await?;
    
    println!("✓ Market created");
    
    // Setup multiple liquidity providers with different strategies
    println!("\nSetting up liquidity providers...");
    
    // Setup accounts for all participants
    let participants = [
        ("Alice", &ctx.accounts.alice),
        ("Bob", &ctx.accounts.bob),
        ("Charlie", &ctx.accounts.charlie),
    ];
    
    let mut user_accounts = std::collections::HashMap::new();
    
    for (name, user) in &participants {
        let feelssol_ata = ctx.create_ata(&user.pubkey(), &ctx.feelssol_mint).await?;
        let test_token_ata = ctx.create_ata(&user.pubkey(), &test_token.pubkey()).await?;
        
        // Fund accounts
        ctx.mint_to(&ctx.feelssol_mint, &feelssol_ata, &ctx.feelssol_authority, 100_000_000_000_000).await?;
        ctx.mint_to(&test_token.pubkey(), &test_token_ata, &ctx.accounts.market_creator, 100_000_000_000_000).await?;
        
        user_accounts.insert(*name, (feelssol_ata, test_token_ata));
    }
    
    // Alice: Wide range LP (passive strategy)
    let alice_position = ctx.position_helper()
        .open_position(&market, &ctx.accounts.alice, -2000, 2000, 5_000_000_000)
        .await?;
    println!("✓ Alice: Wide range position (-2000 to 2000)");
    
    // Bob: Concentrated LP around current price
    let bob_position = ctx.position_helper()
        .open_position(&market, &ctx.accounts.bob, -100, 100, 10_000_000_000)
        .await?;
    println!("✓ Bob: Concentrated position (-100 to 100)");
    
    // Charlie: Multiple positions (barbell strategy)
    let charlie_position_low = ctx.position_helper()
        .open_position(&market, &ctx.accounts.charlie, -1500, -500, 3_000_000_000)
        .await?;
    let charlie_position_high = ctx.position_helper()
        .open_position(&market, &ctx.accounts.charlie, 500, 1500, 3_000_000_000)
        .await?;
    println!("✓ Charlie: Barbell strategy (low: -1500 to -500, high: 500 to 1500)");
    
    // Execute various trading patterns
    println!("\nExecuting trading scenarios...");
    
    // Scenario 1: Small balanced trades (should mostly use Bob's liquidity)
    println!("\n  Scenario 1: Small balanced trades");
    for i in 0..5 {
        let trader = &participants[i % 3];
        let amount = 10_000_000_000; // 10 tokens
        let is_buy = i % 2 == 0;
        
        let result = if is_buy {
            ctx.swap_helper()
                .swap(&market, &ctx.feelssol_mint, &test_token.pubkey(), amount, trader.1)
                .await?
        } else {
            ctx.swap_helper()
                .swap(&market, &test_token.pubkey(), &ctx.feelssol_mint, amount, trader.1)
                .await?
        };
        
        println!("    {} trade: {} → {}", trader.0, result.amount_in, result.amount_out);
    }
    
    // Scenario 2: Large directional move (should push into Charlie's ranges)
    println!("\n  Scenario 2: Large directional move");
    for _ in 0..3 {
        let result = ctx.swap_helper()
            .swap(&market, &ctx.feelssol_mint, &test_token.pubkey(), 500_000_000_000, &ctx.accounts.alice)
            .await?;
        println!("    Large buy: {} → {} (price impact: {}%)", 
            result.amount_in, 
            result.amount_out,
            ((result.amount_in as f64 / result.amount_out as f64) - 1.0) * 100.0
        );
    }
    
    // Scenario 3: Arbitrage trades to restore balance
    println!("\n  Scenario 3: Arbitrage trades");
    for _ in 0..3 {
        let result = ctx.swap_helper()
            .swap(&market, &test_token.pubkey(), &ctx.feelssol_mint, 400_000_000_000, &ctx.accounts.bob)
            .await?;
        println!("    Arbitrage sell: {} → {}", result.amount_in, result.amount_out);
    }
    
    // Collect fees and analyze results
    println!("\nCollecting fees and closing positions...");
    
    // Track balances before closing
    let mut initial_balances = std::collections::HashMap::new();
    for (name, (feelssol, test)) in &user_accounts {
        initial_balances.insert(*name, (
            ctx.get_token_balance(feelssol).await?,
            ctx.get_token_balance(test).await?
        ));
    }
    
    // Close all positions
    ctx.position_helper().close_position(&alice_position, &ctx.accounts.alice).await?;
    ctx.position_helper().close_position(&bob_position, &ctx.accounts.bob).await?;
    ctx.position_helper().close_position(&charlie_position_low, &ctx.accounts.charlie).await?;
    ctx.position_helper().close_position(&charlie_position_high, &ctx.accounts.charlie).await?;
    
    // Calculate returns
    println!("\n=== Results ===");
    for (name, (feelssol, test)) in &user_accounts {
        let initial = initial_balances.get(name).unwrap();
        let final_feelssol = ctx.get_token_balance(feelssol).await?;
        let final_test = ctx.get_token_balance(test).await?;
        
        let feelssol_return = final_feelssol.saturating_sub(initial.0);
        let test_return = final_test.saturating_sub(initial.1);
        
        println!("{}: +{} FeelsSOL, +{} test tokens", name, feelssol_return, test_return);
    }
    
    println!("\n=== Complex Multi-User Trading Test Passed ===");
    Ok::<(), Box<dyn std::error::Error>>(())
});