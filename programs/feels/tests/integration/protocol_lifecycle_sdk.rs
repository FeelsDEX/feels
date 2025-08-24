/// Protocol lifecycle integration tests using SDK and simulation framework
/// Tests real on-chain protocol initialization, pool creation, and state management

use anchor_lang::prelude::*;
use feels_sdk::{FeelsClient, SdkConfig};
use feels_simulation::{ScenarioRunner, TestEnvironment};

#[tokio::test]
async fn test_protocol_initialization_sequence() {
    // Create test environment with real program deployment
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize protocol through scenario runner
    let init_result = runner.initialize_protocol().await.unwrap();
    
    // Query real on-chain protocol state
    let protocol_state = runner.env.client
        .get_protocol_state()
        .await
        .unwrap();
    
    // Validate protocol configuration
    assert_eq!(protocol_state.authority, init_result.authority);
    assert_eq!(protocol_state.emergency_authority, init_result.emergency_authority);
    assert_ne!(protocol_state.authority, protocol_state.emergency_authority);
    assert_eq!(protocol_state.protocol_fee_rate, 1000); // 10%
    assert_eq!(protocol_state.version, 1);
    
    // Verify FeelsSOL initialization
    let feelssol_state = runner.env.client
        .get_feelssol_state()
        .await
        .unwrap();
    
    assert_eq!(feelssol_state.authority, init_result.authority);
    assert_eq!(feelssol_state.mint, init_result.feelssol_mint);
    assert_eq!(feelssol_state.total_supply, 0);
    assert_eq!(feelssol_state.version, 1);
}

#[tokio::test]
async fn test_pool_creation_lifecycle() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize protocol
    runner.initialize_protocol().await.unwrap();
    
    // Create multiple pools with different fee tiers
    let fee_tiers = vec![1, 5, 30, 100]; // 0.01%, 0.05%, 0.3%, 1%
    let mut pools = vec![];
    
    for fee_tier in fee_tiers {
        // Create test token
        let token = runner.env.token_factory
            .create_test_token(&format!("TEST{}", fee_tier), 9)
            .await
            .unwrap();
        
        // Create pool with specific fee tier
        let pool = runner.env.pool_factory
            .create_pool(token.mint, fee_tier)
            .await
            .unwrap();
        
        pools.push((pool, fee_tier));
    }
    
    // Verify each pool configuration
    for (pool, expected_fee) in pools {
        let pool_info = runner.env.client
            .get_pool_info(pool.address)
            .await
            .unwrap();
        
        assert_eq!(pool_info.fee_rate, expected_fee);
        assert_eq!(pool_info.protocol_fee_rate, 100); // 10% of swap fees
        assert_eq!(pool_info.liquidity, 0);
        assert_eq!(pool_info.tick_spacing, pool.tick_spacing);
        
        // Verify vaults were created
        assert_ne!(pool_info.vault_a, Pubkey::default());
        assert_ne!(pool_info.vault_b, Pubkey::default());
    }
}

#[tokio::test]
async fn test_authority_management() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize protocol
    let init_result = runner.initialize_protocol().await.unwrap();
    let original_authority = init_result.authority;
    
    // Create new authority keypair
    let new_authority = runner.env.account_factory
        .create_funded_account()
        .await
        .unwrap();
    
    // Transfer protocol authority
    runner.env.client
        .with_payer(&runner.env.protocol_authority)
        .transfer_protocol_authority(new_authority.pubkey())
        .await
        .unwrap();
    
    // Verify authority was transferred
    let protocol_state = runner.env.client
        .get_protocol_state()
        .await
        .unwrap();
    
    assert_eq!(protocol_state.authority, new_authority.pubkey());
    assert_ne!(protocol_state.authority, original_authority);
    
    // Old authority should no longer be able to make changes
    let result = runner.env.client
        .with_payer(&runner.env.protocol_authority)
        .update_protocol_fee_rate(2000)
        .await;
    
    assert!(result.is_err());
    
    // New authority should be able to make changes
    runner.env.client
        .with_payer(&new_authority)
        .update_protocol_fee_rate(2000)
        .await
        .unwrap();
    
    // Verify fee rate was updated
    let updated_state = runner.env.client
        .get_protocol_state()
        .await
        .unwrap();
    
    assert_eq!(updated_state.protocol_fee_rate, 2000); // 20%
}

#[tokio::test]
async fn test_pool_state_consistency() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Run basic AMM scenario to set up pool with activity
    let scenario = runner.run_basic_amm_scenario().await.unwrap();
    
    // Get pool state
    let pool_state = runner.env.client
        .get_pool_info(scenario.pool_address)
        .await
        .unwrap();
    
    // Verify state consistency
    assert!(pool_state.liquidity > 0);
    assert_ne!(pool_state.sqrt_price, 0);
    assert!(pool_state.total_volume_0 > 0 || pool_state.total_volume_1 > 0);
    
    // Verify tick is consistent with sqrt price
    let calculated_tick = runner.env.client
        .sqrt_price_to_tick(pool_state.sqrt_price)
        .await
        .unwrap();
    
    assert_eq!(pool_state.current_tick, calculated_tick);
    
    // Verify fee growth tracking
    assert!(pool_state.fee_growth_global_0 > 0 || pool_state.fee_growth_global_1 > 0);
}

#[tokio::test]
async fn test_emergency_controls() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize protocol
    let init_result = runner.initialize_protocol().await.unwrap();
    
    // Create and setup a pool
    let pool = runner.create_basic_pool().await.unwrap();
    
    // Add liquidity
    runner.env.liquidity_simulator
        .add_liquidity_to_pool(pool.pool_address, 1_000_000, -1000, 1000)
        .await
        .unwrap();
    
    // Pause the pool using emergency authority
    runner.env.client
        .with_payer(&runner.env.emergency_authority)
        .pause_pool(pool.pool_address)
        .await
        .unwrap();
    
    // Verify pool is paused
    let pool_info = runner.env.client
        .get_pool_info(pool.pool_address)
        .await
        .unwrap();
    
    assert!(pool_info.is_paused);
    
    // Swaps should fail on paused pool
    let swap_result = runner.env.client
        .swap(pool.pool_address, 1000, 900, true, None)
        .await;
    
    assert!(swap_result.is_err());
    
    // Unpause the pool
    runner.env.client
        .with_payer(&runner.env.emergency_authority)
        .unpause_pool(pool.pool_address)
        .await
        .unwrap();
    
    // Swaps should work again
    let swap_result = runner.env.client
        .swap(pool.pool_address, 1000, 900, true, None)
        .await;
    
    assert!(swap_result.is_ok());
}

#[tokio::test]
async fn test_feelssol_minting_and_burning() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize protocol
    runner.initialize_protocol().await.unwrap();
    
    // Create test account
    let user = runner.env.account_factory
        .create_funded_account()
        .await
        .unwrap();
    
    // Mint FeelsSOL
    let mint_amount = 1_000_000;
    runner.env.client
        .mint_feelssol(user.pubkey(), mint_amount)
        .await
        .unwrap();
    
    // Check FeelsSOL balance
    let balance = runner.env.client
        .get_feelssol_balance(user.pubkey())
        .await
        .unwrap();
    
    assert_eq!(balance, mint_amount);
    
    // Check total supply
    let feelssol_state = runner.env.client
        .get_feelssol_state()
        .await
        .unwrap();
    
    assert_eq!(feelssol_state.total_supply, mint_amount);
    
    // Burn some FeelsSOL
    let burn_amount = 400_000;
    runner.env.client
        .with_payer(&user)
        .burn_feelssol(burn_amount)
        .await
        .unwrap();
    
    // Verify balance and supply updated
    let new_balance = runner.env.client
        .get_feelssol_balance(user.pubkey())
        .await
        .unwrap();
    
    assert_eq!(new_balance, mint_amount - burn_amount);
    
    let updated_state = runner.env.client
        .get_feelssol_state()
        .await
        .unwrap();
    
    assert_eq!(updated_state.total_supply, mint_amount - burn_amount);
}

#[tokio::test]
async fn test_protocol_upgrade_path() {
    let mut runner = ScenarioRunner::new().await.unwrap();
    
    // Initialize protocol
    runner.initialize_protocol().await.unwrap();
    
    // Get current protocol version
    let protocol_state = runner.env.client
        .get_protocol_state()
        .await
        .unwrap();
    
    assert_eq!(protocol_state.version, 1);
    
    // Simulate protocol upgrade preparation
    // In real scenario, this would involve program upgrade
    runner.env.client
        .prepare_protocol_upgrade(2)
        .await
        .unwrap();
    
    // Verify upgrade readiness
    let upgrade_state = runner.env.client
        .get_protocol_upgrade_state()
        .await
        .unwrap();
    
    assert!(upgrade_state.is_ready);
    assert_eq!(upgrade_state.target_version, 2);
}