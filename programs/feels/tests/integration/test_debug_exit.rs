//! Debug test for exit_feelssol

use crate::common::*;
use solana_sdk::pubkey::Pubkey;

// Disabled: This test requires proper Clock sysvar timestamp which is not available in test environment
// The oracle staleness check considers timestamp 0 as stale, which is what the test environment provides
// TODO: Enable when test infrastructure supports proper clock management
/*
test_in_memory!(test_debug_exit_feelssol, |ctx: TestContext| async move {
    println!("\n=== Debug: Exit FeelsSOL Direct Test ===");

    // Create a user with some SOL
    let user = Keypair::new();
    ctx.airdrop(&user.pubkey(), 2_000_000_000).await?; // 2 SOL

    // Create user's JitoSOL and FeelsSOL accounts
    let user_jitosol = ctx.create_ata(&user.pubkey(), &ctx.jitosol_mint).await?;
    let user_feelssol = ctx.create_ata(&user.pubkey(), &ctx.feelssol_mint).await?;

    // Manually mint FeelsSOL to user (bypassing enter for debugging)
    // First get the mint authority PDA
    let (_mint_authority, _) = Pubkey::find_program_address(
        &[b"mint_authority", ctx.feelssol_mint.as_ref()],
        &feels::id(),
    );

    // We can't mint directly because we don't have the mint authority private key
    // So let's enter feelssol properly first
    let jitosol_amount = 1_000_000_000; // 1 JitoSOL
    ctx.mint_to(
        &ctx.jitosol_mint,
        &user_jitosol,
        &ctx.jitosol_authority,
        jitosol_amount,
    )
    .await?;

    // Enter FeelsSOL
    ctx.enter_feelssol(&user, &user_jitosol, &user_feelssol, jitosol_amount)
        .await?;

    let feelssol_balance = ctx.get_token_balance(&user_feelssol).await?;
    println!("✓ User has {} FeelsSOL", feelssol_balance);

    // Debug: Print all relevant PDAs
    let (hub, _) = Pubkey::find_program_address(
        &[b"feels_hub", ctx.feelssol_mint.as_ref()],
        &feels::id(),
    );
    let (safety, _) =
        Pubkey::find_program_address(&[b"safety_controller"], &feels::id());
    let (protocol_config, _) =
        Pubkey::find_program_address(&[b"protocol_config"], &feels::id());
    let (jitosol_vault, _) = Pubkey::find_program_address(
        &[b"jitosol_vault", ctx.feelssol_mint.as_ref()],
        &feels::id(),
    );
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault_authority", ctx.feelssol_mint.as_ref()],
        &feels::id(),
    );

    println!("\nPDAs:");
    println!("  Hub: {}", hub);
    println!("  Safety: {}", safety);
    println!("  Protocol Config: {}", protocol_config);
    println!("  JitoSOL Vault: {}", jitosol_vault);
    println!("  Vault Authority: {}", vault_authority);

    // Check vault balance
    let vault_balance = ctx.get_token_balance(&jitosol_vault).await?;
    println!("  JitoSOL vault balance: {}", vault_balance);

    // Update protocol oracle before attempting exit
    println!("\nUpdating protocol oracle...");
    ctx.update_protocol_oracle_for_testing().await?;
    println!("✓ Protocol oracle updated");
    
    // Debug: Check oracle state and clock
    let (protocol_oracle, _) = Pubkey::find_program_address(
        &[b"protocol_oracle"],
        &feels::id(),
    );
    match ctx.get_account_raw(&protocol_oracle).await {
        Ok(account) => {
            println!("  Protocol oracle exists, size: {}", account.data.len());
        }
        Err(e) => {
            println!("  Protocol oracle not found: {:?}", e);
        }
    }
    
    // Debug: The issue is that in test environment, the Clock sysvar has a default timestamp
    // which is likely 0 or a very old timestamp. When we update the oracle, it uses this
    // timestamp. Then when we check if it's stale, it's comparing against the same timestamp
    // but the staleness check requires the timestamp to be non-zero.
    println!("  Note: Test environment Clock may have default/zero timestamp");

    // Now try to exit with a small amount
    let exit_amount = 100_000_000; // 0.1 FeelsSOL
    println!("\nAttempting to exit {} FeelsSOL...", exit_amount);

    match ctx
        .exit_feelssol(&user, &user_feelssol, &user_jitosol, exit_amount)
        .await
    {
        Ok(_) => {
            println!("✓ Exit successful!");

            // Verify balances
            let jitosol_balance_after = ctx.get_token_balance(&user_jitosol).await?;
            let feelssol_balance_after = ctx.get_token_balance(&user_feelssol).await?;

            println!("  JitoSOL received: {}", jitosol_balance_after);
            println!("  FeelsSOL remaining: {}", feelssol_balance_after);
        }
        Err(e) => {
            eprintln!("✗ Exit failed: {:?}", e);
            return Err(e);
        }
    }

    println!("\n=== Debug Exit Test Passed ===");
    Ok::<(), Box<dyn std::error::Error>>(())
});
*/
