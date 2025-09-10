use crate::common::*;

#[tokio::test]
async fn test_market_creation_to_fee_collection() {
    let mut suite = TestSuite::new().await.unwrap();
    let accounts = fixtures::get_test_accounts();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    // Step 1: Create market
    println!("Creating market...");
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 5)
        .build(&mut suite).await.unwrap();
    
    // Step 2: Add liquidity positions
    println!("Adding liquidity...");
    let alice_position = PositionBuilder::new()
        .with_market(market_data.market)
        .with_owner(accounts.alice.insecure_clone())
        .with_ticks(-1000, 1000)
        .with_liquidity(1_000_000_000)
        .open(&mut suite).await.unwrap();
    
    let bob_position = PositionBuilder::new()
        .with_market(market_data.market)
        .with_owner(accounts.bob.insecure_clone())
        .with_ticks(-500, 500)
        .with_liquidity(2_000_000_000)
        .open(&mut suite).await.unwrap();
    
    // Step 3: Execute multiple swaps
    println!("Executing swaps...");
    let mut total_volume = 0u64;
    
    for i in 0..10 {
        let zero_for_one = i % 2 == 0;
        let user = if i < 5 { &accounts.alice } else { &accounts.charlie };
        
        let swap_result = SwapTestBuilder::new()
            .with_market(market_data.market)
            .with_user(user.insecure_clone())
            .with_amount(fixtures::test_constants::MEDIUM_SWAP_AMOUNT)
            .zero_for_one(zero_for_one)
            .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
            .execute(&mut suite).await.unwrap();
        
        total_volume += swap_result.amount_in;
        println!("  Swap {}: {} -> {}", i + 1, swap_result.amount_in, swap_result.amount_out);
    }
    
    // Step 4: Collect fees from positions
    println!("Collecting fees...");
    
    // Alice collects fees
    let (alice_fees_0, alice_fees_1) = suite.collect_fees(
        alice_position,
        &accounts.alice,
    ).await.unwrap();
    
    println!("  Alice collected: {} token0, {} token1", alice_fees_0, alice_fees_1);
    assert!(alice_fees_0 > 0 || alice_fees_1 > 0, "Alice should have earned fees");
    
    // Bob collects fees
    let (bob_fees_0, bob_fees_1) = suite.collect_fees(
        bob_position,
        &accounts.bob,
    ).await.unwrap();
    
    println!("  Bob collected: {} token0, {} token1", bob_fees_0, bob_fees_1);
    assert!(bob_fees_0 > 0 || bob_fees_1 > 0, "Bob should have earned fees");
    
    // Verify total fees are reasonable
    let total_fees = alice_fees_0 + alice_fees_1 + bob_fees_0 + bob_fees_1;
    let expected_fees = (total_volume as u128 * market_data.fee_tier as u128) / 1_000_000;
    
    println!("Total volume: {}, Total fees collected: {}, Expected: {}", 
        total_volume, total_fees, expected_fees);
    
    // Allow for rounding differences
    let tolerance = expected_fees / 100; // 1% tolerance
    assert!(
        (total_fees as i128 - expected_fees as i128).abs() <= tolerance as i128,
        "Total fees should match expected"
    );
    
    // Step 5: Close positions
    println!("Closing positions...");
    suite.close_position(alice_position, &accounts.alice).await.unwrap();
    suite.close_position(bob_position, &accounts.bob).await.unwrap();
    
    println!("Full trading flow completed successfully!");
}

#[tokio::test]
async fn test_liquidity_migration_scenario() {
    let mut suite = TestSuite::new().await.unwrap();
    let accounts = fixtures::get_test_accounts();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    // Create two markets with different fee tiers
    let low_fee_market = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::LOW_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 5)
        .build(&mut suite).await.unwrap();
    
    let high_fee_market = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::HIGH_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::HIGH_FEE_TIER)
        .add_tick_arrays_around_current(0, 5)
        .build(&mut suite).await.unwrap();
    
    // Add initial liquidity to low fee market
    let initial_position = PositionBuilder::new()
        .with_market(low_fee_market.market)
        .with_owner(accounts.alice.insecure_clone())
        .with_ticks(-1000, 1000)
        .with_liquidity(5_000_000_000)
        .open(&mut suite).await.unwrap();
    
    // Simulate period of normal trading
    for i in 0..5 {
        SwapTestBuilder::new()
            .with_market(low_fee_market.market)
            .with_user(accounts.bob.insecure_clone())
            .with_amount(fixtures::test_constants::SMALL_SWAP_AMOUNT)
            .zero_for_one(i % 2 == 0)
            .with_tick_arrays(low_fee_market.tick_arrays.values().cloned().collect())
            .execute(&mut suite).await.unwrap();
    }
    
    // Simulate market volatility - migrate liquidity to high fee market
    println!("Market volatility detected, migrating liquidity...");
    
    // Close position in low fee market
    suite.close_position(initial_position, &accounts.alice).await.unwrap();
    
    // Open new position in high fee market
    let new_position = PositionBuilder::new()
        .with_market(high_fee_market.market)
        .with_owner(accounts.alice.insecure_clone())
        .with_ticks(-2000, 2000) // Wider range for volatility
        .with_liquidity(5_000_000_000)
        .open(&mut suite).await.unwrap();
    
    // Simulate volatile trading
    for i in 0..10 {
        let amount = if i % 3 == 0 {
            fixtures::test_constants::LARGE_SWAP_AMOUNT
        } else {
            fixtures::test_constants::MEDIUM_SWAP_AMOUNT
        };
        
        SwapTestBuilder::new()
            .with_market(high_fee_market.market)
            .with_user(accounts.charlie.insecure_clone())
            .with_amount(amount)
            .zero_for_one(i % 2 == 0)
            .with_tick_arrays(high_fee_market.tick_arrays.values().cloned().collect())
            .execute(&mut suite).await.unwrap();
    }
    
    // Collect fees from high fee market
    let (fees_0, fees_1) = suite.collect_fees(
        new_position,
        &accounts.alice,
    ).await.unwrap();
    
    println!("Collected {} token0 and {} token1 in fees from volatile market", fees_0, fees_1);
    assert!(fees_0 > 0 || fees_1 > 0, "Should have earned higher fees in volatile market");
}