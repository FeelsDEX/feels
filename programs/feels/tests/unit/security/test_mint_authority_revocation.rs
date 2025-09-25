//! Test that mint and freeze authorities are properly transferred and revoked
//!
//! This test verifies the fix for the critical vulnerability where mint and freeze
//! authorities were not immediately revoked after token minting.
//!
//! NOTE: This test is currently disabled because it uses methods and types not yet available in the test framework

/*
use crate::common::*;

test_in_memory!(
    test_mint_authority_transferred_to_escrow,
    |ctx: TestContext| async move {
        println!("Testing mint authority transfer to escrow...");

        // Create token using mint_token instruction
        let creator = &ctx.accounts.market_creator;
        let token_mint = Keypair::new();
        let feelssol_mint = ctx.feelssol_mint;

        // Get escrow PDA
        let (escrow, _) = Pubkey::find_program_address(
            &[feels::constants::ESCROW_SEED, token_mint.pubkey().as_ref()],
            &feels::ID,
        );

        // Get escrow authority PDA
        let (escrow_authority, _) = Pubkey::find_program_address(
            &[feels::constants::ESCROW_AUTHORITY_SEED, escrow.as_ref()],
            &feels::ID,
        );

        // Mint the token
        ctx.mint_token_with_keypair(
            &token_mint,
            &creator,
            "TEST",
            "Test Token",
            "https://test.com",
        )
        .await?;

        // Fetch the mint account and check authorities
        let mint_account: Mint = ctx
            .get_account(&token_mint.pubkey())
            .await?
            .ok_or("Mint account not found")?;

        // Verify mint authority was transferred to escrow authority
        assert_eq!(
            mint_account.mint_authority.unwrap(),
            escrow_authority,
            "Mint authority should be transferred to escrow authority PDA"
        );

        // Verify freeze authority was transferred to escrow authority
        assert_eq!(
            mint_account.freeze_authority.unwrap(),
            escrow_authority,
            "Freeze authority should be transferred to escrow authority PDA"
        );

        // Verify creator no longer has any authorities
        assert_ne!(
            mint_account.mint_authority.unwrap(),
            creator,
            "Creator should no longer have mint authority"
        );
        assert_ne!(
            mint_account.freeze_authority.unwrap(),
            creator,
            "Creator should no longer have freeze authority"
        );

        println!("Mint and freeze authorities successfully transferred to protocol");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);

test_in_memory!(
    test_creator_cannot_mint_after_token_creation,
    |ctx: TestContext| async move {
        println!("Testing creator cannot mint additional tokens...");

        let creator = ctx.payer_pubkey();
        let token_mint = Keypair::new();

        // Mint the token
        ctx.mint_token_with_keypair(
            &token_mint,
            &creator,
            "SAFE",
            "Safe Token",
            "https://safe.com",
        )
        .await?;

        // Get creator's token account
        let creator_token_account = get_associated_token_address(&creator, &token_mint.pubkey());

        // Try to mint additional tokens as creator (should fail)
        let mint_to_ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            &token_mint.pubkey(),
            &creator_token_account,
            &creator,
            &[],
            1_000_000, // Try to mint 1M tokens
        )?;

        // This should fail because creator no longer has mint authority
        let result = ctx
            .execute_transaction_with_signers(&[mint_to_ix], &[])
            .await;

        assert!(
            result.is_err(),
            "Creator should not be able to mint additional tokens"
        );

        println!("Creator cannot mint additional tokens after token creation");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);

test_in_memory!(
    test_creator_cannot_freeze_accounts,
    |ctx: TestContext| async move {
        println!("Testing creator cannot freeze accounts...");

        let creator = ctx.payer_pubkey();
        let token_mint = Keypair::new();
        let victim = Keypair::new();

        // Mint the token
        ctx.mint_token_with_keypair(
            &token_mint,
            &creator,
            "NOFREEZE",
            "No Freeze Token",
            "https://nofreeze.com",
        )
        .await?;

        // Create a token account for victim
        let victim_token_account = get_associated_token_address(&victim.pubkey(), &token_mint.pubkey());
        ctx.create_associated_token_account(&victim.pubkey(), &token_mint.pubkey())
            .await?;

        // Try to freeze the victim's account as creator (should fail)
        let freeze_ix = spl_token::instruction::freeze_account(
            &spl_token::id(),
            &victim_token_account,
            &token_mint.pubkey(),
            &creator,
            &[],
        )?;

        // This should fail because creator no longer has freeze authority
        let result = ctx
            .execute_transaction_with_signers(&[freeze_ix], &[])
            .await;

        assert!(
            result.is_err(),
            "Creator should not be able to freeze accounts"
        );

        println!("Creator cannot freeze accounts after token creation");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);

test_in_memory!(
    test_authorities_fully_revoked_after_market_init,
    |ctx: TestContext| async move {
        println!("Testing authorities are fully revoked after market initialization...");

        let creator = ctx.payer_pubkey();
        let token_mint = Keypair::new();

        // Create token and market
        let setup = ctx
            .market_helper()
            .create_test_market_with_new_token_keypair(
                &token_mint,
                "FINAL",
                "Final Token",
                6,
            )
            .await?;

        // Fetch the mint account after market initialization
        let mint_account: Mint = ctx
            .get_account(&token_mint.pubkey())
            .await?
            .ok_or("Mint account not found")?;

        // Verify mint authority is fully revoked (None)
        assert!(
            mint_account.mint_authority.is_none(),
            "Mint authority should be fully revoked after market initialization"
        );

        // Verify freeze authority is fully revoked (None)
        assert!(
            mint_account.freeze_authority.is_none(),
            "Freeze authority should be fully revoked after market initialization"
        );

        println!("All authorities properly revoked after market initialization");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);

test_in_memory!(
    test_escrow_authority_can_be_used_before_market_init,
    |ctx: TestContext| async move {
        println!("Testing escrow authority controls token before market init...");

        let creator = ctx.payer_pubkey();
        let token_mint = Keypair::new();

        // Mint the token
        ctx.mint_token_with_keypair(
            &token_mint,
            &creator,
            "ESCROW",
            "Escrow Token",
            "https://escrow.com",
        )
        .await?;

        // Get escrow and escrow authority PDAs
        let (escrow, _) = Pubkey::find_program_address(
            &[crate::constants::ESCROW_SEED, token_mint.pubkey().as_ref()],
            &PROGRAM_ID,
        );

        let (escrow_authority, escrow_authority_bump) = Pubkey::find_program_address(
            &[crate::constants::ESCROW_AUTHORITY_SEED, escrow.as_ref()],
            &PROGRAM_ID,
        );

        // Fetch escrow account
        let escrow_account: PreLaunchEscrow = ctx
            .get_account(&escrow)
            .await?
            .ok_or("Escrow account not found")?;

        // Verify escrow authority bump is stored
        assert_eq!(
            escrow_account.escrow_authority_bump,
            escrow_authority_bump,
            "Escrow authority bump should be stored correctly"
        );

        // Verify the escrow authority PDA is properly derived
        let derived_authority = Pubkey::find_program_address(
            &[crate::constants::ESCROW_AUTHORITY_SEED, escrow.as_ref()],
            &PROGRAM_ID,
        ).0;

        assert_eq!(
            escrow_authority,
            derived_authority,
            "Escrow authority should be properly derived"
        );

        println!("Escrow authority properly controls token before market initialization");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);
*/
