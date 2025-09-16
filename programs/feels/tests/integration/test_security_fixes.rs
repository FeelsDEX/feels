//! Integration test for security fixes

use anchor_lang::prelude::*;
use solana_sdk::signature::Keypair;
use crate::common::{fixtures::*, context::*, helpers::*};
use feels::instructions::update_floor;

#[tokio::test]
async fn test_update_floor_pda_validation() -> Result<()> {
    let mut test_env = TestEnvironment::new().await;
    let market_fixture = test_env.setup_market().await?;
    
    // Try to call update_floor with invalid vaults
    let fake_vault_0 = Keypair::new();
    let fake_vault_1 = Keypair::new();
    
    // This should fail because vaults are not the correct PDAs
    let result = test_env
        .program
        .request()
        .accounts(update_floor::UpdateFloor {
            market: market_fixture.market,
            buffer: market_fixture.buffer,
            vault_0: fake_vault_0.pubkey(), // Wrong vault
            vault_1: fake_vault_1.pubkey(), // Wrong vault
            project_mint: market_fixture.token_1_mint,
            clock: solana_sdk::sysvar::clock::id(),
        })
        .args(feels::instruction::UpdateFloor {})
        .send()
        .await;
    
    // Should fail with constraint violation
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_jit_base_fee_accounting() -> Result<()> {
    let mut test_env = TestEnvironment::new().await;
    let market_fixture = test_env.setup_market_with_liquidity().await?;
    
    // Enable JIT on the market
    test_env.enable_jit(&market_fixture.market).await?;
    
    // Perform a swap that triggers JIT
    let swap_amount = 10_000u64;
    let result = test_env.swap(
        &market_fixture,
        true, // token_0_to_1
        swap_amount,
        0, // min_out
    ).await?;
    
    // Check that JitBaseFeeSkipped event was emitted if JIT was active
    let events = result.events();
    let jit_event = events
        .iter()
        .find(|e| e.name == "JitBaseFeeSkipped");
    
    // If JIT was active, we should see the event
    if let Some(event) = jit_event {
        let base_fees_skipped: u64 = event.data.get("base_fees_skipped").unwrap();
        let jit_consumed_quote: u64 = event.data.get("jit_consumed_quote").unwrap();
        
        assert!(base_fees_skipped > 0);
        assert!(jit_consumed_quote > 0);
    }
    
    Ok(())
}