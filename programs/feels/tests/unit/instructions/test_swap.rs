use crate::common::*;

#[tokio::test]
async fn test_swap_exact_input_zero_for_one() {
    let mut suite = TestSuite::new().await.unwrap();
    let accounts = fixtures::get_test_accounts();
    
    // Setup environment
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    // Create market
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 3)
        .add_liquidity(-1000, 1000, 1_000_000_000)
        .build(&mut suite).await.unwrap();
    
    // Create user token accounts
    let user_token_0 = suite.create_token_account(
        &env.test_token_mint.pubkey(),
        &accounts.alice.pubkey()
    ).await.unwrap();
    
    let user_token_1 = suite.create_token_account(
        &env.feelssol_mint.pubkey(),
        &accounts.alice.pubkey()
    ).await.unwrap();
    
    // Mint tokens to user
    let input_amount = fixtures::test_constants::MEDIUM_SWAP_AMOUNT;
    suite.mint_to(
        &env.test_token_mint.pubkey(),
        &user_token_0.pubkey(),
        &accounts.market_creator,
        input_amount * 2,
    ).await.unwrap();
    
    // Get balances before
    let balance_0_before = suite.get_token_balance(&user_token_0.pubkey()).await.unwrap();
    let balance_1_before = suite.get_token_balance(&user_token_1.pubkey()).await.unwrap();
    
    // Execute swap
    let swap_result = SwapTestBuilder::new()
        .with_market(market_data.market)
        .with_user(accounts.alice.insecure_clone())
        .with_amount(input_amount)
        .zero_for_one(true)
        .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
        .execute(&mut suite).await.unwrap();
    
    // Get balances after
    let balance_0_after = suite.get_token_balance(&user_token_0.pubkey()).await.unwrap();
    let balance_1_after = suite.get_token_balance(&user_token_1.pubkey()).await.unwrap();
    
    // Assertions
    assert_eq!(
        balance_0_before - balance_0_after,
        swap_result.amount_in,
        "Token 0 balance should decrease by amount in"
    );
    
    assert_eq!(
        balance_1_after - balance_1_before,
        swap_result.amount_out,
        "Token 1 balance should increase by amount out"
    );
    
    assert!(
        swap_result.fee_amount > 0,
        "Should have collected fees"
    );
    
    let expected_fee = (input_amount as u128 * market_data.fee_tier as u128) / 1_000_000;
    assert!(
        (swap_result.fee_amount as i64 - expected_fee as i64).abs() <= 1,
        "Fee amount should match expected"
    );
}

#[tokio::test]
async fn test_swap_exact_input_one_for_zero() {
    let mut suite = TestSuite::new().await.unwrap();
    let accounts = fixtures::get_test_accounts();
    
    // Setup similar to above but swap in opposite direction
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 3)
        .add_liquidity(-1000, 1000, 1_000_000_000)
        .build(&mut suite).await.unwrap();
    
    // Create user token accounts
    let user_token_0 = suite.create_token_account(
        &env.test_token_mint.pubkey(),
        &accounts.bob.pubkey()
    ).await.unwrap();
    
    let user_token_1 = suite.create_token_account(
        &env.feelssol_mint.pubkey(),
        &accounts.bob.pubkey()
    ).await.unwrap();
    
    // Mint token 1 to user for one-for-zero swap
    let input_amount = fixtures::test_constants::MEDIUM_SWAP_AMOUNT;
    #[cfg(feature = "test-utils")]
    suite.mint_feelssol_test(
        env.feelssol_mint.pubkey(),
        user_token_1.pubkey(),
        input_amount * 2,
    ).await.unwrap();
    
    // Execute swap
    let swap_result = SwapTestBuilder::new()
        .with_market(market_data.market)
        .with_user(accounts.bob.insecure_clone())
        .with_amount(input_amount)
        .zero_for_one(false) // One for zero
        .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
        .execute(&mut suite).await.unwrap();
    
    // Verify swap executed in correct direction
    let market_after = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
    assert!(
        market_after.sqrt_price > fixtures::test_constants::PRICE_1_TO_1,
        "Price should have increased for one-for-zero swap"
    );
}

#[tokio::test]
async fn test_swap_with_slippage_protection() {
    let mut suite = TestSuite::new().await.unwrap();
    let accounts = fixtures::get_test_accounts();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 3)
        .add_liquidity(-100, 100, 10_000_000) // Small liquidity for high slippage
        .build(&mut suite).await.unwrap();
    
    // Try swap with tight slippage limit
    let result = SwapTestBuilder::new()
        .with_market(market_data.market)
        .with_user(accounts.alice.insecure_clone())
        .with_amount(fixtures::test_constants::LARGE_SWAP_AMOUNT)
        .with_minimum_output(fixtures::test_constants::LARGE_SWAP_AMOUNT * 99 / 100) // Max 1% slippage
        .zero_for_one(true)
        .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
        .execute(&mut suite).await;
    
    // Should fail due to excessive slippage
    assert_error!(result, FeelsError::SlippageExceeded);
}

#[tokio::test]
async fn test_swap_insufficient_liquidity_error() {
    let mut suite = TestSuite::new().await.unwrap();
    let accounts = fixtures::get_test_accounts();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    // Create market with no liquidity
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 3)
        // No liquidity added!
        .build(&mut suite).await.unwrap();
    
    // Try to swap
    let result = SwapTestBuilder::new()
        .with_market(market_data.market)
        .with_user(accounts.alice.insecure_clone())
        .with_amount(fixtures::test_constants::SMALL_SWAP_AMOUNT)
        .zero_for_one(true)
        .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
        .execute(&mut suite).await;
    
    assert_error!(result, FeelsError::InsufficientLiquidity);
}

#[tokio::test]
async fn test_swap_missing_tick_array_error() {
    let mut suite = TestSuite::new().await.unwrap();
    let accounts = fixtures::get_test_accounts();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 1) // Only one array
        .add_liquidity(-5000, 5000, 1_000_000_000) // Liquidity spans multiple arrays
        .build(&mut suite).await.unwrap();
    
    // Try large swap that would need multiple arrays
    let result = SwapTestBuilder::new()
        .with_market(market_data.market)
        .with_user(accounts.alice.insecure_clone())
        .with_amount(u64::MAX) // Huge swap
        .zero_for_one(true)
        .with_tick_arrays(vec![market_data.tick_arrays.values().next().unwrap().clone()]) // Only provide one array
        .execute(&mut suite).await;
    
    assert_error!(result, FeelsError::MissingTickArrayCoverage);
}

#[tokio::test]
async fn test_swap_max_ticks_crossed() {
    let mut suite = TestSuite::new().await.unwrap();
    let accounts = fixtures::get_test_accounts();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    // Create market with many small liquidity positions
    let mut builder = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::LOW_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 10);
    
    // Add many small positions to force tick crossings
    for i in 0..20 {
        let lower = -100 + i * 10;
        let upper = lower + 10;
        builder = builder.add_liquidity(lower, upper, 100_000);
    }
    
    let market_data = builder.build(&mut suite).await.unwrap();
    
    // Execute swap with tick limit
    let swap_result = SwapTestBuilder::new()
        .with_market(market_data.market)
        .with_user(accounts.alice.insecure_clone())
        .with_amount(u64::MAX)
        .zero_for_one(true)
        .with_max_ticks(3)
        .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
        .execute(&mut suite).await.unwrap();
    
    // Should have stopped at tick limit
    assert!(
        swap_result.amount_in < u64::MAX,
        "Swap should have been limited by max ticks"
    );
}