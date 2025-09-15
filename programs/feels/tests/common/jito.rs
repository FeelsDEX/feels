//! Jito integration utilities for tests
//!
//! This module provides utilities for integrating with JitoSOL in tests,
//! including mocking the Jito stake pool for local testing.

use super::*;
use anchor_lang::prelude::*;
use solana_program::program_pack::Pack;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
// Note: spl_stake_pool is not available in test environment
// These would be used for devnet/mainnet testing

// Jito mainnet constants
pub const JITOSOL_MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
pub const JITO_STAKE_POOL: Pubkey = pubkey!("Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb");
pub const JITO_SOL_DEPOSIT_AUTHORITY: Pubkey =
    pubkey!("6iQKfEyhr3bZMotVkW6beNZz5CPAkiwvgV2CTje9pVSS");

/// Mock JitoSOL configuration for local testing
pub struct MockJitoConfig {
    pub jitosol_mint: Keypair,
    pub stake_pool: Keypair,
    pub mint_authority: Keypair,
}

impl MockJitoConfig {
    /// Create a new mock Jito configuration
    pub fn new() -> Self {
        Self {
            jitosol_mint: Keypair::new(),
            stake_pool: Keypair::new(),
            mint_authority: Keypair::new(),
        }
    }
}

/// Setup mock JitoSOL mint for testing
pub async fn setup_mock_jitosol(ctx: &TestContext) -> TestResult<Pubkey> {
    // In test environment, we create a mock JitoSOL mint
    let jitosol_mint = Keypair::new();
    let jitosol_authority = Keypair::new();

    // Create the mint
    let payer_pubkey = ctx.payer().await;
    let rent = solana_program::sysvar::rent::Rent::default();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    let instructions = vec![
        solana_sdk::system_instruction::create_account(
            &payer_pubkey,
            &jitosol_mint.pubkey(),
            mint_rent,
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &jitosol_mint.pubkey(),
            &jitosol_authority.pubkey(),
            None,
            9, // JitoSOL has 9 decimals
        )?,
    ];

    // Get the payer keypair from the client
    let payer = match &*ctx.client.lock().await {
        TestClient::InMemory(client) => client.payer.insecure_clone(),
        TestClient::Devnet(client) => client.payer.insecure_clone(),
    };

    ctx.process_transaction(&instructions, &[&payer, &jitosol_mint])
        .await?;

    // Store the authority for later minting
    // Note: In a real implementation, you'd want to store this properly
    // For now, we'll use the context's jitosol_authority field

    Ok(jitosol_mint.pubkey())
}

/// Mint mock JitoSOL to a user account
/// This simulates the Jito stake pool deposit without needing the actual program
pub async fn mint_mock_jitosol(
    ctx: &TestContext,
    user_jitosol_account: &Pubkey,
    amount: u64,
) -> TestResult<()> {
    // In test environment, we just mint directly
    let ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &ctx.jitosol_mint,
        user_jitosol_account,
        &ctx.jitosol_authority.pubkey(),
        &[],
        amount,
    )?;

    ctx.process_instruction(ix, &[&ctx.jitosol_authority]).await
}

/// Get JitoSOL by staking SOL (for use with actual Jito stake pool on devnet/mainnet)
pub async fn get_jitosol_by_staking(
    _ctx: &TestContext,
    _user: &Keypair,
    _sol_amount: u64,
) -> TestResult<u64> {
    // This would be used on devnet/mainnet with actual Jito stake pool
    // For now, return an error indicating it's not supported in test environment
    Err("JitoSOL staking not supported in test environment. Use mock JitoSOL instead.".into())
}

/// Helper to create and fund a user with JitoSOL for testing
pub async fn create_user_with_jitosol(
    ctx: &TestContext,
    sol_amount: u64,
    jitosol_amount: u64,
) -> TestResult<(Keypair, Pubkey, Pubkey)> {
    // Create user
    let user = Keypair::new();
    ctx.airdrop(&user.pubkey(), sol_amount).await?;

    // Create JitoSOL account
    let user_jitosol = ctx.create_ata(&user.pubkey(), &ctx.jitosol_mint).await?;

    // Mint JitoSOL
    mint_mock_jitosol(ctx, &user_jitosol, jitosol_amount).await?;

    // Create FeelsSOL account
    let user_feelssol = ctx.create_ata(&user.pubkey(), &ctx.feelssol_mint).await?;

    Ok((user, user_jitosol, user_feelssol))
}

/// Test helper to enter FeelsSOL system with mock JitoSOL
pub async fn enter_feelssol_with_mock_jitosol(
    ctx: &TestContext,
    user: &Keypair,
    amount: u64,
) -> TestResult<(Pubkey, Pubkey)> {
    // Create accounts
    let user_jitosol = ctx.create_ata(&user.pubkey(), &ctx.jitosol_mint).await?;
    let user_feelssol = ctx.create_ata(&user.pubkey(), &ctx.feelssol_mint).await?;

    // Mint mock JitoSOL
    mint_mock_jitosol(ctx, &user_jitosol, amount).await?;

    // Enter FeelsSOL
    ctx.enter_feelssol(user, &user_jitosol, &user_feelssol, amount)
        .await?;

    Ok((user_jitosol, user_feelssol))
}
