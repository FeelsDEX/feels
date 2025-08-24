/// Multi-user scenario tests using SDK and simulation framework
/// Tests concurrent operations, MEV scenarios, and complex market dynamics

use anchor_lang::prelude::*;
use feels_sdk::FeelsClient;
use feels_simulation::{ScenarioRunner, TestEnvironment};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_concurrent_liquidity_providers() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize and create pool
    runner.initialize_protocol().await.unwrap();
    let pool = runner.create_basic_pool().await.unwrap();
    
    // Create multiple LPs
    let lp_count = 10;
    let lps = runner.env.account_factory
        .create_liquidity_providers(lp_count)
        .await
        .unwrap();
    
    // Each LP adds liquidity concurrently
    let pool_address = pool.pool_address;
    let client = Arc::new(runner.env.client.clone());
    let mut handles = vec![];
    
    for (i, lp) in lps.iter().enumerate() {
        let client_clone = client.clone();
        let lp_keypair = lp.keypair.clone();
        
        // Each LP targets different tick ranges
        let tick_lower = -1000 + (i as i32 * 100);
        let tick_upper = 1000 + (i as i32 * 100);
        let liquidity = 1_000_000 + (i as u128 * 100_000);
        
        let handle = tokio::spawn(async move {
            client_clone
                .with_payer(&lp_keypair)
                .add_liquidity(
                    pool_address,
                    liquidity,
                    tick_lower,
                    tick_upper,
                    u64::MAX,
                    u64::MAX,
                )
                .await
        });
        
        handles.push(handle);
    }
    
    // Wait for all LPs to finish
    let mut successful_adds = 0;
    let mut positions = vec![];
    
    for handle in handles {
        match handle.await.unwrap() {
            Ok(position) => {
                successful_adds += 1;
                positions.push(position);
            }
            Err(e) => {
                eprintln!("LP failed to add liquidity: {:?}", e);
            }
        }
    }
    
    // Most LPs should succeed
    assert!(successful_adds >= 8);
    
    // Verify pool state after concurrent additions
    let pool_info = runner.env.client
        .get_pool_info(pool_address)
        .await
        .unwrap();
    
    assert!(pool_info.liquidity > 0);
    
    // Verify each position was created correctly
    for position in positions {
        let pos_info = runner.env.client
            .get_position_info(position.position_mint)
            .await
            .unwrap();
        
        assert!(pos_info.liquidity > 0);
    }
}

#[tokio::test]
async fn test_concurrent_traders() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Set up pool with liquidity
    let scenario = runner.run_basic_amm_scenario().await.unwrap();
    
    // Create multiple traders
    let trader_count = 20;
    let traders = runner.env.account_factory
        .create_traders(trader_count)
        .await
        .unwrap();
    
    // Record initial pool state
    let initial_state = runner.env.client
        .get_pool_info(scenario.pool_address)
        .await
        .unwrap();
    
    // Execute concurrent swaps
    let pool_address = scenario.pool_address;
    let client = Arc::new(runner.env.client.clone());
    let mut handles = vec![];
    
    for (i, trader) in traders.iter().enumerate() {
        let client_clone = client.clone();
        let trader_keypair = trader.keypair.clone();
        
        // Alternate between buys and sells
        let is_buy = i % 2 == 0;
        let amount = 5_000 + (i as u64 * 1_000);
        
        let handle = tokio::spawn(async move {
            // Small random delay to spread out trades
            tokio::time::sleep(tokio::time::Duration::from_millis(i as u64 * 10)).await;
            
            client_clone
                .with_payer(&trader_keypair)
                .swap(
                    pool_address,
                    amount,
                    0, // Accept any output
                    is_buy,
                    None,
                )
                .await
        });
        
        handles.push(handle);
    }
    
    // Collect results
    let mut successful_swaps = 0;
    let mut total_volume = 0u64;
    
    for handle in handles {
        match handle.await.unwrap() {
            Ok(swap_result) => {
                successful_swaps += 1;
                total_volume += swap_result.amount_in;
            }
            Err(e) => {
                eprintln!("Trader swap failed: {:?}", e);
            }
        }
    }
    
    // Most swaps should succeed
    assert!(successful_swaps >= 15);
    
    // Verify pool processed all trades
    let final_state = runner.env.client
        .get_pool_info(pool_address)
        .await
        .unwrap();
    
    // Volume should have increased
    let volume_increase = 
        final_state.total_volume_0 + final_state.total_volume_1 - 
        initial_state.total_volume_0 - initial_state.total_volume_1;
    
    assert!(volume_increase > 0);
    
    // Price should have moved from trading activity
    assert_ne!(final_state.sqrt_price, initial_state.sqrt_price);
}

#[tokio::test]
async fn test_arbitrage_scenario() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize protocol
    runner.initialize_protocol().await.unwrap();
    
    // Create two pools for same token with different fee tiers
    let token = runner.env.token_factory
        .create_test_token("ARB", 9)
        .await
        .unwrap();
    
    // Pool with 0.05% fee
    let pool_low_fee = runner.env.pool_factory
        .create_pool(token.mint, 5)
        .await
        .unwrap();
    
    // Pool with 0.3% fee
    let pool_high_fee = runner.env.pool_factory
        .create_pool(token.mint, 30)
        .await
        .unwrap();
    
    // Add liquidity to both pools at same price
    runner.env.liquidity_simulator
        .add_balanced_liquidity(pool_low_fee.address, 10_000_000)
        .await
        .unwrap();
    
    runner.env.liquidity_simulator
        .add_balanced_liquidity(pool_high_fee.address, 10_000_000)
        .await
        .unwrap();
    
    // Create price imbalance by large trade in one pool
    runner.env.client
        .swap(
            pool_low_fee.address,
            500_000,
            0,
            true,
            None,
        )
        .await
        .unwrap();
    
    // Get prices from both pools
    let low_fee_state = runner.env.client
        .get_pool_info(pool_low_fee.address)
        .await
        .unwrap();
    
    let high_fee_state = runner.env.client
        .get_pool_info(pool_high_fee.address)
        .await
        .unwrap();
    
    // Prices should diverge
    assert_ne!(low_fee_state.sqrt_price, high_fee_state.sqrt_price);
    
    // Create arbitrageur
    let arbitrageur = runner.env.account_factory
        .create_funded_account()
        .await
        .unwrap();
    
    // Execute arbitrage: buy from cheap pool, sell to expensive pool
    let arb_amount = 100_000;
    
    // Buy from the pool with lower price (after buy, price is higher)
    let buy_result = runner.env.client
        .with_payer(&arbitrageur)
        .swap(
            pool_high_fee.address,
            arb_amount,
            0,
            false, // Sell to this pool
            None,
        )
        .await
        .unwrap();
    
    // Sell to the pool with higher price
    let sell_result = runner.env.client
        .with_payer(&arbitrageur)
        .swap(
            pool_low_fee.address,
            buy_result.amount_out,
            0,
            true, // Buy from this pool
            None,
        )
        .await
        .unwrap();
    
    // Arbitrageur should profit (accounting for fees)
    let profit = sell_result.amount_out as i64 - arb_amount as i64;
    assert!(profit > 0);
    
    // Prices should converge after arbitrage
    let final_low_fee = runner.env.client
        .get_pool_info(pool_low_fee.address)
        .await
        .unwrap();
    
    let final_high_fee = runner.env.client
        .get_pool_info(pool_high_fee.address)
        .await
        .unwrap();
    
    let price_diff = (final_low_fee.sqrt_price as i128 - final_high_fee.sqrt_price as i128).abs();
    let avg_price = (final_low_fee.sqrt_price + final_high_fee.sqrt_price) / 2;
    let price_diff_pct = (price_diff as f64 / avg_price as f64) * 100.0;
    
    // Prices should be within 1% after arbitrage
    assert!(price_diff_pct < 1.0);
}

#[tokio::test]
async fn test_liquidity_sniping() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Set up pool
    runner.initialize_protocol().await.unwrap();
    let pool = runner.create_basic_pool().await.unwrap();
    
    // Add initial wide-range liquidity
    let wide_position = runner.env.client
        .add_liquidity(
            pool.pool_address,
            10_000_000,
            -10000,
            10000,
            u64::MAX,
            u64::MAX,
        )
        .await
        .unwrap();
    
    // Create sniper who will add concentrated liquidity
    let sniper = runner.env.account_factory
        .create_funded_account()
        .await
        .unwrap();
    
    // Get current tick
    let pool_state = runner.env.client
        .get_pool_info(pool.pool_address)
        .await
        .unwrap();
    
    let current_tick = pool_state.current_tick;
    
    // Sniper adds very concentrated liquidity around current price
    let sniper_position = runner.env.client
        .with_payer(&sniper)
        .add_liquidity(
            pool.pool_address,
            1_000_000,
            current_tick - 10,
            current_tick + 10,
            u64::MAX,
            u64::MAX,
        )
        .await
        .unwrap();
    
    // Execute trades that stay within sniper's range
    let mut sniper_fees = 0u64;
    
    for i in 0..20 {
        let is_buy = i % 2 == 0;
        runner.env.client
            .swap(
                pool.pool_address,
                1_000,
                0,
                is_buy,
                None,
            )
            .await
            .unwrap();
    }
    
    // Collect sniper's fees
    let sniper_collected = runner.env.client
        .with_payer(&sniper)
        .collect_position_fees(
            sniper_position.position_mint,
            u64::MAX,
            u64::MAX,
        )
        .await
        .unwrap();
    
    sniper_fees = sniper_collected.amount_0 + sniper_collected.amount_1;
    
    // Collect wide position fees
    let wide_collected = runner.env.client
        .collect_position_fees(
            wide_position.position_mint,
            u64::MAX,
            u64::MAX,
        )
        .await
        .unwrap();
    
    let wide_fees = wide_collected.amount_0 + wide_collected.amount_1;
    
    // Sniper should earn disproportionate fees despite less capital
    let sniper_fee_ratio = sniper_fees as f64 / 1_000_000.0;
    let wide_fee_ratio = wide_fees as f64 / 10_000_000.0;
    
    assert!(sniper_fee_ratio > wide_fee_ratio * 5.0); // At least 5x better returns
}

#[tokio::test]
async fn test_sandwich_attack_scenario() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Set up pool with liquidity
    let scenario = runner.run_basic_amm_scenario().await.unwrap();
    
    // Create victim and attacker accounts
    let victim = runner.env.account_factory
        .create_funded_account()
        .await
        .unwrap();
    
    let attacker = runner.env.account_factory
        .create_funded_account()
        .await
        .unwrap();
    
    // Monitor pool for large pending trade (simulated)
    let victim_trade_size = 100_000;
    
    // Attacker front-runs with buy
    let front_run = runner.env.client
        .with_payer(&attacker)
        .swap(
            scenario.pool_address,
            50_000,
            0,
            true, // Buy
            None,
        )
        .await
        .unwrap();
    
    // Victim executes their trade (pushed to higher price)
    let victim_trade = runner.env.client
        .with_payer(&victim)
        .swap(
            scenario.pool_address,
            victim_trade_size,
            0,
            true, // Buy
            None,
        )
        .await
        .unwrap();
    
    // Attacker back-runs with sell
    let back_run = runner.env.client
        .with_payer(&attacker)
        .swap(
            scenario.pool_address,
            front_run.amount_out,
            0,
            false, // Sell
            None,
        )
        .await
        .unwrap();
    
    // Calculate attacker profit
    let attacker_profit = back_run.amount_out as i64 - 50_000i64;
    
    // Attacker should profit from sandwich
    assert!(attacker_profit > 0);
    
    // Victim got worse execution due to sandwich
    let expected_output = runner.env.client
        .quote_swap(
            scenario.pool_address,
            victim_trade_size,
            true,
        )
        .await
        .unwrap();
    
    assert!(victim_trade.amount_out < expected_output.amount_out);
}

#[tokio::test]
async fn test_multi_pool_liquidity_migration() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize protocol
    runner.initialize_protocol().await.unwrap();
    
    // Create token
    let token = runner.env.token_factory
        .create_test_token("MIGRATE", 9)
        .await
        .unwrap();
    
    // Create multiple pools with different fee tiers
    let fee_tiers = vec![1, 5, 30, 100];
    let mut pools = vec![];
    
    for fee in &fee_tiers {
        let pool = runner.env.pool_factory
            .create_pool(token.mint, *fee)
            .await
            .unwrap();
        pools.push(pool);
    }
    
    // Add liquidity to highest fee pool initially
    let initial_lp = runner.env.account_factory
        .create_liquidity_provider()
        .await
        .unwrap();
    
    let initial_position = runner.env.client
        .with_payer(&initial_lp.keypair)
        .add_liquidity(
            pools[3].address, // 1% fee pool
            10_000_000,
            -5000,
            5000,
            u64::MAX,
            u64::MAX,
        )
        .await
        .unwrap();
    
    // Simulate high volume to justify lower fee tier
    let traders = runner.env.account_factory
        .create_traders(5)
        .await
        .unwrap();
    
    for trader in &traders {
        for _ in 0..10 {
            runner.env.client
                .with_payer(&trader.keypair)
                .swap(
                    pools[3].address,
                    10_000,
                    0,
                    true,
                    None,
                )
                .await
                .unwrap();
        }
    }
    
    // Calculate earned fees in high fee pool
    let high_fee_earned = runner.env.client
        .with_payer(&initial_lp.keypair)
        .get_position_fees_owed(initial_position.position_mint)
        .await
        .unwrap();
    
    // Remove liquidity from high fee pool
    let removed = runner.env.client
        .with_payer(&initial_lp.keypair)
        .remove_liquidity(
            initial_position.position_mint,
            10_000_000,
            0,
            0,
        )
        .await
        .unwrap();
    
    // Add liquidity to medium fee pool
    let new_position = runner.env.client
        .with_payer(&initial_lp.keypair)
        .add_liquidity(
            pools[2].address, // 0.3% fee pool
            10_000_000,
            -5000,
            5000,
            removed.amount_0,
            removed.amount_1,
        )
        .await
        .unwrap();
    
    // Higher volume in lower fee pool
    for trader in &traders {
        for _ in 0..30 {
            runner.env.client
                .with_payer(&trader.keypair)
                .swap(
                    pools[2].address,
                    10_000,
                    0,
                    true,
                    None,
                )
                .await
                .unwrap();
        }
    }
    
    // Medium fee pool should generate more total fees due to higher volume
    let medium_fee_earned = runner.env.client
        .with_payer(&initial_lp.keypair)
        .get_position_fees_owed(new_position.position_mint)
        .await
        .unwrap();
    
    let total_medium_fees = medium_fee_earned.amount_0 + medium_fee_earned.amount_1;
    let total_high_fees = high_fee_earned.amount_0 + high_fee_earned.amount_1;
    
    // Despite lower fee rate, should earn more from volume
    assert!(total_medium_fees > total_high_fees / 2);
}