use anchor_lang::prelude::*;
use crate::common::context::TestContext;
use crate::common::helpers::MarketHelper;
use crate::common::mod as common;

#[tokio::test]
async fn test_update_floor_ratchets_up() -> common::TestResult<()> {
    let ctx = TestContext::new(common::environment::TestEnvironment::InMemory).await?;
    let mh = ctx.market_helper();

    // Create a custom token mint
    let creator = &ctx.accounts.market_creator;
    let project_mint = ctx.create_mint(&creator.pubkey(), 6).await?;

    // Initialize a market with FeelsSOL + custom token
    let market_id = mh.create_simple_market(&ctx.feelssol_mint, &project_mint.pubkey()).await?;
    let market: feels::state::Market = ctx.get_account(&market_id).await?.ok_or("market not found")?;

    // Derive buffer and vaults
    let (buffer, _) = sdk::find_buffer_address(&market_id);
    let (vault_0, _) = sdk::find_vault_0_address(&market.token_0, &market.token_1);
    let (vault_1, _) = sdk::find_vault_1_address(&market.token_0, &market.token_1);

    // Mint FeelsSOL to its vault and some project tokens to the project vault
    // FeelsSOL authority exists in TestContext
    let feelssol_is_token_0 = market.token_0 == ctx.feelssol_mint;
    let (feels_vault, proj_vault) = if feelssol_is_token_0 { (vault_0, vault_1) } else { (vault_1, vault_0) };

    // Create ATAs for vaults aren't needed; vaults are PDAs. We can mint directly to PDAs.
    ctx.mint_to(&ctx.feelssol_mint, &feels_vault, 5_000_000_000).await?; // 5e9
    ctx.mint_to(&project_mint.pubkey(), &proj_vault, 1_000_000_000).await?; // 1e9

    // Record old floor and run update_floor
    let before: feels::state::Market = ctx.get_account(&market_id).await?.ok_or("market")?;
    let ix = sdk::update_floor(market_id, buffer, vault_0, vault_1, project_mint.pubkey());
    ctx.process_instruction(ix, &[&creator]).await?;
    let after: feels::state::Market = ctx.get_account(&market_id).await?.ok_or("market")?;

    assert!(after.floor_tick >= before.floor_tick, "floor did not ratchet up");
    Ok(())
}

