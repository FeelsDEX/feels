//! MVP smoke test: initialize protocol and create test context

use crate::common::*;

#[tokio::test]
async fn test_mvp_smoke_in_memory() {
    let ctx = TestContext::new(TestEnvironment::InMemory).await.unwrap();
    // Basic sanity: protocol is initialized, FeelsSOL mint exists
    assert_ne!(ctx.feelssol_mint, Pubkey::default());
}

