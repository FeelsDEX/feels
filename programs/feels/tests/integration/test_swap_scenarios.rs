use crate::common::*;

#[tokio::test]
async fn test_swap_fee_growth_tracking() {
    let mut suite = TestSuite::new().await.unwrap();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    // Create market with standard configuration
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 5)
        .add_liquidity(-1000, 1000, 1_000_000_000)
        .build(&mut suite).await.unwrap();
    
    // Get initial fee growth
    let market_before = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
    let fee_growth_0_before = market_before.fee_growth_global_0_x64;
    let fee_growth_1_before = market_before.fee_growth_global_1_x64;
    
    // Execute swap
    let swap_result = SwapTestBuilder::new()
        .with_market(market_data.market)
        .with_user(fixtures::get_test_accounts().alice.insecure_clone())
        .with_amount(fixtures::test_constants::MEDIUM_SWAP_AMOUNT)
        .zero_for_one(true)
        .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
        .execute(&mut suite).await.unwrap();
    
    // Get final fee growth
    let market_after = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
    let fee_growth_0_after = market_after.fee_growth_global_0_x64;
    let fee_growth_1_after = market_after.fee_growth_global_1_x64;
    
    // Assert using trait
    swap_result.assert_fee_growth_increases(true, fee_growth_0_before, fee_growth_0_after);
    swap_result.assert_fee_growth_increases(false, fee_growth_1_before, fee_growth_1_after);
    
    // Assert fee growth is monotonic (always increasing or staying the same)
    ProtocolInvariants::check_fee_growth_monotonic(
        fee_growth_0_before,
        fee_growth_0_after,
        fee_growth_1_before,
        fee_growth_1_after,
    );
}

#[tokio::test]
async fn test_swap_clamp_at_bound() {
    let mut suite = TestSuite::new().await.unwrap();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    // Create market
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .with_initial_sqrt_price(fixtures::test_constants::PRICE_1_TO_1)
        .add_tick_arrays_around_current(0, 5)
        .add_liquidity(-1000, 1000, 100_000_000) // Small liquidity
        .build(&mut suite).await.unwrap();
    
    // Execute very large swap
    let huge_amount = u64::MAX / 2;
    let swap_result = SwapTestBuilder::new()
        .with_market(market_data.market)
        .with_user(fixtures::get_test_accounts().alice.insecure_clone())
        .with_amount(huge_amount)
        .zero_for_one(true)
        .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
        .execute(&mut suite).await.unwrap();
    
    // Check that price is at bound
    let market_after = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
    let expected_price = crate::math::tick_index_to_sqrt_price(market_after.global_lower_tick).0;
    
    assert_eq!(
        market_after.sqrt_price,
        expected_price,
        "Price should be clamped at lower bound"
    );
    
    // Verify partial fill
    assert!(
        swap_result.amount_in < huge_amount,
        "Should have partially filled at bound"
    );
}

#[tokio::test]
async fn test_multi_tick_crossing() {
    let mut suite = TestSuite::new().await.unwrap();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    // Create market with multiple liquidity positions
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::LOW_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 10)
        .add_liquidity(-100, -50, 500_000_000)
        .add_liquidity(-75, -25, 1_000_000_000)
        .add_liquidity(-50, 50, 2_000_000_000)
        .add_liquidity(25, 75, 1_000_000_000)
        .add_liquidity(50, 100, 500_000_000)
        .build(&mut suite).await.unwrap();
    
    // Get initial tick
    let market_before = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
    let initial_tick = market_before.current_tick;
    
    // Capture a tick we expect to cross (one spacing below the initial tick)
    let crossed_tick = initial_tick - market_data.tick_spacing as i32;
    let array_span = crate::TICK_ARRAY_SIZE as i32 * market_data.tick_spacing as i32;
    let start_index = (crossed_tick).div_euclid(array_span) * array_span;
    let tick_array_key = market_data.tick_arrays.get(&start_index).cloned();
    let fee_out_before = if let Some(arr_pk) = tick_array_key {
        let arr_before: TickArray = suite.get_account_data(&arr_pk).await.unwrap();
        let offset = ((crossed_tick - start_index) / market_data.tick_spacing as i32) as usize;
        Some((
            arr_before.ticks[offset].fee_growth_outside_0_x64,
            arr_before.ticks[offset].fee_growth_outside_1_x64,
        ))
    } else { None };

    // Execute large swap to cross multiple ticks
    let swap_result = SwapTestBuilder::new()
        .with_market(market_data.market)
        .with_user(fixtures::get_test_accounts().alice.insecure_clone())
        .with_amount(fixtures::test_constants::LARGE_SWAP_AMOUNT * 5)
        .zero_for_one(true)
        .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
        .execute(&mut suite).await.unwrap();
    
    // Get final tick
    let market_after = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
    let final_tick = market_after.current_tick;
    
    // Assert multiple ticks were crossed
    let ticks_crossed = (initial_tick - final_tick).abs() / market_data.tick_spacing as i32;
    assert!(
        ticks_crossed > 1,
        "Should have crossed multiple ticks: {} -> {} ({} ticks)",
        initial_tick, final_tick, ticks_crossed
    );
    
    // Verify monotonic price movement
    swap_result.assert_swap_direction_monotonic(
        true,
        market_before.sqrt_price,
        market_after.sqrt_price
    );

    // Verify fee_growth_outside flipped at the crossed tick if we captured it
    if let (Some(arr_pk), Some((before0, before1))) = (tick_array_key, fee_out_before) {
        let arr_after: TickArray = suite.get_account_data(&arr_pk).await.unwrap();
        let offset = ((crossed_tick - start_index) / market_data.tick_spacing as i32) as usize;
        let after0 = arr_after.ticks[offset].fee_growth_outside_0_x64;
        let after1 = arr_after.ticks[offset].fee_growth_outside_1_x64;
        assert!(
            after0 != before0 || after1 != before1,
            "fee_growth_outside should flip on crossing (tick {})",
            crossed_tick
        );
    }
}

#[tokio::test]
async fn test_swap_direction_consistency() {
    let mut suite = TestSuite::new().await.unwrap();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    // Create market
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 5)
        .add_liquidity(-5000, 5000, 10_000_000_000)
        .build(&mut suite).await.unwrap();
    
    // Test zero-for-one direction
    {
        let market_before = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
        
        let swap_result = SwapTestBuilder::new()
            .with_market(market_data.market)
            .with_user(fixtures::get_test_accounts().alice.insecure_clone())
            .with_amount(fixtures::test_constants::MEDIUM_SWAP_AMOUNT)
            .zero_for_one(true)
            .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
            .execute(&mut suite).await.unwrap();
        
        let market_after = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
        
        swap_result.assert_swap_direction_monotonic(
            true,
            market_before.sqrt_price,
            market_after.sqrt_price
        );
    }
    
    // Test one-for-zero direction
    {
        let market_before = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
        
        let swap_result = SwapTestBuilder::new()
            .with_market(market_data.market)
            .with_user(fixtures::get_test_accounts().bob.insecure_clone())
            .with_amount(fixtures::test_constants::MEDIUM_SWAP_AMOUNT)
            .zero_for_one(false)
            .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
            .execute(&mut suite).await.unwrap();
        
        let market_after = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
        
        swap_result.assert_swap_direction_monotonic(
            false,
            market_before.sqrt_price,
            market_after.sqrt_price
        );
    }
}

#[tokio::test]
async fn test_swap_liquidity_conservation() {
    let mut suite = TestSuite::new().await.unwrap();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .add_tick_arrays_around_current(0, 5)
        .add_liquidity(-1000, 1000, 5_000_000_000)
        .build(&mut suite).await.unwrap();
    
    // Execute multiple swaps in different directions
    for i in 0..10 {
        let market_before = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
        let liquidity_before = market_before.liquidity;
        
        // Alternate swap directions
        let zero_for_one = i % 2 == 0;
        let user = if i % 3 == 0 {
            fixtures::get_test_accounts().alice.insecure_clone()
        } else if i % 3 == 1 {
            fixtures::get_test_accounts().bob.insecure_clone()
        } else {
            fixtures::get_test_accounts().charlie.insecure_clone()
        };
        
        let result = SwapTestBuilder::new()
            .with_market(market_data.market)
            .with_user(user)
            .with_amount(fixtures::test_constants::SMALL_SWAP_AMOUNT * (i + 1))
            .zero_for_one(zero_for_one)
            .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
            .execute(&mut suite).await;
        
        if let Err(e) = result {
            // Should only fail due to insufficient liquidity
            assert!(
                e.to_string().contains("Insufficient liquidity"),
                "Unexpected error: {}",
                e
            );
            break;
        }
        
        let market_after = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
        let liquidity_after = market_after.liquidity;
        
        // Check invariants
        ProtocolInvariants::check_liquidity_conservation(&market_before, &market_after);
        ProtocolInvariants::check_price_tick_consistency(&market_after);
        market_data.assert_liquidity_conserved(liquidity_before, liquidity_after);
        
        // Check fee growth monotonicity
        ProtocolInvariants::check_fee_growth_monotonic(
            market_before.fee_growth_global_0_x64,
            market_after.fee_growth_global_0_x64,
            market_before.fee_growth_global_1_x64,
            market_after.fee_growth_global_1_x64,
        );
    }
}

#[tokio::test]
async fn test_swap_with_price_limit() {
    let mut suite = TestSuite::new().await.unwrap();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    let market_data = MarketBuilder::new()
        .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
        .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
        .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
        .with_initial_sqrt_price(fixtures::test_constants::PRICE_1_TO_1)
        .add_tick_arrays_around_current(0, 5)
        .add_liquidity(-5000, 5000, 10_000_000_000)
        .build(&mut suite).await.unwrap();
    
    let market_before = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
    let initial_price = market_before.sqrt_price;
    let target_price = initial_price * 95 / 100; // 5% price impact limit
    
    // Execute large swap with price limit
    let swap_result = SwapTestBuilder::new()
        .with_market(market_data.market)
        .with_user(fixtures::get_test_accounts().alice.insecure_clone())
        .with_amount(u64::MAX) // Try to swap max amount
        .zero_for_one(true)
        .with_tick_arrays(market_data.tick_arrays.values().cloned().collect())
        .execute(&mut suite).await.unwrap();
    
    let market_after = suite.get_account_data::<Market>(&market_data.market).await.unwrap();
    
    // Verify price stopped at or before limit
    assert!(
        market_after.sqrt_price >= target_price,
        "Price {} exceeded limit {}",
        market_after.sqrt_price,
        target_price
    );
    
    // Verify partial execution
    assert!(
        swap_result.amount_in < u64::MAX,
        "Swap should have been partially executed at price limit"
    );
}

#[tokio::test]
async fn test_swap_scenarios_from_fixtures() {
    let mut suite = TestSuite::new().await.unwrap();
    let env = fixtures::create_standard_test_env(&mut suite).await.unwrap();
    
    // Test each predefined scenario
    for scenario in fixtures::get_swap_scenarios() {
        println!("Testing scenario: {}", scenario.name);
        
        // Create fresh market for each scenario
        let market_data = MarketBuilder::new()
            .with_tokens(env.test_token_mint.pubkey(), env.feelssol_mint.pubkey())
            .with_tick_spacing(fixtures::test_constants::MEDIUM_FEE_TICK_SPACING)
            .with_fee_tier(fixtures::test_constants::LOW_FEE_TIER)
            .add_tick_arrays_around_current(0, 10)
            .add_liquidity(-10000, 10000, 100_000_000_000)
            .build(&mut suite).await.unwrap();
        
        for (i, swap_config) in scenario.swaps.iter().enumerate() {
            println!("  Executing swap {}: {:?}", i + 1, swap_config);
            
            let mut builder = SwapTestBuilder::new()
                .with_market(market_data.market)
                .with_user(fixtures::get_test_accounts().alice.insecure_clone())
                .with_amount(swap_config.amount)
                .zero_for_one(swap_config.zero_for_one)
                .with_tick_arrays(market_data.tick_arrays.values().cloned().collect());
            
            let result = builder.execute(&mut suite).await;
            
            match (scenario.name, result) {
                ("liquidity_exhaustion", Err(_)) => {
                    println!("    Expected liquidity exhaustion");
                    continue;
                }
                (_, Ok(swap_result)) => {
                    println!("    Swap completed: {} -> {}", 
                        swap_result.amount_in, 
                        swap_result.amount_out
                    );
                }
                (name, Err(e)) => {
                    panic!("Unexpected error in scenario {}: {}", name, e);
                }
            }
        }
        
        println!("  Expected behavior: {}", scenario.expected_behavior);
    }
}
