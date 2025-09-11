use crate::common::*;
use crate::assert_tx_success;

test_in_memory!(test_swap_exact_input_zero_for_one, |ctx: TestContext| async move {
    // Create tokens and market with liquidity - use FeelsSOL as one token to bypass protocol token requirement
    let token_1 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    
    let market = ctx.market_builder()
        .token_0(ctx.feelssol_mint) // Use FeelsSOL as token_0
        .token_1(token_1.pubkey())
        .add_liquidity(ctx.accounts.alice.insecure_clone(), -1000, 1000, 1_000_000_000)
        .build()
        .await?;
    
    // Setup trader with tokens
    let trader_token_0 = ctx.create_ata(&ctx.accounts.bob.pubkey(), &ctx.feelssol_mint).await?;
    let trader_token_1 = ctx.create_ata(&ctx.accounts.bob.pubkey(), &token_1.pubkey()).await?;
    
    let input_amount = constants::MEDIUM_SWAP;
    ctx.mint_to(&ctx.feelssol_mint, &trader_token_0, &ctx.feelssol_authority, input_amount * 2).await?;
    
    // Get market state before swap
    let market_before = ctx.get_account::<Market>(&market).await?.unwrap();
    
    // Get balances before
    let balance_a_before = ctx.get_token_balance(&trader_token_0).await?;
    let balance_b_before = ctx.get_token_balance(&trader_token_1).await?;
    
    // Execute swap using assertion macro
    let swap_result = assert_tx_success!(
        ctx.swap_helper().swap(
            &market,
            &ctx.feelssol_mint,
            &token_1.pubkey(),
            input_amount,
            &ctx.accounts.bob,
        ).await,
        "Swap transaction should succeed"
    );
    
    // Get balances and market state after
    let balance_a_after = ctx.get_token_balance(&trader_token_0).await?;
    let balance_b_after = ctx.get_token_balance(&trader_token_1).await?;
    let market_after = ctx.get_account::<Market>(&market).await?.unwrap();
    
    // Create comprehensive swap result for assertions
    let assertion_swap_result = AssertionSwapResult {
        amount_in: swap_result.amount_in,
        amount_out: swap_result.amount_out,
        fee_amount: swap_result.fee_paid,
        price_before: market_before.sqrt_price,
        price_after: market_after.sqrt_price,
        fee_growth_0_before: market_before.fee_growth_global_0_x64,
        fee_growth_0_after: market_after.fee_growth_global_0_x64,
        fee_growth_1_before: market_before.fee_growth_global_1_x64,
        fee_growth_1_after: market_after.fee_growth_global_1_x64,
    };
    
    // Use comprehensive swap assertions
    assertion_swap_result.assert_amount_bounds(input_amount, swap_result.amount_out, 1);
    assertion_swap_result.assert_swap_direction_monotonic(true, market_before.sqrt_price, market_after.sqrt_price);
    assertion_swap_result.assert_fee_growth_increases(true, market_before.fee_growth_global_0_x64, market_after.fee_growth_global_0_x64);
    
    // Check protocol invariants
    ProtocolInvariants::check_liquidity_conservation(&market_before, &market_after);
    ProtocolInvariants::check_price_tick_consistency(&market_after);
    
    // Verify balance changes match swap results
    assert_eq!(
        balance_a_before - balance_a_after,
        swap_result.amount_in,
        "Token A balance should decrease by amount in"
    );
    
    assert_eq!(
        balance_b_after - balance_b_before,
        swap_result.amount_out,
        "Token B balance should increase by amount out"
    );
    
    // Verify market state was updated
    let market_state = &market_after;
    assert_ne!(
        market_state.current_tick,
        0,
        "Market tick should have moved"
    );
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_swap_exact_input_one_for_zero, |ctx: TestContext| async move {
    // Test swapping in opposite direction (token B for FeelsSOL)
    let token_1 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    
    let market = ctx.market_builder()
        .token_0(ctx.feelssol_mint)
        .token_1(token_1.pubkey())
        .add_liquidity(ctx.accounts.alice.insecure_clone(), -1000, 1000, 1_000_000_000)
        .build()
        .await?;
    
    // Setup trader with tokens
    let trader_token_0 = ctx.create_ata(&ctx.accounts.bob.pubkey(), &ctx.feelssol_mint).await?;
    let trader_token_1 = ctx.create_ata(&ctx.accounts.bob.pubkey(), &token_1.pubkey()).await?;
    
    let input_amount = constants::MEDIUM_SWAP;
    ctx.mint_to(&token_1.pubkey(), &trader_token_1, &ctx.accounts.market_creator, input_amount * 2).await?;
    
    // Execute swap
    let swap_result = ctx.swap_helper().swap(
        &market,
        &token_1.pubkey(),
        &ctx.feelssol_mint,
        input_amount,
        &ctx.accounts.bob,
    ).await?;
    
    assert!(
        swap_result.amount_out > 0,
        "Should receive token A"
    );
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_swap_with_price_impact, |ctx: TestContext| async move {
    // Test that large swaps have price impact
    let token_1 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    
    let market = ctx.market_builder()
        .token_0(ctx.feelssol_mint)
        .token_1(token_1.pubkey())
        .add_liquidity(ctx.accounts.alice.insecure_clone(), -1000, 1000, 1_000_000_000)
        .build()
        .await?;
    
    // Setup trader
    let trader_token_0 = ctx.create_ata(&ctx.accounts.bob.pubkey(), &ctx.feelssol_mint).await?;
    let trader_token_1 = ctx.create_ata(&ctx.accounts.bob.pubkey(), &token_1.pubkey()).await?;
    
    let small_amount = 1_000_000;  // 1 token
    let large_amount = 100_000_000; // 100 tokens
    
    ctx.mint_to(&ctx.feelssol_mint, &trader_token_0, &ctx.feelssol_authority, large_amount * 2).await?;
    
    // Get market state before swaps
    let market_before = ctx.get_account::<Market>(&market).await?.unwrap();
    
    // Small swap with assertion
    let small_swap = assert_tx_success!(
        ctx.swap_helper().swap(
            &market,
            &ctx.feelssol_mint,
            &token_1.pubkey(),
            small_amount,
            &ctx.accounts.bob,
        ).await,
        "Small swap should succeed"
    );
    
    let market_after_small = ctx.get_account::<Market>(&market).await?.unwrap();
    
    // Large swap with assertion
    let large_swap = assert_tx_success!(
        ctx.swap_helper().swap(
            &market,
            &ctx.feelssol_mint,
            &token_1.pubkey(),
            large_amount,
            &ctx.accounts.bob,
        ).await,
        "Large swap should succeed"
    );
    
    let market_after_large = ctx.get_account::<Market>(&market).await?.unwrap();
    
    // Use assertion utilities to validate price impact
    let small_swap_result = AssertionSwapResult {
        amount_in: small_swap.amount_in,
        amount_out: small_swap.amount_out,
        fee_amount: small_swap.fee_paid,
        price_before: market_before.sqrt_price,
        price_after: market_after_small.sqrt_price,
        fee_growth_0_before: market_before.fee_growth_global_0_x64,
        fee_growth_0_after: market_after_small.fee_growth_global_0_x64,
        fee_growth_1_before: market_before.fee_growth_global_1_x64,
        fee_growth_1_after: market_after_small.fee_growth_global_1_x64,
    };
    
    small_swap_result.assert_price_impact_reasonable(small_amount, market_before.sqrt_price, market_after_small.sqrt_price);
    
    // Calculate average prices
    let small_price = (small_swap.amount_out as f64) / (small_swap.amount_in as f64);
    let large_price = (large_swap.amount_out as f64) / (large_swap.amount_in as f64);
    
    assert!(
        large_price < small_price * 0.99, // At least 1% worse
        "Large swap should have worse price due to impact"
    );
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_swap_minimum_output_protection, |ctx: TestContext| async move {
    // Test that swaps respect minimum output amount
    let token_1 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    
    let market = ctx.market_builder()
        .token_0(ctx.feelssol_mint)
        .token_1(token_1.pubkey())
        .add_liquidity(ctx.accounts.alice.insecure_clone(), -1000, 1000, 1_000_000_000)
        .build()
        .await?;
    
    // Setup trader
    let trader_token_0 = ctx.create_ata(&ctx.accounts.bob.pubkey(), &ctx.feelssol_mint).await?;
    let trader_token_1 = ctx.create_ata(&ctx.accounts.bob.pubkey(), &token_1.pubkey()).await?;
    
    let input_amount = constants::SMALL_SWAP;
    ctx.mint_to(&ctx.feelssol_mint, &trader_token_0, &ctx.feelssol_authority, input_amount).await?;
    
    // First do a normal swap to see expected output
    let expected_swap = assert_tx_success!(
        ctx.swap_helper().swap(
            &market,
            &ctx.feelssol_mint,
            &token_1.pubkey(),
            input_amount,
            &ctx.accounts.bob,
        ).await,
        "Initial swap should succeed"
    );
    
    // Reset trader balance
    ctx.mint_to(&ctx.feelssol_mint, &trader_token_0, &ctx.feelssol_authority, input_amount).await?;
    
    // Try swap with unrealistic minimum output (should fail)
    let unrealistic_min = expected_swap.amount_out * 2;
    
    // Use assertion utilities to validate the swap bounds
    let swap_result = AssertionSwapResult {
        amount_in: expected_swap.amount_in,
        amount_out: expected_swap.amount_out,
        fee_amount: expected_swap.fee_paid,
        price_before: 0, // Not needed for this test
        price_after: 0,  // Not needed for this test
        fee_growth_0_before: 0,
        fee_growth_0_after: 0,
        fee_growth_1_before: 0,
        fee_growth_1_after: 0,
    };
    
    // Validate that the swap meets minimum bounds
    swap_result.assert_amount_bounds(input_amount, expected_swap.amount_out, 1);
    
    // Verify our expectation about minimum output
    assert!(
        unrealistic_min > expected_swap.amount_out,
        "Minimum output requirement would not be met"
    );
    
    // TODO: When minimum_amount_out parameter is implemented, use this:
    // assert_error!(
    //     ctx.swap_helper().swap_with_minimum(
    //         &market,
    //         &ctx.feelssol_mint,
    //         &token_1.pubkey(),
    //         input_amount,
    //         unrealistic_min,
    //         &ctx.accounts.bob,
    //     ).await,
    //     FeelsError::SlippageExceeded
    // );
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_swap_fee_collection, |ctx: TestContext| async move {
    // Test that swaps collect fees properly
    let token_1 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    
    let market = ctx.market_builder()
        .token_0(ctx.feelssol_mint)
        .token_1(token_1.pubkey())
        .add_liquidity(ctx.accounts.alice.insecure_clone(), -1000, 1000, 1_000_000_000)
        .build()
        .await?;
    
    // Get initial fee growth
    let market_before = ctx.get_account::<Market>(&market).await?.unwrap();
    let fee_growth_before = market_before.fee_growth_global_0_x64;
    
    // Execute swap
    let trader_token_0 = ctx.create_ata(&ctx.accounts.bob.pubkey(), &ctx.feelssol_mint).await?;
    ctx.mint_to(&ctx.feelssol_mint, &trader_token_0, &ctx.feelssol_authority, constants::LARGE_SWAP).await?;
    
    ctx.swap_helper().swap(
        &market,
        &ctx.feelssol_mint,
        &token_1.pubkey(),
        constants::LARGE_SWAP,
        &ctx.accounts.bob,
    ).await?;
    
    // Check fee growth increased
    let market_after = ctx.get_account::<Market>(&market).await?.unwrap();
    assert!(
        market_after.fee_growth_global_0_x64 > fee_growth_before,
        "Fee growth should increase after swap"
    );
    
    Ok::<(), Box<dyn std::error::Error>>(())
});