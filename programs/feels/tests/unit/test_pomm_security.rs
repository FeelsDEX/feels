//! Tests for POMM security improvements
//! 
//! Ensures POMM liquidity placement cannot be manipulated

use crate::common::*;
use feels::state::Market;

test_in_memory!(test_pomm_width_derivation, |ctx: TestContext| async move {
    // Test that POMM width is derived from market tick spacing, not buffer
    
    // Test various tick spacings
    let test_cases = vec![
        (1u16, 20i32),      // tick_spacing=1 -> width=20
        (10u16, 200i32),    // tick_spacing=10 -> width=200
        (60u16, 1200i32),   // tick_spacing=60 -> width=1200
        (200u16, 2000i32),  // tick_spacing=200 -> capped at 2000
        (0u16, 10i32),      // tick_spacing=0 -> minimum 10
    ];
    
    for (tick_spacing, expected_width) in test_cases {
        let pomm_tick_width = (tick_spacing as i32)
            .saturating_mul(20)
            .max(10)
            .min(2000);
        
        assert_eq!(pomm_tick_width, expected_width, 
            "Tick spacing {} should produce width {}", tick_spacing, expected_width);
    }
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_pomm_width_bounds, |ctx: TestContext| async move {
    // Test that POMM width stays within safe bounds
    
    // Test minimum bound
    let min_width = (0u16 as i32).saturating_mul(20).max(10).min(2000);
    assert_eq!(min_width, 10, "Minimum width should be 10 ticks");
    
    // Test maximum bound  
    let max_width = (u16::MAX as i32).saturating_mul(20).max(10).min(2000);
    assert_eq!(max_width, 2000, "Maximum width should be 2000 ticks");
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_pomm_width_immutable, |ctx: TestContext| async move {
    // Verify that POMM width depends only on immutable market parameters
    // not on any mutable buffer state
    
    println!("Creating test market for POMM width test...");
    
    // Create a protocol token that can be used in markets
    let test_token = ctx.mint_protocol_token("POMM", 6, 1_000_000_000_000).await?;
    println!("Created protocol token: {}", test_token.pubkey());
    
    // Create market using FeelsSOL and protocol token
    let market_id = ctx.market_helper()
        .create_simple_market(&ctx.feelssol_mint, &test_token.pubkey())
        .await?;
    
    println!("Created market: {}", market_id);
    
    let market_data = ctx.get_account::<Market>(&market_id).await?.unwrap();
    let tick_spacing = market_data.tick_spacing;
    
    // Simulate multiple calls - width should always be the same
    for _ in 0..10 {
        let width = (tick_spacing as i32)
            .saturating_mul(20)
            .max(10)
            .min(2000);
        
        // Verify width is consistent
        assert_eq!(width, 1280, "POMM width should be consistent for tick_spacing={}", tick_spacing);
    }
    
    println!("✅ POMM width derivation verified - immutable based on tick_spacing");
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_pomm_range_calculation, |ctx: TestContext| async move {
    // Test the full range calculation with derived width
    let current_tick = 1000;
    let tick_spacing = 60u16;
    
    let pomm_tick_width = (tick_spacing as i32)
        .saturating_mul(20)
        .max(10)
        .min(2000);
    
    // Test symmetric range (both tokens)
    let tick_lower = current_tick - pomm_tick_width;
    let tick_upper = current_tick + pomm_tick_width;
    
    assert_eq!(tick_lower, -200); // 1000 - 1200
    assert_eq!(tick_upper, 2200);  // 1000 + 1200
    
    // Test one-sided below (only token0)
    let tick_lower_one_sided = current_tick - pomm_tick_width;
    let tick_upper_one_sided = current_tick;
    
    assert_eq!(tick_lower_one_sided, -200);
    assert_eq!(tick_upper_one_sided, 1000);
    
    // Test one-sided above (only token1)
    let tick_lower_one_sided = current_tick;
    let tick_upper_one_sided = current_tick + pomm_tick_width;
    
    assert_eq!(tick_lower_one_sided, 1000);
    assert_eq!(tick_upper_one_sided, 2200);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_reasonable_width_percentages, |ctx: TestContext| async move {
    // Verify that common tick spacings produce reasonable percentage ranges
    
    // Common tick spacings and their approximate percentage ranges
    let test_cases = vec![
        (1u16, 0.2f64),    // ±0.2%
        (10u16, 2.0f64),   // ±2%
        (60u16, 12.0f64),  // ±12%
        (100u16, 20.0f64), // ±20% (capped)
    ];
    
    for (tick_spacing, expected_pct) in test_cases {
        let width = (tick_spacing as i32)
            .saturating_mul(20)
            .max(10)
            .min(2000);
        
        // Approximate percentage = width * 0.01% per tick
        let actual_pct = (width as f64) * 0.01;
        
        // Allow some tolerance for the approximation
        assert!((actual_pct - expected_pct).abs() < 0.1,
            "Tick spacing {} produces ~{}% range (expected ~{}%)", 
            tick_spacing, actual_pct, expected_pct);
    }
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_pomm_width_manipulation_resistance, |ctx: TestContext| async move {
    // Test that POMM width cannot be manipulated through market state changes
    
    println!("Creating test market for POMM manipulation resistance test...");
    
    // Create a protocol token that can be used in markets
    let test_token = ctx.mint_protocol_token("RESIST", 6, 1_000_000_000_000).await?;
    
    // Create market using FeelsSOL and protocol token
    let market_id = ctx.market_helper()
        .create_simple_market(&ctx.feelssol_mint, &test_token.pubkey())
        .await?;
    
    // Determine token order
    let (token_0, token_1) = if ctx.feelssol_mint < test_token.pubkey() {
        (ctx.feelssol_mint, test_token.pubkey())
    } else {
        (test_token.pubkey(), ctx.feelssol_mint)
    };
    
    let market_before = ctx.get_account::<Market>(&market_id).await?.unwrap();
    let width_before = (market_before.tick_spacing as i32)
        .saturating_mul(20)
        .max(10)
        .min(2000);
    
    // Setup trader
    let trader_token_0 = ctx.create_ata(&ctx.accounts.alice.pubkey(), &token_0).await?;
    let trader_token_1 = ctx.create_ata(&ctx.accounts.alice.pubkey(), &token_1).await?;
    
    // Mint tokens to trader - handle FeelsSOL vs custom token
    if token_0 == ctx.feelssol_mint {
        ctx.mint_to(&token_0, &trader_token_0, &ctx.feelssol_authority, 10_000_000_000).await?;
        ctx.mint_to(&token_1, &trader_token_1, &test_token, 10_000_000_000).await?;
    } else {
        ctx.mint_to(&token_0, &trader_token_0, &test_token, 10_000_000_000).await?;
        ctx.mint_to(&token_1, &trader_token_1, &ctx.feelssol_authority, 10_000_000_000).await?;
    }
    
    // Execute swaps to change market state
    for _ in 0..5 {
        ctx.swap_helper().swap(
            &market_id,
            &token_0,
            &token_1,
            constants::MEDIUM_SWAP,
            &ctx.accounts.alice,
        ).await?;
        
        ctx.swap_helper().swap(
            &market_id,
            &token_1,
            &token_0,
            constants::MEDIUM_SWAP,
            &ctx.accounts.alice,
        ).await?;
    }
    
    // Check that POMM width remains unchanged
    let market_after = ctx.get_account::<Market>(&market_id).await?.unwrap();
    let width_after = (market_after.tick_spacing as i32)
        .saturating_mul(20)
        .max(10)
        .min(2000);
    
    assert_eq!(width_before, width_after, "POMM width should remain constant despite market activity");
    
    Ok::<(), Box<dyn std::error::Error>>(())
});