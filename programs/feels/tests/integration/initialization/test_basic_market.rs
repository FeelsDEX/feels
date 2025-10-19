use crate::common::*;

// MVP: Ensure non-protocol tokens cannot create markets; basic FeelsSOL infra sanity

test_all_environments!(
    test_initialize_simple_market,
    |ctx: TestContext| async move {
        println!("\n=== Test: Basic Market Initialization ===");
        println!("This test verifies market initialization requirements");
        println!("Program ID: {}", PROGRAM_ID);
        println!("FeelsSOL mint: {}", ctx.feelssol_mint);

        // Test 1: Try to create a market with non-protocol token (should fail)
        println!("\n1. Testing market creation with non-protocol token...");
        let regular_token = ctx
            .create_mint(&ctx.accounts.market_creator.pubkey(), 6)
            .await?;
        println!("   Created regular SPL token: {}", regular_token.pubkey());

        let result = ctx
            .market_helper()
            .create_simple_market(&ctx.feelssol_mint, &regular_token.pubkey())
            .await;

        match result {
            Ok(_) => panic!("Market creation should have failed without protocol token!"),
            Err(e) => {
                println!("   [OK] Expected error: {}", e);
                println!("   Non-protocol tokens correctly rejected");
            }
        }

        // Test 2: Verify FeelsSOL token setup
        println!("\n2. Testing FeelsSOL token functionality...");
        let alice_feelssol = ctx
            .create_ata(&ctx.accounts.alice.pubkey(), &ctx.feelssol_mint)
            .await?;
        println!("   Created Alice's FeelsSOL ATA: {}", alice_feelssol);

        // Note: JitoSOL is a real mainnet token, we can't create ATAs for it in tests
        // In production, users would already have JitoSOL to convert to FeelsSOL
        println!("   Note: JitoSOL integration would be tested with mock tokens");

        println!("\n[OK] Market initialization requirements test passed!");
        println!("  - Non-protocol tokens are correctly rejected");
        println!("  - FeelsSOL infrastructure is set up correctly");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);

// Post-MVP tests for market styles, liquidity details, and duplicates are removed for now.
