//! Tests for JitoSOL integration
//!
//! These tests demonstrate how to use the mock JitoSOL infrastructure
//! for testing enter/exit FeelsSOL flows.

use crate::common::jito::*;
use crate::common::*;
use feels_sdk as sdk;
use solana_sdk::pubkey::Pubkey;

test_all_environments!(
    test_enter_feelssol_with_mock_jitosol,
    |ctx: TestContext| async move {
        println!("\n=== Test: Enter FeelsSOL with Mock JitoSOL ===");

        // Create a user
        let user = Keypair::new();
        ctx.airdrop(&user.pubkey(), 1_000_000_000).await?; // 1 SOL

        // Create user's JitoSOL and FeelsSOL accounts
        let user_jitosol = ctx.create_ata(&user.pubkey(), &ctx.jitosol_mint).await?;
        let user_feelssol = ctx.create_ata(&user.pubkey(), &ctx.feelssol_mint).await?;

        // Mint mock JitoSOL to user
        let jitosol_amount = 1_000_000_000; // 1 JitoSOL
        ctx.mint_to(
            &ctx.jitosol_mint,
            &user_jitosol,
            &ctx.jitosol_authority,
            jitosol_amount,
        )
        .await?;

        let jitosol_balance = ctx.get_token_balance(&user_jitosol).await?;
        println!("✓ User has {} JitoSOL", jitosol_balance);

        // Debug: Check FeelsSOL mint authority
        let (mint_authority, _) = Pubkey::find_program_address(
            &[b"mint_authority", ctx.feelssol_mint.as_ref()],
            &feels_sdk::program_id(),
        );
        println!("  Mint authority PDA: {}", mint_authority);

        // Check FeelsSOL mint details
        let mint_info = ctx.get_mint(&ctx.feelssol_mint).await?;
        println!("  FeelsSOL mint authority: {:?}", mint_info.mint_authority);
        println!("  FeelsSOL supply: {}", mint_info.supply);

        // Enter FeelsSOL system
        match ctx
            .enter_feelssol(&user, &user_jitosol, &user_feelssol, jitosol_amount)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error entering FeelsSOL: {:?}", e);
                return Err(e);
            }
        }

        // Verify balances
        let jitosol_balance_after = ctx.get_token_balance(&user_jitosol).await?;
        let feelssol_balance = ctx.get_token_balance(&user_feelssol).await?;

        assert_eq!(
            jitosol_balance_after, 0,
            "All JitoSOL should be transferred"
        );
        assert_eq!(
            feelssol_balance, jitosol_amount,
            "Should receive equal FeelsSOL"
        );

        println!("✓ Successfully entered FeelsSOL system");
        println!("  JitoSOL spent: {}", jitosol_amount);
        println!("  FeelsSOL received: {}", feelssol_balance);

        println!("\n=== Enter FeelsSOL Test Passed ===");
        Ok::<(), Box<dyn std::error::Error>>(())
    }
);

// Disabled: This test requires proper Clock sysvar timestamp which is not available in test environment
// The oracle staleness check considers timestamp 0 as stale, which is what the test environment provides
// TODO: Enable when test infrastructure supports proper clock management
/*
test_all_environments!(
    test_exit_feelssol_to_mock_jitosol,
    |ctx: TestContext| async move {
        println!("\n=== Test: Exit FeelsSOL to Mock JitoSOL ===");

        // Create user with JitoSOL and enter FeelsSOL
        let user = Keypair::new();
        ctx.airdrop(&user.pubkey(), 1_000_000_000).await?;

        let (user_jitosol, user_feelssol) =
            enter_feelssol_with_mock_jitosol(&ctx, &user, 1_000_000_000).await?;

        let feelssol_balance = ctx.get_token_balance(&user_feelssol).await?;
        println!("✓ User has {} FeelsSOL", feelssol_balance);

        // Debug: Check if safety controller exists
        let (safety_controller, _) =
            Pubkey::find_program_address(&[b"safety_controller"], &feels_sdk::program_id());
        match ctx.get_account_raw(&safety_controller).await {
            Ok(_) => println!("  Safety controller exists"),
            Err(e) => {
                eprintln!("  Safety controller does not exist: {:?}", e);
                eprintln!(
                    "  This may indicate protocol initialization didn't create safety controller"
                );
            }
        }

        // Check JitoSOL vault balance
        let (jitosol_vault, _) = Pubkey::find_program_address(
            &[b"jitosol_vault", ctx.feelssol_mint.as_ref()],
            &feels_sdk::program_id(),
        );
        let vault_balance = ctx.get_token_balance(&jitosol_vault).await?;
        println!("  JitoSOL vault balance: {}", vault_balance);
        if vault_balance == 0 {
            eprintln!("  WARNING: JitoSOL vault is empty! Exit will fail.");
        }

        // Update protocol oracle before attempting exit
        println!("\nUpdating protocol oracle...");
        ctx.update_protocol_oracle_for_testing().await?;
        println!("✓ Protocol oracle updated");

        // Exit FeelsSOL system
        let exit_amount = 500_000_000; // 0.5 FeelsSOL
        match ctx
            .exit_feelssol(&user, &user_feelssol, &user_jitosol, exit_amount)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error exiting FeelsSOL: {:?}", e);
                return Err(e);
            }
        }

        // Verify balances
        let jitosol_balance_after = ctx.get_token_balance(&user_jitosol).await?;
        let feelssol_balance_after = ctx.get_token_balance(&user_feelssol).await?;

        assert_eq!(
            jitosol_balance_after, exit_amount,
            "Should receive JitoSOL back"
        );
        assert_eq!(
            feelssol_balance_after,
            feelssol_balance - exit_amount,
            "FeelsSOL should be burned"
        );

        println!("✓ Successfully exited FeelsSOL system");
        println!("  FeelsSOL burned: {}", exit_amount);
        println!("  JitoSOL received: {}", jitosol_balance_after);
        println!("  FeelsSOL remaining: {}", feelssol_balance_after);

        println!("\n=== Exit FeelsSOL Test Passed ===");
        Ok::<(), Box<dyn std::error::Error>>(())
    }
);
*/

// Disabled: mint_token instruction is not fully implemented yet
// This test requires a complete implementation of protocol token minting
// TODO: Enable when mint_token is fully implemented
/*
test_all_environments!(
    test_market_launch_with_jito_integration,
    |ctx: TestContext| async move {
        println!("\n=== Test: Market Launch with JitoSOL Integration ===");

        // Create token creator
        let creator = Keypair::new();
        ctx.airdrop(&creator.pubkey(), 5_000_000_000).await?; // 5 SOL

        // Get FeelsSOL via mock JitoSOL
        let (_creator_jitosol, creator_feelssol) = enter_feelssol_with_mock_jitosol(
            &ctx,
            &creator,
            2_000_000_000, // 2 JitoSOL/FeelsSOL
        )
        .await?;

        let feelssol_balance = ctx.get_token_balance(&creator_feelssol).await?;
        println!("✓ Creator has {} FeelsSOL", feelssol_balance);

        // Create a protocol token using mint_token instruction
        // Keep generating keypairs until we get one that satisfies token ordering
        let token_mint = loop {
            let candidate = Keypair::new();
            if candidate.pubkey() > ctx.feelssol_mint {
                break candidate;
            }
            println!(
                "Generated token mint {} < FeelsSOL mint {}, retrying...",
                candidate.pubkey(),
                ctx.feelssol_mint
            );
        };

        println!(
            "Found valid token mint: {} > FeelsSOL mint {}",
            token_mint.pubkey(),
            ctx.feelssol_mint
        );

        // Mint a protocol token
        let mint_params = feels::instructions::MintTokenParams {
            ticker: "TEST".to_string(),
            name: "Test Token".to_string(),
            uri: "https://test.com/metadata.json".to_string(),
        };

        let mint_ix = sdk_compat::instructions::mint_token(
            creator.pubkey(),
            token_mint.pubkey(),
            ctx.feelssol_mint,
            creator_feelssol,
            mint_params,
        )?;

        ctx.process_instruction(mint_ix, &[&creator, &token_mint])
            .await?;

        println!("✓ Created protocol token: {}", token_mint.pubkey());

        // Verify escrow was created
        let (escrow_pda, _) = Pubkey::find_program_address(
            &[b"escrow", token_mint.pubkey().as_ref()],
            &sdk::program_id(),
        );

        match ctx.get_account_raw(&escrow_pda).await {
            Ok(account) => println!("✓ Escrow exists at: {}", escrow_pda),
            Err(e) => {
                println!("✗ Escrow not found at {}: {:?}", escrow_pda, e);
                return Err(format!("Escrow not found: {:?}", e).into());
            }
        }

        // For now, just verify that we can:
        // 1. Enter FeelsSOL via JitoSOL
        // 2. Create a protocol token
        // 3. The escrow is created correctly

        // These are the key integrations we're testing
        println!("✓ Successfully entered FeelsSOL via JitoSOL");
        println!("✓ Successfully created protocol token with escrow");

        // Verify the creator still has their FeelsSOL
        let feelssol_balance_after = ctx.get_token_balance(&creator_feelssol).await?;
        assert_eq!(
            feelssol_balance_after, feelssol_balance,
            "FeelsSOL balance should be unchanged"
        );

        // Note: Full market initialization with protocol tokens requires
        // additional setup that's beyond the scope of this integration test

        println!("\n=== Market Launch with JitoSOL Integration Test Passed ===");
        Ok::<(), Box<dyn std::error::Error>>(())
    }
);
*/

test_in_memory!(test_helper_functions, |ctx: TestContext| async move {
    println!("\n=== Test: JitoSOL Helper Functions ===");

    // Test create_user_with_jitosol helper
    let (user, user_jitosol, user_feelssol) = create_user_with_jitosol(
        &ctx,
        2_000_000_000, // 2 SOL
        1_000_000_000, // 1 JitoSOL
    )
    .await?;

    let jitosol_balance = ctx.get_token_balance(&user_jitosol).await?;

    println!("✓ Created user with:");
    println!("  User pubkey: {}", user.pubkey());
    println!("  JitoSOL balance: {}", jitosol_balance);
    println!("  JitoSOL account: {}", user_jitosol);
    println!("  FeelsSOL account: {}", user_feelssol);

    assert_eq!(jitosol_balance, 1_000_000_000);

    println!("\n=== Helper Functions Test Passed ===");
    Ok::<(), Box<dyn std::error::Error>>(())
});
