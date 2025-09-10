//! E2E test for positions with NFT metadata

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use mpl_token_metadata::ID as METADATA_PROGRAM_ID;
use feels::{
    constants::*,
    instructions::{
        open_position_with_metadata, close_position_with_metadata,
        OpenPositionWithMetadata, ClosePositionWithMetadata,
    },
    state::{Position, Market},
    error::FeelsError,
};
use crate::common::*;

#[tokio::test]
async fn test_position_with_metadata_lifecycle() -> Result<()> {
    // Setup test environment
    let mut test = FeelsTestSuite::new().await;
    
    // Create user with token balances
    let user = test.funded_keypair().await?;
    let user_token_0 = test.create_token_account(&user.pubkey(), &test.token_0).await?;
    let user_token_1 = test.create_token_account(&user.pubkey(), &test.token_1).await?;
    
    // Fund user accounts
    test.mint_tokens(&user_token_0, 1_000_000_000_000).await?; // 1M tokens
    test.mint_tokens(&user_token_1, 1_000_000_000_000).await?; // 1M tokens
    
    // Initialize market
    let market = test.create_initialized_market().await?;
    
    // Position parameters
    let tick_lower = -1000;
    let tick_upper = 1000;
    let liquidity_amount = 1_000_000_000u128; // 1B liquidity units
    
    // Create position mint
    let position_mint = Keypair::new();
    
    // Derive PDAs
    let (position_pda, _) = Pubkey::find_program_address(
        &[POSITION_SEED, position_mint.pubkey().as_ref()],
        &feels::id(),
    );
    let (metadata_pda, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            METADATA_PROGRAM_ID.as_ref(),
            position_mint.pubkey().as_ref(),
        ],
        &METADATA_PROGRAM_ID,
    );
    let position_token_account = get_associated_token_address(&user.pubkey(), &position_mint.pubkey());
    
    // Create tick arrays
    let tick_array_lower = test.create_tick_array(&market.pubkey(), tick_lower).await?;
    let tick_array_upper = test.create_tick_array(&market.pubkey(), tick_upper).await?;
    
    // Get market vaults
    let (vault_0, _) = Pubkey::find_program_address(
        &[VAULT_SEED, market.pubkey().as_ref(), test.token_0.as_ref()],
        &feels::id(),
    );
    let (vault_1, _) = Pubkey::find_program_address(
        &[VAULT_SEED, market.pubkey().as_ref(), test.token_1.as_ref()],
        &feels::id(),
    );
    
    println!("Opening position with metadata...");
    
    // Open position with metadata
    let open_ix = open_position_with_metadata_instruction(
        &feels::id(),
        &user.pubkey(),
        &market.pubkey(),
        &position_mint.pubkey(),
        &position_token_account,
        &position_pda,
        &metadata_pda,
        &user_token_0,
        &user_token_1,
        &vault_0,
        &vault_1,
        &tick_array_lower,
        &tick_array_upper,
        tick_lower,
        tick_upper,
        liquidity_amount,
    )?;
    
    let mut open_tx = Transaction::new_with_payer(&[open_ix], Some(&user.pubkey()));
    open_tx.sign(&[&user, &position_mint], test.context.last_blockhash);
    test.context.banks_client.process_transaction(open_tx).await?;
    
    // Verify position was created
    let position_account = test
        .context
        .banks_client
        .get_account(position_pda)
        .await?
        .expect("Position should exist");
    let position: Position = Position::try_deserialize(&mut position_account.data.as_slice())?;
    assert_eq!(position.owner, user.pubkey());
    assert_eq!(position.liquidity, liquidity_amount);
    assert_eq!(position.tick_lower, tick_lower);
    assert_eq!(position.tick_upper, tick_upper);
    
    // Verify position NFT was minted
    let position_token_acc = test
        .context
        .banks_client
        .get_account(position_token_account)
        .await?
        .expect("Position token account should exist");
    let position_token_data: TokenAccount = 
        TokenAccount::try_deserialize(&mut position_token_acc.data.as_slice())?;
    assert_eq!(position_token_data.amount, 1);
    assert_eq!(position_token_data.mint, position_mint.pubkey());
    
    // Verify metadata was created
    let metadata_account = test
        .context
        .banks_client
        .get_account(metadata_pda)
        .await?
        .expect("Metadata should exist");
    assert!(metadata_account.data.len() > 0, "Metadata should have data");
    
    println!("Position opened successfully with NFT metadata!");
    println!("Position mint: {}", position_mint.pubkey());
    println!("Position PDA: {}", position_pda);
    println!("Metadata: {}", metadata_pda);
    
    // Now close the position
    println!("\nClosing position with metadata...");
    
    // Get market authority
    let (market_authority, _) = Pubkey::find_program_address(
        &[MARKET_AUTHORITY_SEED, market.pubkey().as_ref()],
        &feels::id(),
    );
    
    let close_ix = close_position_with_metadata_instruction(
        &feels::id(),
        &user.pubkey(),
        &market.pubkey(),
        &position_mint.pubkey(),
        &position_token_account,
        &position_pda,
        &metadata_pda,
        &user_token_0,
        &user_token_1,
        &vault_0,
        &vault_1,
        &market_authority,
        &tick_array_lower,
        &tick_array_upper,
        0, // amount_0_min
        0, // amount_1_min
    )?;
    
    let mut close_tx = Transaction::new_with_payer(&[close_ix], Some(&user.pubkey()));
    close_tx.sign(&[&user], test.context.last_blockhash);
    test.context.banks_client.process_transaction(close_tx).await?;
    
    // Verify position was closed
    let position_account = test
        .context
        .banks_client
        .get_account(position_pda)
        .await?;
    assert!(position_account.is_none(), "Position account should be closed");
    
    // Verify position token was burned
    let position_token_acc = test
        .context
        .banks_client
        .get_account(position_token_account)
        .await?;
    assert!(position_token_acc.is_none() || {
        let acc = position_token_acc.unwrap();
        let token_data: TokenAccount = TokenAccount::try_deserialize(&mut acc.data.as_slice()).unwrap();
        token_data.amount == 0
    }, "Position token should be burned");
    
    // Verify metadata was removed
    let metadata_account = test
        .context
        .banks_client
        .get_account(metadata_pda)
        .await?;
    assert!(metadata_account.is_none() || metadata_account.unwrap().data.is_empty(), 
        "Metadata should be removed");
    
    println!("Position closed successfully and NFT metadata cleaned up!");
    
    Ok(())
}

// Helper functions
fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    anchor_spl::associated_token::get_associated_token_address(wallet, mint)
}

fn open_position_with_metadata_instruction(
    program_id: &Pubkey,
    provider: &Pubkey,
    market: &Pubkey,
    position_mint: &Pubkey,
    position_token_account: &Pubkey,
    position: &Pubkey,
    metadata: &Pubkey,
    provider_token_0: &Pubkey,
    provider_token_1: &Pubkey,
    vault_0: &Pubkey,
    vault_1: &Pubkey,
    lower_tick_array: &Pubkey,
    upper_tick_array: &Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_amount: u128,
) -> Result<solana_sdk::instruction::Instruction> {
    let accounts = vec![
        AccountMeta::new(*provider, true),
        AccountMeta::new(*market, false),
        AccountMeta::new(*position_mint, false),
        AccountMeta::new(*position_token_account, false),
        AccountMeta::new(*position, false),
        AccountMeta::new(*metadata, false),
        AccountMeta::new(*provider_token_0, false),
        AccountMeta::new(*provider_token_1, false),
        AccountMeta::new(*vault_0, false),
        AccountMeta::new(*vault_1, false),
        AccountMeta::new(*lower_tick_array, false),
        AccountMeta::new(*upper_tick_array, false),
        AccountMeta::new_readonly(METADATA_PROGRAM_ID, false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
    ];
    
    let mut data = vec![0u8; 8]; // discriminator
    data.extend_from_slice(&tick_lower.to_le_bytes());
    data.extend_from_slice(&tick_upper.to_le_bytes());
    data.extend_from_slice(&liquidity_amount.to_le_bytes());
    
    Ok(solana_sdk::instruction::Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

fn close_position_with_metadata_instruction(
    program_id: &Pubkey,
    owner: &Pubkey,
    market: &Pubkey,
    position_mint: &Pubkey,
    position_token_account: &Pubkey,
    position: &Pubkey,
    metadata: &Pubkey,
    owner_token_0: &Pubkey,
    owner_token_1: &Pubkey,
    vault_0: &Pubkey,
    vault_1: &Pubkey,
    market_authority: &Pubkey,
    lower_tick_array: &Pubkey,
    upper_tick_array: &Pubkey,
    amount_0_min: u64,
    amount_1_min: u64,
) -> Result<solana_sdk::instruction::Instruction> {
    let accounts = vec![
        AccountMeta::new(*owner, true),
        AccountMeta::new(*market, false),
        AccountMeta::new(*position_mint, false),
        AccountMeta::new(*position_token_account, false),
        AccountMeta::new(*position, false),
        AccountMeta::new(*metadata, false),
        AccountMeta::new(*owner_token_0, false),
        AccountMeta::new(*owner_token_1, false),
        AccountMeta::new(*vault_0, false),
        AccountMeta::new(*vault_1, false),
        AccountMeta::new_readonly(*market_authority, false),
        AccountMeta::new(*lower_tick_array, false),
        AccountMeta::new(*upper_tick_array, false),
        AccountMeta::new_readonly(METADATA_PROGRAM_ID, false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
    ];
    
    let mut data = vec![0u8; 8]; // discriminator
    data.extend_from_slice(&amount_0_min.to_le_bytes());
    data.extend_from_slice(&amount_1_min.to_le_bytes());
    
    Ok(solana_sdk::instruction::Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}