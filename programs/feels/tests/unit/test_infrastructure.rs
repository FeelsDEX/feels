//! Basic test to verify test infrastructure is working

use crate::common::*;

test_in_memory!(test_basic_infrastructure, |ctx: TestContext| async move {
    // Test that we can get the payer
    let payer = ctx.payer().await;
    assert_ne!(payer, Pubkey::default());
    
    // Test that accounts are initialized
    assert_ne!(ctx.accounts.alice.pubkey(), Pubkey::default());
    assert_ne!(ctx.accounts.bob.pubkey(), Pubkey::default());
    
    // Test that token mints are set up
    assert_ne!(ctx.feelssol_mint, Pubkey::default());
    assert_eq!(ctx.jitosol_mint, constants::JITOSOL_MINT);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_airdrop, |ctx: TestContext| async move {
    // Test airdrop functionality
    let recipient = Pubkey::new_unique();
    ctx.airdrop(&recipient, 1_000_000_000).await?;
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_create_mint, |ctx: TestContext| async move {
    // Test token mint creation
    let authority = ctx.accounts.alice.pubkey();
    let mint = ctx.create_mint(&authority, 6).await?;
    
    assert_ne!(mint.pubkey(), Pubkey::default());
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_create_ata, |ctx: TestContext| async move {
    // Create a test mint
    let authority = ctx.accounts.alice.pubkey();
    let mint = ctx.create_mint(&authority, 6).await?;
    
    // Create ATA
    let owner = ctx.accounts.bob.pubkey();
    let ata = ctx.create_ata(&owner, &mint.pubkey()).await?;
    
    assert_ne!(ata, Pubkey::default());
    
    Ok::<(), Box<dyn std::error::Error>>(())
});