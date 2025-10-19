use anchor_lang::prelude::*;
use crate::common::mod as common;
use crate::common::context::TestContext;

#[tokio::test]
async fn test_redemptions_pause_and_resume_on_divergence() -> common::TestResult<()> {
    let ctx = TestContext::new(common::environment::TestEnvironment::InMemory).await?;
    // Initialize protocol is already called in TestContext::new

    // Create user token accounts (sketched via helpers)
    let user = &ctx.accounts.alice;
    let user_feelssol = ctx.common::helpers::create_token_account(&ctx, &ctx.feelssol_mint, &user.pubkey()).await?;
    let user_jitosol = ctx.common::helpers::create_token_account(&ctx, &ctx.jitosol_mint, &user.pubkey()).await?;

    // Mint some FeelsSOL by depositing JitoSOL (mint some JitoSOL to user first)
    ctx.common::helpers::mint_to(&ctx.jitosol_mint, &user_jitosol, 1_000_000_000).await?;
    ctx.enter_feelssol(user, &user_jitosol, &user_feelssol, 500_000_000).await?;

    // Diverge oracle rates twice to trigger pause
    let payer = match &*ctx.client.lock().await { common::client::TestClient::InMemory(c) => c.payer.insecure_clone(), common::client::TestClient::Devnet(c) => c.payer.insecure_clone() };
    let payer_pubkey = payer.pubkey();
    let native = 1u128 << 64;
    let dex = 2u128 << 64; // 2x divergence = 10000 bps
    let ix1 = sdk::update_dex_twap(payer_pubkey, dex, 1800, 1, Pubkey::default());
    ctx.process_instruction(ix1, &[&payer]).await?;
    let ix2 = sdk::update_native_rate(payer_pubkey, native);
    ctx.process_instruction(ix2, &[&payer]).await?;

    // Attempt exit should fail due to pause
    let res = ctx.exit_feelssol(user, &user_feelssol, &user_jitosol, 100_000_000).await;
    assert!(res.is_err(), "Exit should be paused on divergence");

    // Clear divergence twice to resume (set dex = native)
    let ix3 = sdk::update_dex_twap(payer_pubkey, native, 1800, 1, Pubkey::default());
    ctx.process_instruction(ix3, &[&payer]).await?;
    let ix4 = sdk::update_native_rate(payer_pubkey, native);
    ctx.process_instruction(ix4, &[&payer]).await?;

    // Exit should succeed now
    ctx.exit_feelssol(user, &user_feelssol, &user_jitosol, 100_000_000).await?;

    Ok(())
}

