//! Basic integration test for unified Market account
//! This tests the consolidated Market structure without relying on MarketField/MarketManager

use anchor_lang::prelude::*;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use feels::state::{Market, DomainWeights};
use feels_core::constants::Q64;

#[tokio::test]
async fn test_unified_market_initialization() {
    // Create program test environment
    let program_id = feels::ID;
    let mut program_test = ProgramTest::new(
        "feels",
        program_id,
        processor!(feels::entry),
    );
    
    // Start test runtime
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Generate PDAs
    let market_key = Keypair::new();
    let token_0_mint = Keypair::new();
    let token_1_mint = Keypair::new();
    let buffer_key = Keypair::new();
    
    // Create initialization params
    let params = feels::instructions::InitializeUnifiedMarketParams {
        initial_sqrt_price: Q64, // 1.0
        domain_weights: DomainWeights {
            w_s: 3333,
            w_t: 3333,
            w_l: 3334,
            w_tau: 0,
        },
        base_fee_bps: 30,
        max_fee_bps: 300,
    };
    
    // Build initialize instruction
    let ix = feels::instruction::initialize_unified_market(
        &program_id,
        &market_key.pubkey(),
        &buffer_key.pubkey(),
        &token_0_mint.pubkey(),
        &token_1_mint.pubkey(),
        &payer.pubkey(),
        params,
    );
    
    // Execute transaction
    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    
    let result = banks_client.process_transaction(tx).await;
    assert!(result.is_ok(), "Failed to initialize unified market");
    
    // Fetch and verify market account
    let market_account = banks_client
        .get_account(market_key.pubkey())
        .await
        .expect("get_account")
        .expect("market not found");
    
    let market = Market::try_from_slice(&market_account.data).expect("deserialize");
    
    // Verify market state
    assert!(market.is_initialized);
    assert!(!market.is_paused);
    assert_eq!(market.sqrt_price, Q64);
    assert_eq!(market.S, Q64);
    assert_eq!(market.T, Q64);
    assert_eq!(market.L, Q64);
    assert_eq!(market.w_s, 3333);
    assert_eq!(market.w_t, 3333);
    assert_eq!(market.w_l, 3334);
    assert_eq!(market.w_tau, 0);
    assert_eq!(market.base_fee_bps, 30);
    assert_eq!(market.max_fee_bps, 300);
    
    println!("✅ Unified market initialized successfully");
}

#[tokio::test]
async fn test_unified_order_swap() {
    // This test would require more setup including token accounts,
    // but demonstrates the structure for testing unified orders
    
    println!("✅ Unified order swap test placeholder");
}