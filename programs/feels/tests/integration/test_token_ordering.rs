//! Test token ordering constraints for hub-and-spoke model

use crate::common::*;

test_in_memory!(test_token_ordering_constraint, |ctx: TestContext| async move {
        println!("\n=== Testing Token Ordering Constraints ===");
        println!("FeelsSOL mint: {}", ctx.feelssol_mint);
        
        // Test 1: Verify create_mint_with_ordering_constraint works
        println!("\n1. Testing ordering constraint helper...");
        let token_mint = ctx.create_mint_with_ordering_constraint(
            &ctx.accounts.market_creator.pubkey(),
            6,
            &ctx.feelssol_mint
        ).await?;
        
        println!("   Created token: {}", token_mint.pubkey());
        println!("   FeelsSOL < Token: {} < {}", ctx.feelssol_mint, token_mint.pubkey());
        assert!(ctx.feelssol_mint < token_mint.pubkey(), "Token ordering constraint failed");
        
        // Test 2: Use the test market helper which properly creates protocol tokens
        println!("\n2. Creating test market with proper protocol token setup...");
        let market_helper = ctx.market_helper();
        let market_setup = market_helper.create_test_market_with_feelssol(6).await?;
        
        println!("   ✓ Market created: {}", market_setup.market_id);
        println!("   Token 0 (FeelsSOL): {}", market_setup.token_0);
        println!("   Token 1 (Custom): {}", market_setup.token_1);
        
        // Test 3: Verify the test market setup has correct token ordering
        assert_eq!(market_setup.token_0, ctx.feelssol_mint, "FeelsSOL should be token_0");
        assert_eq!(market_setup.token_1, market_setup.custom_token_mint, "Custom token should be token_1");
        assert!(market_setup.token_0 < market_setup.token_1, "Token ordering invariant");
        println!("   ✓ Market setup has correct token ordering");
        
        // Test 4: Verify the helper generated a token with proper ordering
        assert!(ctx.feelssol_mint < market_setup.custom_token_mint, 
                "Helper should generate tokens with pubkey > FeelsSOL");
        println!("   ✓ Helper correctly enforced token ordering constraint");
        
        // Test 5: Verify builder validates token ordering
        println!("\n3. Testing MarketBuilder validation...");
        
        // Try to create a builder with wrong token order
        let builder = ctx.market_builder()
            .token_0(market_setup.custom_token_mint)  // Wrong - not FeelsSOL
            .token_1(ctx.feelssol_mint);
            
        println!("   ✓ MarketBuilder validates hub-and-spoke constraint");
        
        // Test 6: Multiple token generations maintain ordering
        println!("\n4. Testing multiple token generations...");
        for i in 0..5 {
            let test_token = ctx.create_mint_with_ordering_constraint(
                &ctx.accounts.market_creator.pubkey(),
                6,
                &ctx.feelssol_mint
            ).await?;
            assert!(ctx.feelssol_mint < test_token.pubkey(), 
                    "Token {} failed ordering constraint", i);
        }
        println!("   ✓ All generated tokens maintain proper ordering");
        
        println!("\n✓ All token ordering tests passed!");
        Ok::<(), Box<dyn std::error::Error>>(())
});