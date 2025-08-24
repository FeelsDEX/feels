/// AMM operation tests using SDK and simulation framework
/// Tests liquidity provision, swaps, and complex trading scenarios

use anchor_lang::prelude::*;
use feels_sdk::{FeelsClient, SwapResult};
use feels_simulation::{ScenarioRunner, TestEnvironment};

#[tokio::test]
async fn test_liquidity_provision_lifecycle() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Run basic setup
    runner.initialize_protocol().await.unwrap();
    let pool = runner.create_basic_pool().await.unwrap();
    
    // Test adding liquidity at different price ranges
    let test_cases = vec![
        (-2000, -1000, 1_000_000), // Below current price
        (-500, 500, 2_000_000),    // Around current price
        (1000, 2000, 1_500_000),   // Above current price
    ];
    
    let mut positions = vec![];
    
    for (tick_lower, tick_upper, liquidity) in test_cases {
        let result = runner.env.client
            .add_liquidity(
                pool.pool_address,
                liquidity,
                tick_lower,
                tick_upper,
                u64::MAX, // Max slippage for test
                u64::MAX,
            )
            .await
            .unwrap();
        
        positions.push(result);
        
        // Verify position was created correctly
        let position_info = runner.env.client
            .get_position_info(result.position_mint)
            .await
            .unwrap();
        
        assert_eq!(position_info.liquidity, liquidity);
        assert_eq!(position_info.tick_lower, tick_lower);
        assert_eq!(position_info.tick_upper, tick_upper);
    }
    
    // Verify pool liquidity updated
    let pool_info = runner.env.client
        .get_pool_info(pool.pool_address)
        .await
        .unwrap();
    
    // Only the position around current price should contribute to active liquidity
    assert_eq!(pool_info.liquidity, 2_000_000);
    
    // Remove liquidity from one position
    let remove_result = runner.env.client
        .remove_liquidity(
            positions[1].position_mint,
            1_000_000, // Remove half
            0,
            0,
        )
        .await
        .unwrap();
    
    assert!(remove_result.amount_0 > 0 || remove_result.amount_1 > 0);
    
    // Verify pool liquidity decreased
    let updated_pool = runner.env.client
        .get_pool_info(pool.pool_address)
        .await
        .unwrap();
    
    assert_eq!(updated_pool.liquidity, 1_000_000);
}

#[tokio::test]
async fn test_swap_execution_scenarios() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Set up pool with liquidity
    let scenario = runner.run_basic_amm_scenario().await.unwrap();
    
    // Test various swap scenarios
    struct SwapTest {
        amount_in: u64,
        is_buy: bool,
        expected_min_out: u64,
    }
    
    let swap_tests = vec![
        SwapTest { amount_in: 1_000, is_buy: true, expected_min_out: 950 },
        SwapTest { amount_in: 10_000, is_buy: false, expected_min_out: 9_500 },
        SwapTest { amount_in: 100_000, is_buy: true, expected_min_out: 90_000 },
    ];
    
    for test in swap_tests {
        let result = runner.env.client
            .swap(
                scenario.pool_address,
                test.amount_in,
                test.expected_min_out,
                test.is_buy,
                None,
            )
            .await
            .unwrap();
        
        // Verify swap executed correctly
        assert!(result.amount_out >= test.expected_min_out);
        assert_eq!(result.fee, (test.amount_in as u128 * 30 / 10_000) as u64);
        
        // Verify price moved in expected direction
        let pool_after = runner.env.client
            .get_pool_info(scenario.pool_address)
            .await
            .unwrap();
        
        if test.is_buy {
            assert!(pool_after.sqrt_price > scenario.initial_sqrt_price);
        } else {
            assert!(pool_after.sqrt_price < scenario.initial_sqrt_price);
        }
    }
}

#[tokio::test]
async fn test_cross_tick_swaps() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize and create pool
    runner.initialize_protocol().await.unwrap();
    let pool = runner.create_basic_pool().await.unwrap();
    
    // Add liquidity at multiple tick ranges
    let tick_spacing = 60;
    let ranges = vec![
        (-10 * tick_spacing, -5 * tick_spacing, 500_000),
        (-5 * tick_spacing, 0, 1_000_000),
        (0, 5 * tick_spacing, 1_500_000),
        (5 * tick_spacing, 10 * tick_spacing, 500_000),
    ];
    
    for (lower, upper, liquidity) in ranges {
        runner.env.client
            .add_liquidity(
                pool.pool_address,
                liquidity,
                lower,
                upper,
                u64::MAX,
                u64::MAX,
            )
            .await
            .unwrap();
    }
    
    // Execute large swap that crosses multiple ticks
    let large_swap = runner.env.client
        .swap(
            pool.pool_address,
            500_000,
            0, // Accept any output
            true,
            None,
        )
        .await
        .unwrap();
    
    // Verify swap crossed ticks
    let pool_after = runner.env.client
        .get_pool_info(pool.pool_address)
        .await
        .unwrap();
    
    assert!(pool_after.current_tick > 0); // Price moved up significantly
    
    // Execute swap in opposite direction
    let reverse_swap = runner.env.client
        .swap(
            pool.pool_address,
            large_swap.amount_out,
            0,
            false,
            None,
        )
        .await
        .unwrap();
    
    // Price should return close to original
    let final_pool = runner.env.client
        .get_pool_info(pool.pool_address)
        .await
        .unwrap();
    
    assert!(final_pool.current_tick.abs() < tick_spacing);
}

#[tokio::test]
async fn test_slippage_protection() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Set up pool
    let scenario = runner.run_basic_amm_scenario().await.unwrap();
    
    // Test swap with tight slippage - should fail
    let tight_slippage = runner.env.client
        .swap(
            scenario.pool_address,
            100_000,
            99_900, // Only 0.1% slippage allowed
            true,
            None,
        )
        .await;
    
    assert!(tight_slippage.is_err());
    
    // Test with price limit
    let current_price = runner.env.client
        .get_pool_info(scenario.pool_address)
        .await
        .unwrap()
        .sqrt_price;
    
    let price_limit = (current_price as f64 * 1.01) as u128; // 1% price movement limit
    
    let limited_swap = runner.env.client
        .swap(
            scenario.pool_address,
            1_000_000, // Large swap
            0,
            true,
            Some(price_limit),
        )
        .await
        .unwrap();
    
    // Verify price didn't exceed limit
    let pool_after = runner.env.client
        .get_pool_info(scenario.pool_address)
        .await
        .unwrap();
    
    assert!(pool_after.sqrt_price <= price_limit);
}

#[tokio::test]
async fn test_fee_accumulation_and_collection() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Set up pool with liquidity
    let scenario = runner.run_basic_amm_scenario().await.unwrap();
    
    // Create a position to track fees
    let position = runner.env.client
        .add_liquidity(
            scenario.pool_address,
            1_000_000,
            -1000,
            1000,
            u64::MAX,
            u64::MAX,
        )
        .await
        .unwrap();
    
    // Execute many swaps to generate fees
    for i in 0..50 {
        let is_buy = i % 2 == 0;
        runner.env.client
            .swap(
                scenario.pool_address,
                10_000,
                9_000,
                is_buy,
                None,
            )
            .await
            .unwrap();
    }
    
    // Check position fees owed
    let fees_owed = runner.env.client
        .get_position_fees_owed(position.position_mint)
        .await
        .unwrap();
    
    assert!(fees_owed.amount_0 > 0 || fees_owed.amount_1 > 0);
    
    // Collect position fees
    let collected = runner.env.client
        .collect_position_fees(position.position_mint, u64::MAX, u64::MAX)
        .await
        .unwrap();
    
    assert_eq!(collected.amount_0, fees_owed.amount_0);
    assert_eq!(collected.amount_1, fees_owed.amount_1);
    
    // Check protocol fees accumulated
    let pool_info = runner.env.client
        .get_pool_info(scenario.pool_address)
        .await
        .unwrap();
    
    assert!(pool_info.protocol_fees_0 > 0 || pool_info.protocol_fees_1 > 0);
}

#[tokio::test]
async fn test_cross_token_routing() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize protocol
    runner.initialize_protocol().await.unwrap();
    
    // Create two test tokens and pools
    let usdc = runner.env.token_factory
        .create_test_token("USDC", 6)
        .await
        .unwrap();
    
    let pepe = runner.env.token_factory
        .create_test_token("PEPE", 9)
        .await
        .unwrap();
    
    // Create pools
    let usdc_pool = runner.env.pool_factory
        .create_pool(usdc.mint, 30)
        .await
        .unwrap();
    
    let pepe_pool = runner.env.pool_factory
        .create_pool(pepe.mint, 30)
        .await
        .unwrap();
    
    // Add liquidity to both pools
    runner.env.liquidity_simulator
        .add_balanced_liquidity(usdc_pool.address, 10_000_000)
        .await
        .unwrap();
    
    runner.env.liquidity_simulator
        .add_balanced_liquidity(pepe_pool.address, 10_000_000)
        .await
        .unwrap();
    
    // Execute cross-token swap (USDC -> PEPE)
    let route_swap = runner.env.client
        .swap_cross_token(
            usdc.mint,
            pepe.mint,
            100_000,
            90_000,
            None,
            None,
        )
        .await
        .unwrap();
    
    // Verify two-hop execution
    assert_eq!(route_swap.route_type, "TwoHop");
    assert!(route_swap.intermediate_amount > 0);
    assert!(route_swap.final_amount >= 90_000);
    
    // Verify both pools were used
    let usdc_pool_after = runner.env.client
        .get_pool_info(usdc_pool.address)
        .await
        .unwrap();
    
    let pepe_pool_after = runner.env.client
        .get_pool_info(pepe_pool.address)
        .await
        .unwrap();
    
    assert!(usdc_pool_after.total_volume_0 > 0);
    assert!(pepe_pool_after.total_volume_1 > 0);
}

#[tokio::test]
async fn test_liquidity_migration() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Set up initial pool
    runner.initialize_protocol().await.unwrap();
    let pool = runner.create_basic_pool().await.unwrap();
    
    // Add initial liquidity position
    let initial_position = runner.env.client
        .add_liquidity(
            pool.pool_address,
            1_000_000,
            -1000,
            1000,
            u64::MAX,
            u64::MAX,
        )
        .await
        .unwrap();
    
    // Execute some swaps to accumulate fees
    for _ in 0..10 {
        runner.env.client
            .swap(pool.pool_address, 10_000, 9_000, true, None)
            .await
            .unwrap();
    }
    
    // Collect fees before migration
    let fees = runner.env.client
        .collect_position_fees(initial_position.position_mint, u64::MAX, u64::MAX)
        .await
        .unwrap();
    
    // Remove liquidity
    let removed = runner.env.client
        .remove_liquidity(
            initial_position.position_mint,
            1_000_000,
            0,
            0,
        )
        .await
        .unwrap();
    
    // Add liquidity to new range
    let new_position = runner.env.client
        .add_liquidity(
            pool.pool_address,
            1_000_000,
            -2000,
            2000,
            removed.amount_0 + fees.amount_0,
            removed.amount_1 + fees.amount_1,
        )
        .await
        .unwrap();
    
    // Verify migration successful
    let position_info = runner.env.client
        .get_position_info(new_position.position_mint)
        .await
        .unwrap();
    
    assert_eq!(position_info.liquidity, 1_000_000);
    assert_eq!(position_info.tick_lower, -2000);
    assert_eq!(position_info.tick_upper, 2000);
}