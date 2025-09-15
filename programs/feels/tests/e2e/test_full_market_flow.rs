//! Devnet/Localnet E2E: mint protocol token, initialize market, deploy liquidity

use crate::common::*;

test_devnet!(test_full_market_flow_devnet, |ctx: TestContext| async move {
    let setup = ctx.market_helper().create_test_market_with_feelssol(6).await?;
    assert_ne!(setup.market_id, Pubkey::default());
    // TODO: add a small swap when SPL token balances are provisioned on devnet
    Ok::<(), Box<dyn std::error::Error>>(())
});

