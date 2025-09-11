//! Debug test to understand failures

use crate::common::*;

test_in_memory!(test_minimal_setup, |ctx: TestContext| async move {
    println!("Test context created successfully");
    
    // Just try to get the payer
    let payer = ctx.payer().await;
    println!("Payer: {}", payer);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_create_simple_mint, |ctx: TestContext| async move {
    println!("Creating simple mint...");
    
    // Create a simple mint
    let mint = ctx.create_mint(&ctx.accounts.alice.pubkey(), 6).await?;
    println!("Mint created: {}", mint.pubkey());
    
    Ok::<(), Box<dyn std::error::Error>>(())
});