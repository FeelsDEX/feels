use crate::common::*;

#[tokio::test]
async fn test_can_create_test_suite() {
    let suite = TestSuite::new().await.expect("Failed to create test suite");
    assert_eq!(suite.program_id, feels::ID);
}

#[tokio::test] 
async fn test_can_airdrop_sol() {
    let mut suite = TestSuite::new().await.expect("Failed to create test suite");
    let recipient = Keypair::new();
    
    // Airdrop 1 SOL
    suite.airdrop(&recipient.pubkey(), 1_000_000_000)
        .await
        .expect("Failed to airdrop SOL");
    
    // Check balance
    let account = suite.get_account(&recipient.pubkey())
        .await
        .expect("Failed to get account")
        .expect("Account not found");
    
    assert!(account.lamports >= 1_000_000_000);
}