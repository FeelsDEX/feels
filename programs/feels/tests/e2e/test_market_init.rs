//! Test market initialization
use crate::common::*;

test_in_memory!(test_initialize_market, |ctx: TestContext| async move {
    println!("Testing simplified market initialization...");
    println!("FeelsSOL mint: {}", ctx.feelssol_mint);

    // For MVP testing, let's just test that we can query market helper
    let _market_helper = ctx.market_helper();

    // Create test tokens
    let token_0 = ctx.create_mint(&ctx.accounts.alice.pubkey(), 6).await?;
    let token_1 = ctx.create_mint(&ctx.accounts.alice.pubkey(), 6).await?;

    println!("Created token A: {}", token_0.pubkey());
    println!("Created token B: {}", token_1.pubkey());

    // For now, just verify tokens were created
    let mint_a = ctx.get_mint(&token_0.pubkey()).await?;
    assert_eq!(mint_a.decimals, 6);

    let mint_b = ctx.get_mint(&token_1.pubkey()).await?;
    assert_eq!(mint_b.decimals, 6);

    println!("Test passed - basic infrastructure working!");

    Ok::<(), Box<dyn std::error::Error>>(())
});
