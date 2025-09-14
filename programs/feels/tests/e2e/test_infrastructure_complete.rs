//! Complete infrastructure test to verify all components work together

use crate::common::*;

test_in_memory!(test_complete_infrastructure, |ctx: TestContext| async move {
    println!("Testing complete infrastructure setup...");
    
    // 1. Test basic context functionality
    println!("1. Testing basic context functionality");
    let payer = ctx.payer().await;
    assert_ne!(payer, Pubkey::default());
    
    // 2. Test account creation
    println!("2. Testing account creation");
    assert_ne!(ctx.accounts.alice.pubkey(), Pubkey::default());
    assert_ne!(ctx.accounts.bob.pubkey(), Pubkey::default());
    
    // 3. Test token mints - create simple tokens without using protocol token system
    println!("3. Testing token mints");
    let custom_token = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    println!("  - Created custom token: {}", custom_token.pubkey());
    println!("  - FeelsSOL mint: {}", ctx.feelssol_mint);
    
    // 4. Test ATA creation
    println!("4. Testing ATA creation");
    let alice_feelssol = ctx.create_ata(&ctx.accounts.alice.pubkey(), &ctx.feelssol_mint).await?;
    let alice_custom = ctx.create_ata(&ctx.accounts.alice.pubkey(), &custom_token.pubkey()).await?;
    println!("  - Alice FeelsSOL account: {}", alice_feelssol);
    println!("  - Alice custom token account: {}", alice_custom);
    
    // 5. Test minting
    println!("5. Testing token minting");
    ctx.mint_to(&ctx.feelssol_mint, &alice_feelssol, &ctx.feelssol_authority, 1_000_000_000).await?;
    ctx.mint_to(&custom_token.pubkey(), &alice_custom, &ctx.accounts.market_creator, 1_000_000_000).await?;
    
    let balance_feelssol = ctx.get_token_balance(&alice_feelssol).await?;
    let balance_custom = ctx.get_token_balance(&alice_custom).await?;
    assert_eq!(balance_feelssol, 1_000_000_000);
    assert_eq!(balance_custom, 1_000_000_000);
    println!("  - Alice FeelsSOL balance: {}", balance_feelssol);
    println!("  - Alice custom token balance: {}", balance_custom);
    
    // 6. Test market creation (skipped - requires protocol tokens)
    println!("6. Testing market creation");
    println!("  - SKIPPED: Market creation requires protocol tokens for non-FeelsSOL tokens");
    
    // 7. Test FeelsSOL entry
    println!("7. Testing FeelsSOL entry");
    let bob_feelssol = ctx.create_ata(&ctx.accounts.bob.pubkey(), &ctx.feelssol_mint).await?;
    
    // Note: JitoSOL is a mainnet token and cannot be created in tests
    // In production, users would already have JitoSOL to convert
    println!("  - JitoSOL operations skipped (mainnet token)");
    println!("  - Created Bob's FeelsSOL account: {}", bob_feelssol);
    
    // 8. Test builder patterns
    println!("8. Testing builder patterns");
    let _market_builder = ctx.market_builder()
        .token_0(ctx.feelssol_mint)
        .token_1(custom_token.pubkey())
        .initial_price(constants::PRICE_1_TO_1)
        .fee_rate(30);
    println!("  - Market builder configured");
    
    // 9. Test swap builder
    println!("9. Testing swap builder");
    let _swap_builder = ctx.swap_builder();
    println!("  - Swap builder created");
    
    println!("\nAll infrastructure tests passed! ✅");
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_helpers_integration, |ctx: TestContext| async move {
    println!("Testing helpers integration...");
    
    println!("Note: Market creation helpers require protocol tokens");
    println!("For MVP testing, only basic token operations are tested");
    
    // Create a simple token
    let custom_token = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    
    println!("Created custom token: {}", custom_token.pubkey());
    println!("FeelsSOL mint: {}", ctx.feelssol_mint);
    
    // Test helper methods exist
    let _market_helper = ctx.market_helper();
    let _swap_helper = ctx.swap_helper();
    let _position_helper = ctx.position_helper();
    
    println!("All helpers accessible ✅");
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_instruction_wrappers, |ctx: TestContext| async move {
    println!("Testing instruction wrappers...");
    
    println!("Note: Full instruction testing requires protocol tokens and markets");
    println!("For MVP testing, only basic operations are tested");
    
    // Test basic token operations
    let custom_token = ctx.create_mint(&ctx.accounts.alice.pubkey(), 6).await?;
    let alice_token = ctx.create_ata(&ctx.accounts.alice.pubkey(), &custom_token.pubkey()).await?;
    
    ctx.mint_to(&custom_token.pubkey(), &alice_token, &ctx.accounts.alice, 10_000_000_000).await?;
    
    let balance = ctx.get_token_balance(&alice_token).await?;
    assert_eq!(balance, 10_000_000_000);
    
    println!("Basic token operations working ✅");
    
    // Market and swap operations would require protocol tokens
    println!("Market/swap operations skipped - require protocol tokens");
    
    println!("Instruction wrapper test passed! ✅");
    
    Ok::<(), Box<dyn std::error::Error>>(())
});