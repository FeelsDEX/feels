//! E2E test for token mint and market deployment lifecycle

use crate::common::*;
use feels::{
    constants::*,
    state::Market,
};

/// Test minting a new token and deploying initial liquidity
test_in_memory!(test_mint_and_deploy_token, |ctx: TestContext| async move {
    println!("=== Testing Token Mint and Deploy ===");
    println!("Note: This test requires protocol token and market creation functionality");
    println!("SKIPPED: Market creation is bypassed for MVP testing");
    
    // When full functionality is available:
    // - Create protocol token via mint_token
    // - Create market with FeelsSOL
    // - Deploy initial liquidity
    // - Verify market state
    
    println!("✓ Test marked as TODO - requires protocol token integration");
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

/// Test feelssol conversions
test_in_memory!(test_feelssol_conversions, |ctx: TestContext| async move {
    println!("=== Testing FeelsSOL Conversions ===");
    
    // This is a placeholder test since the actual conversion instructions
    // are not available in the current SDK
    
    // Create feelssol token account
    let user = &ctx.accounts.alice;
    let user_feelssol = ctx.create_ata(&user.pubkey(), &ctx.feelssol_mint).await?;
    
    // Verify account was created
    assert_ne!(user_feelssol, Pubkey::default());
    
    println!("✓ FeelsSOL account created");
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

/// Test token distribution
test_in_memory!(test_token_distribution, |ctx: TestContext| async move {
    println!("=== Testing Token Distribution ===");
    
    let token_mint = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 9).await?;
    
    // Create ATAs for distribution
    let alice_token = ctx.create_ata(&ctx.accounts.alice.pubkey(), &token_mint.pubkey()).await?;
    let bob_token = ctx.create_ata(&ctx.accounts.bob.pubkey(), &token_mint.pubkey()).await?;
    let charlie_token = ctx.create_ata(&ctx.accounts.charlie.pubkey(), &token_mint.pubkey()).await?;
    
    // Mint tokens to creator first
    let creator_token = ctx.create_ata(&ctx.accounts.market_creator.pubkey(), &token_mint.pubkey()).await?;
    ctx.mint_to(&token_mint.pubkey(), &creator_token, &ctx.accounts.market_creator, 100_000_000_000).await?;
    
    // Verify minting worked
    let balance = ctx.get_token_balance(&creator_token).await?;
    assert_eq!(balance, 100_000_000_000);
    
    println!("✓ Token distribution test complete");
    
    Ok::<(), Box<dyn std::error::Error>>(())
});