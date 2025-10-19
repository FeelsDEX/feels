use crate::common::*;

test_in_memory!(
    test_can_create_test_context,
    |ctx: TestContext| async move {
        // Test context is automatically created and passed to the test
        assert_eq!(PROGRAM_ID, feels::ID);

        // Verify we have access to pre-configured accounts
        assert_ne!(ctx.accounts.alice.pubkey(), Pubkey::default());
        assert_ne!(ctx.accounts.bob.pubkey(), Pubkey::default());

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);

test_in_memory!(test_can_airdrop_sol, |ctx: TestContext| async move {
    let recipient = Keypair::new();

    // Airdrop 1 SOL
    ctx.airdrop(&recipient.pubkey(), constants::LAMPORTS_PER_SOL)
        .await?;

    // Check balance using the client
    let _balance = ctx
        .client
        .lock()
        .await
        .get_token_balance(&recipient.pubkey())
        .await
        .unwrap_or(0);

    // For SOL balance, we need to check the account directly
    let account_data = ctx
        .client
        .lock()
        .await
        .get_account_data(&recipient.pubkey())
        .await?;

    assert!(account_data.is_some(), "Account should exist after airdrop");

    Ok::<(), Box<dyn std::error::Error>>(())
});

test_all_environments!(test_create_and_use_token, |ctx: TestContext| async move {
    // Create a new token mint
    let mint = ctx
        .create_mint(&ctx.accounts.market_creator.pubkey(), 9)
        .await?;

    // Create associated token account for alice
    let alice_token_account = ctx
        .create_ata(&ctx.accounts.alice.pubkey(), &mint.pubkey())
        .await?;

    // Mint some tokens
    ctx.mint_to(
        &mint.pubkey(),
        &alice_token_account,
        &ctx.accounts.market_creator,
        1_000_000_000,
    )
    .await?;

    // Verify balance
    let balance = ctx.get_token_balance(&alice_token_account).await?;
    assert_eq!(balance, 1_000_000_000);

    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(
    test_pre_configured_accounts,
    |ctx: TestContext| async move {
        // All test accounts should have SOL
        let accounts = [
            &ctx.accounts.alice,
            &ctx.accounts.bob,
            &ctx.accounts.charlie,
            &ctx.accounts.market_creator,
            &ctx.accounts.fee_collector,
        ];

        for account in accounts {
            // Check that account has some SOL (was airdropped during setup)
            let account_data = ctx
                .client
                .lock()
                .await
                .get_account_data(&account.pubkey())
                .await?;

            assert!(
                account_data.is_some(),
                "Test account {} should exist",
                account.pubkey()
            );
        }

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);
