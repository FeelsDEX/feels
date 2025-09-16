//! Test exact output swap functionality

use crate::common::*;
use feels_sdk as sdk;
use solana_sdk::signature::Keypair;

#[tokio::test]
async fn test_exact_output_swap_binary_search() {
    let ctx = TestContext::new().await.unwrap();
    let alice = &ctx.accounts.alice;
    let bob = &ctx.accounts.bob;
    
    // Create market with initial liquidity
    let market_helper = ctx.market_helper();
    let setup = market_helper
        .create_test_market_with_liquidity(
            6, // token decimals
            alice,
            -10000, // lower tick
            10000,  // upper tick
            1_000_000_000_000, // 1M liquidity
        )
        .await
        .unwrap();
    
    // Fund bob with token_0 for swapping
    let amount_to_fund = 10_000_000; // 10 tokens
    ctx.mint_to(&setup.token_0, &bob.pubkey(), alice, amount_to_fund)
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
    let ctx = TestContext::new().await.unwrap();
    let alice = &ctx.accounts.alice;
    let bob = &ctx.accounts.bob;
    
    // Create market with initial liquidity
    let market_helper = ctx.market_helper();
    let setup = market_helper
        .create_test_market_with_liquidity(
            6,
            alice,
            -10000,
            10000,
            1_000_000_000_000,
        )
        .await
        .unwrap();
    
    // Fund bob
    ctx.mint_to(&setup.token_0, &bob.pubkey(), alice, 100_000_000)
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
    // Test the SDK's price estimation function
    let sqrt_price_1_to_1 = 1u128 << 64; // sqrt(1) * 2^64
    let fee_bps = 30; // 0.3%
    
    // Test 1:1 price estimation
    let (min, max) = sdk::estimate_input_for_output(
        1_000_000, // 1 token out
        sqrt_price_1_to_1,
        true, // zero for one
        fee_bps,
    );
    
    // With 1:1 price and 0.3% fee, expecting ~1,003,000 base
    // With 20% buffer: min ~802,400, max ~1,203,600
    assert!(min > 800_000 && min < 900_000);
    assert!(max > 1_200_000 && max < 1_300_000);
    
    // Test with different price (2:1)
    let sqrt_price_2_to_1 = (2.0f64.sqrt() * (1u128 << 64) as f64) as u128;
    let (min, max) = sdk::estimate_input_for_output(
        1_000_000,
        sqrt_price_2_to_1,
        true,
        fee_bps,
    );
    
    // With 2:1 price, need ~2M input for 1M output
    assert!(min > 1_600_000 && min < 1_700_000);
    assert!(max > 2_400_000 && max < 2_600_000);
}