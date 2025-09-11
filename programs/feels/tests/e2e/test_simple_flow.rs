//! Simple e2e test to validate infrastructure

use crate::common::*;

test_in_memory!(test_simple_infrastructure, |ctx: TestContext| async move {
    // Create test tokens
    let token_0 = ctx.create_mint(&ctx.accounts.alice.pubkey(), 6).await?;
    let token_1 = ctx.create_mint(&ctx.accounts.alice.pubkey(), 6).await?;
    
    // Create ATAs
    let alice_token_0 = ctx.create_ata(&ctx.accounts.alice.pubkey(), &token_0.pubkey()).await?;
    let alice_token_1 = ctx.create_ata(&ctx.accounts.alice.pubkey(), &token_1.pubkey()).await?;
    
    // Mint tokens
    ctx.mint_to(
        &token_0.pubkey(),
        &alice_token_0,
        &ctx.accounts.alice,
        1_000_000_000,
    ).await?;
    
    ctx.mint_to(
        &token_1.pubkey(), 
        &alice_token_1,
        &ctx.accounts.alice,
        1_000_000_000,
    ).await?;
    
    // Verify balances
    let balance_a = ctx.get_token_balance(&alice_token_0).await?;
    let balance_b = ctx.get_token_balance(&alice_token_1).await?;
    
    assert_eq!(balance_a, 1_000_000_000);
    assert_eq!(balance_b, 1_000_000_000);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});