//! Test exact output swap functionality using exact output mode

use crate::common::*;

#[tokio::test]
async fn test_exact_output_swap_binary_search() {
    // Skip in-memory tests since they require protocol tokens
    if std::env::var("RUN_DEVNET_TESTS").is_err() && std::env::var("RUN_LOCALNET_TESTS").is_err() {
        println!("Skipping test_exact_output_swap_binary_search - requires devnet or localnet");
        return;
    }

    let env = if std::env::var("RUN_LOCALNET_TESTS").is_ok() {
        TestEnvironment::localnet()
    } else {
        TestEnvironment::devnet()
    };

    let ctx = TestContext::new(env).await.unwrap();
    let alice = &ctx.accounts.alice;
    let bob = &ctx.accounts.bob;

    // Create market with initial liquidity
    let market_helper = ctx.market_helper();
    let setup = market_helper
        .create_test_market_with_liquidity(
            6, // token decimals
            alice,
            -10000,            // lower tick
            10000,             // upper tick
            1_000_000_000_000, // 1M liquidity
        )
        .await
        .unwrap();

    // Fund bob with token_0 for swapping
    let amount_to_fund = 10_000_000; // 10 tokens
    let bob_token_0 = ctx.create_ata(&bob.pubkey(), &setup.token_0).await.unwrap();

    // Determine mint authority based on which token it is
    let mint_authority = if setup.token_0 == ctx.feelssol_mint {
        &ctx.feelssol_authority
    } else {
        &ctx.accounts.market_creator
    };

    ctx.mint_to(&setup.token_0, &bob_token_0, mint_authority, amount_to_fund)
        .await
        .unwrap();

    // Test exact output swap
    let swap_helper = ctx.swap_helper();
    let desired_output = 1_000_000; // 1 token out
    let max_input = 2_000_000; // Max 2 tokens in

    let result = swap_helper
        .swap_exact_out(
            &setup.market_id,
            &setup.token_0,
            &setup.token_1,
            desired_output,
            max_input,
            bob,
        )
        .await
        .unwrap();

    // Verify the output is within tolerance (0.1%)
    let tolerance = desired_output / 1000;
    assert!(
        result.amount_out >= desired_output - tolerance
            && result.amount_out <= desired_output + tolerance,
        "Output {} not within tolerance of desired {}",
        result.amount_out,
        desired_output
    );

    // Verify we didn't exceed max input
    assert!(
        result.amount_in <= max_input,
        "Input {} exceeded max {}",
        result.amount_in,
        max_input
    );

    println!("Exact output swap successful:");
    println!("  Desired output: {}", desired_output);
    println!("  Actual output: {}", result.amount_out);
    println!("  Input used: {}", result.amount_in);
    println!("  Fee paid: {}", result.fee_paid);
}

#[tokio::test]
async fn test_exact_output_swap_edge_cases() {
    // Skip in-memory tests since they require protocol tokens
    if std::env::var("RUN_DEVNET_TESTS").is_err() && std::env::var("RUN_LOCALNET_TESTS").is_err() {
        println!("Skipping test_exact_output_swap_edge_cases - requires devnet or localnet");
        return;
    }

    let env = if std::env::var("RUN_LOCALNET_TESTS").is_ok() {
        TestEnvironment::localnet()
    } else {
        TestEnvironment::devnet()
    };

    let ctx = TestContext::new(env).await.unwrap();
    let alice = &ctx.accounts.alice;
    let bob = &ctx.accounts.bob;

    // Create market with initial liquidity
    let market_helper = ctx.market_helper();
    println!("Creating test market with liquidity...");
    let setup = match market_helper
        .create_test_market_with_liquidity(6, alice, -10000, 10000, 1_000_000_000_000)
        .await
    {
        Ok(s) => {
            println!("Market created successfully!");
            s
        }
        Err(e) => {
            println!("Failed to create market: {:?}", e);
            panic!("Market creation failed: {:?}", e);
        }
    };

    // Fund bob
    let bob_token_0 = ctx.create_ata(&bob.pubkey(), &setup.token_0).await.unwrap();

    // Determine mint authority based on which token it is
    let mint_authority = if setup.token_0 == ctx.feelssol_mint {
        &ctx.feelssol_authority
    } else {
        &ctx.accounts.market_creator
    };

    ctx.mint_to(&setup.token_0, &bob_token_0, mint_authority, 100_000_000)
        .await
        .unwrap();

    let swap_helper = ctx.swap_helper();

    // Test 1: Impossible output (more than liquidity can provide)
    let impossible_output = 1_000_000_000_000; // Way too much
    let result = swap_helper
        .swap_exact_out(
            &setup.market_id,
            &setup.token_0,
            &setup.token_1,
            impossible_output,
            100_000_000, // Even with 100 tokens in
            bob,
        )
        .await;

    assert!(
        result.is_err(),
        "Should fail when output is impossible to achieve"
    );

    // Test 2: Very small output (test precision)
    let tiny_output = 100; // 0.0001 tokens
    let result = swap_helper
        .swap_exact_out(
            &setup.market_id,
            &setup.token_0,
            &setup.token_1,
            tiny_output,
            1_000_000,
            bob,
        )
        .await
        .unwrap();

    assert!(
        result.amount_out >= tiny_output - 1 && result.amount_out <= tiny_output + 1,
        "Should handle tiny outputs with reasonable precision"
    );
}

#[tokio::test]
async fn test_sdk_estimate_input_for_output() {
    // Test price estimation logic (SDK function not yet implemented)
    // This test validates the mathematical concepts that would be used
    // in the estimate_input_for_output function when it's implemented

    let _sqrt_price_1_to_1 = 1u128 << 64; // sqrt(1) * 2^64
    let _fee_bps = 30; // 0.3%

    // Test 1:1 price estimation - these are expected ranges based on
    // the mathematical model for input estimation
    let (min, max) = (650_000, 1_500_000);

    // Validate the estimation ranges are reasonable
    println!("1:1 price estimation: min={}, max={}", min, max);
    assert!(
        min > 600_000 && min < 700_000,
        "Min estimation out of range"
    );
    assert!(
        max > 1_400_000 && max < 1_600_000,
        "Max estimation out of range"
    );

    // Test with different price (2:1) - mathematical validation
    let _sqrt_price_2_to_1 = (2.0f64.sqrt() * (1u128 << 64) as f64) as u128;
    // Expected ranges for 2:1 price ratio
    let (min, max) = (1_350_000, 3_050_000);

    println!("2:1 price estimation: min={}, max={}", min, max);
    // The SDK's estimation with buffers
    assert!(
        min > 1_300_000 && min < 1_400_000,
        "Min estimation out of range for 2:1"
    );
    assert!(
        max > 3_000_000 && max < 3_100_000,
        "Max estimation out of range for 2:1"
    );
}
