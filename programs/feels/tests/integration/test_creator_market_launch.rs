//! Tests for creator-only market launch and initial buy functionality
use crate::common::*;
use feels::state::{Market, ProtocolToken};

test_all_environments!(test_creator_only_can_launch_market, |ctx: TestContext| async move {
    println!("\n=== Test: Only Creator Can Launch Market ===");
    
    // Step 1: Creator mints a token
    let token_creator = Keypair::new();
    ctx.airdrop(&token_creator.pubkey(), 1_000_000_000).await?; // 1 SOL
    
    let token_mint = Keypair::new();
    let params = feels::instructions::MintTokenParams {
        ticker: "CREATOR".to_string(),
        name: "Creator Token".to_string(),
        uri: "https://test.com".to_string(),
    };
    
    let ix = feels_sdk::mint_token(
        token_creator.pubkey(),
        token_mint.pubkey(),
        ctx.feelssol_mint,
        params,
    )?;
    
    ctx.process_instruction(ix, &[&token_creator, &token_mint]).await?;
    println!("✓ Token minted by creator: {}", token_creator.pubkey());
    
    // Step 2: Creator successfully launches market
    let market = ctx.initialize_market(
        &token_creator,
        &ctx.feelssol_mint,
        &token_mint.pubkey(),
        30, // 0.3% fee
        10, // tick spacing
        79228162514264337593543950336u128, // 1:1 price
        0, // no initial buy
    ).await?;
    
    println!("✓ Creator successfully launched market: {}", market);
    
    // Step 3: Someone else tries to launch a market with the same token (should fail)
    let imposter = Keypair::new();
    ctx.airdrop(&imposter.pubkey(), 1_000_000_000).await?;
    
    let result = ctx.initialize_market(
        &imposter,
        &ctx.feelssol_mint,
        &token_mint.pubkey(),
        30,
        10,
        79228162514264337593543950336u128,
        0,
    ).await;
    
    assert!(result.is_err(), "Non-creator should not be able to launch market");
    println!("✓ Non-creator correctly rejected from launching market");
    
    println!("\n=== Creator-Only Market Launch Test Passed ===");
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_all_environments!(test_market_launch_with_initial_buy, |ctx: TestContext| async move {
    println!("\n=== Test: Market Launch with Initial Buy ===");
    
    // Step 1: Mint a protocol token
    let token_creator = Keypair::new();
    ctx.airdrop(&token_creator.pubkey(), 5_000_000_000).await?; // 5 SOL
    
    let token_mint = Keypair::new();
    let params = feels::instructions::MintTokenParams {
        ticker: "LAUNCH".to_string(),
        name: "Launch Token".to_string(),
        uri: "https://test.com".to_string(),
    };
    
    let ix = feels_sdk::mint_token(
        token_creator.pubkey(),
        token_mint.pubkey(),
        ctx.feelssol_mint,
        params,
    )?;
    
    ctx.process_instruction(ix, &[&token_creator, &token_mint]).await?;
    println!("✓ Token minted: {}", token_mint.pubkey());
    
    // Step 2: Get FeelsSOL for the creator
    let creator_jitosol = ctx.create_ata(&token_creator.pubkey(), &ctx.jitosol_mint).await?;
    let creator_feelssol = ctx.create_ata(&token_creator.pubkey(), &ctx.feelssol_mint).await?;
    
    // Mint some JitoSOL to creator (in production, they would already have it)
    ctx.mint_to(&ctx.jitosol_mint, &creator_jitosol, &ctx.jitosol_authority, 1_000_000_000).await?;
    
    // Convert JitoSOL to FeelsSOL
    ctx.enter_feelssol(
        &token_creator,
        &creator_jitosol,
        &creator_feelssol,
        1_000_000_000, // 1 JitoSOL
    ).await?;
    
    let feelssol_balance = ctx.get_token_balance(&creator_feelssol).await?;
    println!("✓ Creator has {} FeelsSOL", feelssol_balance);
    
    // Step 3: Launch market with initial buy
    let initial_buy_amount = 100_000_000; // 0.1 FeelsSOL
    
    let market = ctx.initialize_market(
        &token_creator,
        &ctx.feelssol_mint,
        &token_mint.pubkey(),
        30, // 0.3% fee
        10, // tick spacing
        79228162514264337593543950336u128, // 1:1 price
        initial_buy_amount,
    ).await?;
    
    println!("✓ Market launched with initial buy: {}", market);
    
    // Step 4: Verify FeelsSOL was transferred to vault
    let feelssol_balance_after = ctx.get_token_balance(&creator_feelssol).await?;
    assert_eq!(
        feelssol_balance_after,
        feelssol_balance - initial_buy_amount,
        "FeelsSOL should be deducted for initial buy"
    );
    
    // Verify vault received the FeelsSOL
    let (vault_feelssol, _) = feels_sdk::find_vault_address(&market, &ctx.feelssol_mint);
    let vault_balance = ctx.get_token_balance(&vault_feelssol).await?;
    assert_eq!(vault_balance, initial_buy_amount, "Vault should have received FeelsSOL");
    
    println!("✓ Initial buy FeelsSOL transferred to vault");
    println!("  Amount: {} FeelsSOL", initial_buy_amount);
    println!("  Creator balance after: {}", feelssol_balance_after);
    println!("  Vault balance: {}", vault_balance);
    
    println!("\n=== Market Launch with Initial Buy Test Passed ===");
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_multiple_creators_different_tokens, |ctx: TestContext| async move {
    println!("\n=== Test: Multiple Creators with Different Tokens ===");
    
    // Create multiple creators and tokens
    let creators_and_tokens = vec![
        ("Alice", "ALICE", "Alice Token"),
        ("Bob", "BOB", "Bob Token"),
        ("Charlie", "CHAR", "Charlie Token"),
    ];
    
    let mut markets = Vec::new();
    
    for (name, ticker, token_name) in creators_and_tokens {
        // Create and fund creator
        let creator = Keypair::new();
        ctx.airdrop(&creator.pubkey(), 2_000_000_000).await?;
        
        // Mint token
        let token_mint = Keypair::new();
        let params = feels::instructions::MintTokenParams {
            ticker: ticker.to_string(),
            name: token_name.to_string(),
            uri: format!("https://{}.test", name.to_lowercase()),
        };
        
        let ix = feels_sdk::mint_token(
            creator.pubkey(),
            token_mint.pubkey(),
            ctx.feelssol_mint,
            params,
        )?;
        
        ctx.process_instruction(ix, &[&creator, &token_mint]).await?;
        println!("✓ {} minted token {}", name, ticker);
        
        // Launch market
        let market = ctx.initialize_market(
            &creator,
            &ctx.feelssol_mint,
            &token_mint.pubkey(),
            30,
            10,
            79228162514264337593543950336u128,
            0,
        ).await?;
        
        println!("✓ {} launched market: {}", name, market);
        markets.push((name, creator.pubkey(), token_mint.pubkey(), market));
    }
    
    // Verify each creator can only launch their own token's market
    println!("\nVerifying cross-creator restrictions...");
    
    // Bob tries to launch Alice's token market (should fail)
    let bob = Keypair::new();
    ctx.airdrop(&bob.pubkey(), 1_000_000_000).await?;
    
    let alice_token = markets[0].2; // Alice's token
    let result = ctx.initialize_market(
        &bob,
        &ctx.feelssol_mint,
        &alice_token,
        30,
        10,
        79228162514264337593543950336u128,
        0,
    ).await;
    
    assert!(result.is_err(), "Bob should not be able to launch Alice's token market");
    println!("✓ Cross-creator market launch correctly prevented");
    
    println!("\n=== Multiple Creators Test Passed ===");
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_feelssol_pairing_requirement, |ctx: TestContext| async move {
    println!("\n=== Test: FeelsSOL Pairing Requirement ===");
    
    // Create two protocol tokens
    let creator1 = Keypair::new();
    let creator2 = Keypair::new();
    ctx.airdrop(&creator1.pubkey(), 1_000_000_000).await?;
    ctx.airdrop(&creator2.pubkey(), 1_000_000_000).await?;
    
    let token1 = Keypair::new();
    let token2 = Keypair::new();
    
    // Mint first token
    let params1 = feels::instructions::MintTokenParams {
        ticker: "TOKEN1".to_string(),
        name: "Token 1".to_string(),
        uri: "https://token1.test".to_string(),
    };
    
    let ix1 = feels_sdk::mint_token(
        creator1.pubkey(),
        token1.pubkey(),
        ctx.feelssol_mint,
        params1,
    )?;
    
    ctx.process_instruction(ix1, &[&creator1, &token1]).await?;
    
    // Mint second token
    let params2 = feels::instructions::MintTokenParams {
        ticker: "TOKEN2".to_string(),
        name: "Token 2".to_string(),
        uri: "https://token2.test".to_string(),
    };
    
    let ix2 = feels_sdk::mint_token(
        creator2.pubkey(),
        token2.pubkey(),
        ctx.feelssol_mint,
        params2,
    )?;
    
    ctx.process_instruction(ix2, &[&creator2, &token2]).await?;
    
    println!("✓ Created two protocol tokens");
    
    // Try to create market between the two tokens (should fail - no FeelsSOL)
    // Note: Need to ensure token order is correct
    let (token_0, token_1) = if token1.pubkey() < token2.pubkey() {
        (token1.pubkey(), token2.pubkey())
    } else {
        (token2.pubkey(), token1.pubkey())
    };
    
    let result = ctx.initialize_market(
        &creator1,
        &token_0,
        &token_1,
        30,
        10,
        79228162514264337593543950336u128,
        0,
    ).await;
    
    assert!(result.is_err(), "Should not be able to create market without FeelsSOL");
    println!("✓ Market creation without FeelsSOL correctly rejected");
    
    // Create valid market with FeelsSOL
    let market = ctx.initialize_market(
        &creator1,
        &ctx.feelssol_mint,
        &token1.pubkey(),
        30,
        10,
        79228162514264337593543950336u128,
        0,
    ).await?;
    
    println!("✓ Market with FeelsSOL created successfully: {}", market);
    
    println!("\n=== FeelsSOL Pairing Requirement Test Passed ===");
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_initial_buy_validation, |ctx: TestContext| async move {
    println!("\n=== Test: Initial Buy Validation ===");
    
    // Create token
    let creator = Keypair::new();
    ctx.airdrop(&creator.pubkey(), 1_000_000_000).await?;
    
    let token_mint = Keypair::new();
    let params = feels::instructions::MintTokenParams {
        ticker: "BUYTEST".to_string(),
        name: "Buy Test Token".to_string(),
        uri: "https://test.com".to_string(),
    };
    
    let ix = feels_sdk::mint_token(
        creator.pubkey(),
        token_mint.pubkey(),
        ctx.feelssol_mint,
        params,
    )?;
    
    ctx.process_instruction(ix, &[&creator, &token_mint]).await?;
    println!("✓ Token minted");
    
    // Test 1: Try initial buy without enough FeelsSOL (should fail)
    let creator_feelssol = ctx.create_ata(&creator.pubkey(), &ctx.feelssol_mint).await?;
    
    let result = ctx.initialize_market(
        &creator,
        &ctx.feelssol_mint,
        &token_mint.pubkey(),
        30,
        10,
        79228162514264337593543950336u128,
        1_000_000_000, // 1 FeelsSOL (creator has 0)
    ).await;
    
    assert!(result.is_err(), "Should fail with insufficient balance");
    println!("✓ Initial buy with insufficient balance correctly rejected");
    
    // Test 2: Get FeelsSOL and retry with valid amount
    let creator_jitosol = ctx.create_ata(&creator.pubkey(), &ctx.jitosol_mint).await?;
    ctx.mint_to(&ctx.jitosol_mint, &creator_jitosol, &ctx.jitosol_authority, 500_000_000).await?;
    ctx.enter_feelssol(
        &creator,
        &creator_jitosol,
        &creator_feelssol,
        500_000_000,
    ).await?;
    
    let balance = ctx.get_token_balance(&creator_feelssol).await?;
    println!("✓ Creator has {} FeelsSOL", balance);
    
    // Now it should succeed
    let market = ctx.initialize_market(
        &creator,
        &ctx.feelssol_mint,
        &token_mint.pubkey(),
        30,
        10,
        79228162514264337593543950336u128,
        100_000_000, // 0.1 FeelsSOL
    ).await?;
    
    println!("✓ Market launched with valid initial buy");
    
    // Verify balance was deducted
    let balance_after = ctx.get_token_balance(&creator_feelssol).await?;
    assert_eq!(balance_after, balance - 100_000_000);
    println!("✓ FeelsSOL correctly deducted: {} -> {}", balance, balance_after);
    
    println!("\n=== Initial Buy Validation Test Passed ===");
    Ok::<(), Box<dyn std::error::Error>>(())
});