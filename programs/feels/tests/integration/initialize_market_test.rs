use crate::common::*;

#[tokio::test]
async fn test_initialize_market() {
    let mut suite = TestSuite::new().await.expect("Failed to create test suite");
    
    // Create token mints
    let token_0 = suite.create_mint(&suite.payer.pubkey(), 6)
        .await
        .expect("Failed to create token 0");
    
    let token_1 = suite.create_mint(&suite.payer.pubkey(), 6)
        .await
        .expect("Failed to create token 1");
    
    // For now, just verify we can create mints
    let token_0_account = suite.get_account(&token_0.pubkey())
        .await
        .expect("Failed to get token 0 account")
        .expect("Token 0 account not found");
    
    assert_eq!(token_0_account.owner, spl_token::id());
}