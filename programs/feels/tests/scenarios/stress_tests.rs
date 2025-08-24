/// Stress test scenarios using SDK and simulation framework
/// Tests protocol limits, edge cases, and performance under load

use anchor_lang::prelude::*;
use feels_sdk::FeelsClient;
use feels_simulation::{ScenarioRunner, TestEnvironment};
use rand::{thread_rng, Rng};
use std::time::Instant;

#[tokio::test]
async fn test_maximum_tick_crossing() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize and create pool
    runner.initialize_protocol().await.unwrap();
    let pool = runner.create_basic_pool().await.unwrap();
    
    // Add liquidity at every possible tick range (within reason)
    let tick_spacing = 60;
    let num_ranges = 100;
    
    for i in 0..num_ranges {
        let tick_lower = -30000 + (i * tick_spacing * 2);
        let tick_upper = tick_lower + tick_spacing;
        
        runner.env.client
            .add_liquidity(
                pool.pool_address,
                100_000, // Small liquidity per range
                tick_lower,
                tick_upper,
                u64::MAX,
                u64::MAX,
            )
            .await
            .unwrap();
    }
    
    // Execute massive swap that crosses many ticks
    let start = Instant::now();
    
    let mega_swap = runner.env.client
        .swap(
            pool.pool_address,
            10_000_000, // Large swap
            0,
            true,
            None,
        )
        .await
        .unwrap();
    
    let duration = start.elapsed();
    
    // Should complete within reasonable time
    assert!(duration.as_secs() < 10);
    
    // Verify swap crossed multiple ticks
    let pool_after = runner.env.client
        .get_pool_info(pool.pool_address)
        .await
        .unwrap();
    
    assert!(pool_after.current_tick > 1000); // Significant price movement
}

#[tokio::test]
async fn test_maximum_concurrent_operations() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Set up pool with liquidity
    let scenario = runner.run_basic_amm_scenario().await.unwrap();
    
    // Create many accounts
    let num_users = 100;
    let accounts = runner.env.account_factory
        .create_funded_accounts(num_users)
        .await
        .unwrap();
    
    // Launch all operations concurrently
    let mut handles = vec![];
    let client = std::sync::Arc::new(runner.env.client.clone());
    let pool_address = scenario.pool_address;
    
    for (i, account) in accounts.iter().enumerate() {
        let client_clone = client.clone();
        let keypair = account.clone();
        
        let handle = tokio::spawn(async move {
            let operation = i % 4;
            match operation {
                0 => {
                    // Swap
                    client_clone
                        .with_payer(&keypair)
                        .swap(pool_address, 1000, 900, true, None)
                        .await
                }
                1 => {
                    // Add liquidity
                    client_clone
                        .with_payer(&keypair)
                        .add_liquidity(
                            pool_address,
                            10_000,
                            -100,
                            100,
                            u64::MAX,
                            u64::MAX,
                        )
                        .await
                        .map(|_| ())
                }
                2 => {
                    // Quote swap
                    client_clone
                        .quote_swap(pool_address, 1000, true)
                        .await
                        .map(|_| ())
                }
                _ => {
                    // Get pool info
                    client_clone
                        .get_pool_info(pool_address)
                        .await
                        .map(|_| ())
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations
    let mut successful = 0;
    let mut failed = 0;
    
    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => successful += 1,
            _ => failed += 1,
        }
    }
    
    // Most operations should succeed
    assert!(successful > 80);
    println!("Concurrent operations: {} successful, {} failed", successful, failed);
}

#[tokio::test]
async fn test_extreme_price_ranges() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize and create pool
    runner.initialize_protocol().await.unwrap();
    let pool = runner.create_basic_pool().await.unwrap();
    
    // Add liquidity at extreme tick ranges
    let extreme_cases = vec![
        (-440000, -430000), // Near minimum
        (430000, 440000),   // Near maximum
        (-100000, 100000),  // Very wide range
        (-10, 10),          // Very narrow range
    ];
    
    for (lower, upper) in extreme_cases {
        let result = runner.env.client
            .add_liquidity(
                pool.pool_address,
                1_000_000,
                lower,
                upper,
                u64::MAX,
                u64::MAX,
            )
            .await;
        
        // Should handle extreme ranges
        assert!(result.is_ok());
    }
    
    // Test swaps at extreme prices
    // Push price very low
    for _ in 0..50 {
        runner.env.client
            .swap(
                pool.pool_address,
                100_000,
                0,
                false, // Sell
                None,
            )
            .await
            .ok(); // Some might fail at extremes
    }
    
    // Push price very high
    for _ in 0..50 {
        runner.env.client
            .swap(
                pool.pool_address,
                100_000,
                0,
                true, // Buy
                None,
            )
            .await
            .ok();
    }
    
    // Pool should still be functional
    let pool_info = runner.env.client
        .get_pool_info(pool.pool_address)
        .await
        .unwrap();
    
    assert!(pool_info.current_tick >= -443636 && pool_info.current_tick <= 443636);
}

#[tokio::test]
async fn test_random_chaos_operations() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Set up multiple pools
    runner.initialize_protocol().await.unwrap();
    
    let mut pools = vec![];
    for i in 0..3 {
        let token = runner.env.token_factory
            .create_test_token(&format!("CHAOS{}", i), 9)
            .await
            .unwrap();
        
        let pool = runner.env.pool_factory
            .create_pool(token.mint, 30)
            .await
            .unwrap();
        
        // Add initial liquidity
        runner.env.liquidity_simulator
            .add_balanced_liquidity(pool.address, 10_000_000)
            .await
            .unwrap();
        
        pools.push(pool);
    }
    
    // Create chaos agents
    let num_agents = 20;
    let agents = runner.env.account_factory
        .create_funded_accounts(num_agents)
        .await
        .unwrap();
    
    // Run random operations for a period
    let duration_secs = 5;
    let start = Instant::now();
    let mut rng = thread_rng();
    
    let mut total_operations = 0;
    let mut successful_operations = 0;
    
    while start.elapsed().as_secs() < duration_secs {
        let agent = &agents[rng.gen_range(0..agents.len())];
        let pool = &pools[rng.gen_range(0..pools.len())];
        let operation = rng.gen_range(0..100);
        
        total_operations += 1;
        
        let result = match operation {
            0..=40 => {
                // 40% swaps
                let amount = rng.gen_range(100..10_000);
                let is_buy = rng.gen_bool(0.5);
                
                runner.env.client
                    .with_payer(agent)
                    .swap(pool.address, amount, 0, is_buy, None)
                    .await
                    .map(|_| ())
            }
            41..=60 => {
                // 20% add liquidity
                let tick_lower = rng.gen_range(-5000..0) / 60 * 60;
                let tick_upper = rng.gen_range(60..5000) / 60 * 60;
                let liquidity = rng.gen_range(10_000..100_000);
                
                runner.env.client
                    .with_payer(agent)
                    .add_liquidity(
                        pool.address,
                        liquidity,
                        tick_lower,
                        tick_upper,
                        u64::MAX,
                        u64::MAX,
                    )
                    .await
                    .map(|_| ())
            }
            61..=80 => {
                // 20% quotes
                let amount = rng.gen_range(100..10_000);
                let is_buy = rng.gen_bool(0.5);
                
                runner.env.client
                    .quote_swap(pool.address, amount, is_buy)
                    .await
                    .map(|_| ())
            }
            _ => {
                // 20% pool info queries
                runner.env.client
                    .get_pool_info(pool.address)
                    .await
                    .map(|_| ())
            }
        };
        
        if result.is_ok() {
            successful_operations += 1;
        }
        
        // Small delay to prevent overwhelming
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    
    // System should remain stable under chaos
    let success_rate = successful_operations as f64 / total_operations as f64;
    assert!(success_rate > 0.8); // 80% success rate under chaos
    
    println!(
        "Chaos test: {} operations, {:.1}% success rate",
        total_operations,
        success_rate * 100.0
    );
    
    // Verify all pools still functional
    for pool in &pools {
        let pool_info = runner.env.client
            .get_pool_info(pool.address)
            .await
            .unwrap();
        
        assert!(pool_info.liquidity > 0);
        assert!(pool_info.sqrt_price > 0);
    }
}

#[tokio::test]
async fn test_gas_optimization_scenarios() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Set up pool
    let scenario = runner.run_basic_amm_scenario().await.unwrap();
    
    // Measure gas for different operations
    let operations = vec![
        ("Simple Swap", async {
            runner.env.client
                .swap(scenario.pool_address, 1000, 900, true, None)
                .await
        }),
        ("Cross-Tick Swap", async {
            runner.env.client
                .swap(scenario.pool_address, 100_000, 0, true, None)
                .await
        }),
        ("Add Liquidity", async {
            runner.env.client
                .add_liquidity(
                    scenario.pool_address,
                    100_000,
                    -1000,
                    1000,
                    u64::MAX,
                    u64::MAX,
                )
                .await
                .map(|_| ())
        }),
        ("Remove Liquidity", async {
            let pos = runner.env.client
                .add_liquidity(
                    scenario.pool_address,
                    100_000,
                    -100,
                    100,
                    u64::MAX,
                    u64::MAX,
                )
                .await
                .unwrap();
            
            runner.env.client
                .remove_liquidity(pos.position_mint, 100_000, 0, 0)
                .await
                .map(|_| ())
        }),
    ];
    
    for (name, operation) in operations {
        let start = Instant::now();
        let result = operation.await;
        let duration = start.elapsed();
        
        assert!(result.is_ok());
        println!("{}: {:?}", name, duration);
        
        // Operations should complete quickly
        assert!(duration.as_millis() < 1000);
    }
}

#[tokio::test]
async fn test_position_limit_stress() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Set up pool
    runner.initialize_protocol().await.unwrap();
    let pool = runner.create_basic_pool().await.unwrap();
    
    // Create many positions from single account
    let whale = runner.env.account_factory
        .create_funded_account()
        .await
        .unwrap();
    
    let mut positions = vec![];
    let max_positions = 50;
    
    // Add many positions
    for i in 0..max_positions {
        let tick_lower = -1000 - (i as i32 * 60);
        let tick_upper = 1000 + (i as i32 * 60);
        
        match runner.env.client
            .with_payer(&whale)
            .add_liquidity(
                pool.pool_address,
                100_000,
                tick_lower,
                tick_upper,
                u64::MAX,
                u64::MAX,
            )
            .await
        {
            Ok(pos) => positions.push(pos),
            Err(_) => break, // Hit some limit
        }
    }
    
    println!("Created {} positions", positions.len());
    assert!(positions.len() >= 20); // Should handle many positions
    
    // Collect fees from all positions
    let mut total_fees = 0u64;
    
    // Generate fees
    for _ in 0..100 {
        runner.env.client
            .swap(pool.pool_address, 1000, 0, true, None)
            .await
            .unwrap();
    }
    
    // Collect from each position
    for position in &positions {
        if let Ok(collected) = runner.env.client
            .with_payer(&whale)
            .collect_position_fees(position.position_mint, u64::MAX, u64::MAX)
            .await
        {
            total_fees += collected.amount_0 + collected.amount_1;
        }
    }
    
    assert!(total_fees > 0);
}

#[tokio::test]
async fn test_protocol_fee_stress() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize with high protocol fee
    runner.initialize_protocol().await.unwrap();
    runner.env.client
        .update_protocol_fee_rate(5000) // 50% protocol fee
        .await
        .unwrap();
    
    // Create multiple pools
    let mut pools = vec![];
    for i in 0..5 {
        let token = runner.env.token_factory
            .create_test_token(&format!("FEE{}", i), 9)
            .await
            .unwrap();
        
        let pool = runner.env.pool_factory
            .create_pool(token.mint, 30)
            .await
            .unwrap();
        
        runner.env.liquidity_simulator
            .add_balanced_liquidity(pool.address, 10_000_000)
            .await
            .unwrap();
        
        pools.push(pool);
    }
    
    // Generate high volume across all pools
    let traders = runner.env.account_factory
        .create_traders(10)
        .await
        .unwrap();
    
    for _ in 0..20 {
        for pool in &pools {
            for trader in &traders {
                runner.env.client
                    .with_payer(&trader.keypair)
                    .swap(pool.address, 10_000, 0, true, None)
                    .await
                    .ok();
            }
        }
    }
    
    // Collect protocol fees from all pools
    let mut total_protocol_fees = 0u64;
    
    for pool in &pools {
        if let Ok(collected) = runner.env.client
            .collect_protocol_fees(pool.address, u64::MAX, u64::MAX)
            .await
        {
            total_protocol_fees += collected.amount_0 + collected.amount_1;
        }
    }
    
    // Should have accumulated significant protocol fees
    assert!(total_protocol_fees > 100_000);
    
    // Verify protocol fee accounting is correct
    for pool in &pools {
        let pool_info = runner.env.client
            .get_pool_info(pool.address)
            .await
            .unwrap();
        
        // Protocol fees should be cleared after collection
        assert_eq!(pool_info.protocol_fees_0, 0);
        assert_eq!(pool_info.protocol_fees_1, 0);
    }
}