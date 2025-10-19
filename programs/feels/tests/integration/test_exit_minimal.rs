//! Minimal test for exit_feelssol debugging

use crate::common::*;
use solana_sdk::signature::{Keypair, Signer};

#[tokio::test]
async fn test_exit_feelssol_minimal() -> TestResult<()> {
    println!("\n=== Minimal Exit FeelsSOL Test ===");
    
    // Create test context
    let ctx = TestContext::new(TestEnvironment::InMemory).await?;
    
    // Create user
    let user = Keypair::new();
    ctx.airdrop(&user.pubkey(), 2_000_000_000).await?;
    
    // Create ATAs
    let user_jitosol = ctx.create_ata(&user.pubkey(), &ctx.jitosol_mint).await?;
    let user_feelssol = ctx.create_ata(&user.pubkey(), &ctx.feelssol_mint).await?;
    
    // Mint JitoSOL to user
    let jitosol_amount = 1_000_000_000;
    ctx.mint_to(
        &ctx.jitosol_mint,
        &user_jitosol,
        &ctx.jitosol_authority,
        jitosol_amount,
    )
    .await?;
    
    // Enter FeelsSOL
    println!("Entering FeelsSOL...");
    ctx.enter_feelssol(&user, &user_jitosol, &user_feelssol, jitosol_amount).await?;
    
    let feelssol_balance = ctx.get_token_balance(&user_feelssol).await?;
    println!("[OK] User has {} FeelsSOL", feelssol_balance);
    
    // Now try to exit
    let exit_amount = 100_000_000;
    println!("\nAttempting to exit {} FeelsSOL...", exit_amount);
    
    match ctx.exit_feelssol(&user, &user_feelssol, &user_jitosol, exit_amount).await {
        Ok(_) => {
            println!("[OK] Exit successful!");
            Ok(())
        },
        Err(e) => {
            eprintln!("[ERROR] Exit failed: {:?}", e);
            Err(e)
        }
    }
}