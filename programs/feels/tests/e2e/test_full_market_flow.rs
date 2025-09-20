//! Devnet/Localnet E2E: mint protocol token, initialize market, deploy liquidity

use crate::common::*;

test_devnet!(
    test_full_market_flow_devnet,
    |ctx: TestContext| async move {
        let setup = ctx
            .market_helper()
            .create_test_market_with_feelssol(6)
            .await?;
        assert_ne!(setup.market_id, Pubkey::default());

        // Test swap functionality with small amounts
        let alice = &ctx.accounts.alice;

        // Create token accounts for Alice
        let alice_feelssol_account = ctx.create_ata(&alice.pubkey(), &ctx.feelssol_mint).await?;
        let alice_token_account = ctx.create_ata(&alice.pubkey(), &setup.custom_token_mint).await?;

        // Mint some FeelsSOL to Alice for testing
        ctx.mint_to(
            &ctx.feelssol_mint,
            &alice_feelssol_account,
            &ctx.feelssol_authority,
            1_000_000, // 1 FeelsSOL (6 decimals)
        )
        .await?;

        // Verify Alice has FeelsSOL balance
        let alice_feelssol_balance = ctx.get_token_balance(&alice_feelssol_account).await?;
        assert!(
            alice_feelssol_balance > 0,
            "Alice should have FeelsSOL balance for swap"
        );

        // Perform a small swap: FeelsSOL -> Token
        let swap_amount = 100_000; // 0.1 FeelsSOL
        let minimum_out = 1; // Very small minimum to avoid slippage issues in test

        // Use the swap helper
        let swap_helper = ctx.swap_helper();
        let swap_result = swap_helper
            .swap(
                &setup.market_id,
                &ctx.feelssol_mint,
                &setup.custom_token_mint,
                swap_amount,
                alice,
            )
            .await?;

        // Verify swap results
        let alice_feelssol_balance_after = ctx.get_token_balance(&alice_feelssol_account).await?;
        let alice_token_balance_after = ctx.get_token_balance(&alice_token_account).await?;

        assert!(
            alice_feelssol_balance_after < alice_feelssol_balance,
            "Alice should have less FeelsSOL after swap"
        );
        assert!(
            alice_token_balance_after > 0,
            "Alice should have received tokens from swap"
        );

        println!("✓ Swap completed successfully");
        println!(
            "  FeelsSOL balance: {} -> {}",
            alice_feelssol_balance, alice_feelssol_balance_after
        );
        println!("  Token balance: {} -> {}", 0, alice_token_balance_after);

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);
