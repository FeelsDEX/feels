//! Example test demonstrating the new test infrastructure

use crate::common::*;

#[test_all_environments!(test_basic_swap)]
async fn test_basic_swap(ctx: TestContext) -> TestResult<()> {
    // Create test tokens
    let token_0 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 9).await?;
    let token_1 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    
    // Create market with initial liquidity
    let market = ctx.market_builder()
        .token_0(token_0.pubkey())
        .token_1(token_1.pubkey())
        .initial_price(constants::PRICE_1_TO_1)
        .add_full_range_liquidity(ctx.accounts.market_creator.insecure_clone(), 1_000_000_000)
        .build()
        .await?;
    
    // Setup trader with tokens
    let trader_token_0 = ctx.create_ata(&ctx.accounts.alice.pubkey(), &token_0.pubkey()).await?;
    ctx.mint_to(
        &token_0.pubkey(),
        &trader_token_0,
        &ctx.accounts.market_creator,
        1_000_000_000,
    ).await?;
    
    // Execute swap
    let swap_result = ctx.swap_helper().swap(
        &market,
        &token_0.pubkey(),
        &token_1.pubkey(),
        100_000_000,
        &ctx.accounts.alice,
    ).await?;
    
    // Verify results
    assert!(swap_result.amount_out > 0);
    assert_eq!(swap_result.amount_in, 100_000_000);
    
    Ok::<(), Box<dyn std::error::Error>>(())
}

#[test_in_memory!(test_position_lifecycle)]
async fn test_position_lifecycle(ctx: TestContext) -> TestResult<()> {
    // Create tokens
    let token_0 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 9).await?;
    let token_1 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    
    // Create market
    let market = ctx.market_builder()
        .token_0(token_0.pubkey())
        .token_1(token_1.pubkey())
        .build()
        .await?;
    
    // Setup liquidity provider
    let lp = &ctx.accounts.bob;
    let lp_token_0 = ctx.create_ata(&lp.pubkey(), &token_0.pubkey()).await?;
    let lp_token_1 = ctx.create_ata(&lp.pubkey(), &token_1.pubkey()).await?;
    
    ctx.mint_to(&token_0.pubkey(), &lp_token_0, &ctx.accounts.market_creator, 10_000_000_000).await?;
    ctx.mint_to(&token_1.pubkey(), &lp_token_1, &ctx.accounts.market_creator, 10_000_000_000).await?;
    
    // Open positions using builder
    let positions = ctx.position_builder()
        .market(market)
        .owner(lp.insecure_clone())
        .add_position(-1000, 1000, 1_000_000_000)
        .add_position(-5000, -1000, 500_000_000)
        .add_position(1000, 5000, 500_000_000)
        .build()
        .await?;
    
    assert_eq!(positions.len(), 3);
    
    // Close one position
    ctx.position_helper().close_position(&positions[0], lp).await?;
    
    Ok::<(), Box<dyn std::error::Error>>(())
}

#[test_in_memory!(test_complex_swap_scenario)]
async fn test_complex_swap_scenario(ctx: TestContext) -> TestResult<()> {
    // Create tokens
    let token_0 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 9).await?;
    let token_1 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    
    // Create market with liquidity
    let market = ctx.market_builder()
        .token_0(token_0.pubkey())
        .token_1(token_1.pubkey())
        .add_full_range_liquidity(ctx.accounts.market_creator.insecure_clone(), 10_000_000_000)
        .build()
        .await?;
    
    // Setup traders
    for trader in [&ctx.accounts.alice, &ctx.accounts.bob] {
        let trader_token_0 = ctx.create_ata(&trader.pubkey(), &token_0.pubkey()).await?;
        let trader_token_1 = ctx.create_ata(&trader.pubkey(), &token_1.pubkey()).await?;
        
        ctx.mint_to(&token_0.pubkey(), &trader_token_0, &ctx.accounts.market_creator, 5_000_000_000).await?;
        ctx.mint_to(&token_1.pubkey(), &trader_token_1, &ctx.accounts.market_creator, 5_000_000_000).await?;
    }
    
    // Execute sandwich attack scenario
    let results = ctx.swap_builder()
        .sandwich_attack(
            market,
            ctx.accounts.bob.insecure_clone(),  // victim
            ctx.accounts.alice.insecure_clone(), // attacker
            token_0.pubkey(),
            token_1.pubkey(),
            1_000_000_000,  // victim amount
            500_000_000,    // front-run amount
        )
        .execute()
        .await?;
    
    assert_eq!(results.len(), 3);
    
    Ok::<(), Box<dyn std::error::Error>>(())
}

#[with_time_test!(test_oracle_updates)]
async fn test_oracle_updates(ctx: &TestContext) -> TestResult<()> {
    // Create tokens and market
    let token_0 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 9).await?;
    let token_1 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    
    let market = ctx.market_builder()
        .token_0(token_0.pubkey())
        .token_1(token_1.pubkey())
        .add_full_range_liquidity(ctx.accounts.market_creator.insecure_clone(), 1_000_000_000)
        .build()
        .await?;
    
    // Test TWAP calculation over time
    let timestamps = TimeScenarios::test_twap_calculation(
        ctx,
        &market,
        10, // 10 second intervals
        5,  // 5 observations
    ).await?;
    
    // Verify timestamps are properly ordered
    ctx.assert_timestamps_ordered(&timestamps)?;
    
    Ok::<(), Box<dyn std::error::Error>>(())
}

#[cfg(test)]
mod market_creation_tests {
    use super::*;
    
    #[test_in_memory!(test_market_creation_variations)]
    async fn test_market_creation_variations(ctx: TestContext) -> TestResult<()> {
        let token_0 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 9).await?;
        let token_1 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
        
        // Simple market
        let simple_market = ctx.market_helper()
            .create_simple_market(&token_0.pubkey(), &token_1.pubkey())
            .await?;
        
        // Verify market exists
        let market_state = ctx.market_helper()
            .get_market(&simple_market)
            .await?
            .ok_or("Market not found")?;
        
        assert_eq!(market_state.token_0, token_0.pubkey());
        assert_eq!(market_state.token_1, token_1.pubkey());
        
        Ok::<(), Box<dyn std::error::Error>>(())
    }
}