//! E2E test for token mint and launch lifecycle

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use feels::{
    constants::*,
    instructions::{
        MintTokenParams, RecipientList, DistributionRecipient,
        mint_token, launch_token,
    },
    state::{Buffer, Market},
};
use crate::common::*;

#[tokio::test]
async fn test_mint_and_launch_token() -> Result<()> {
    // Setup test environment
    let mut test = FeelsTestSuite::new().await;
    
    // Create token mint
    let token_mint = Keypair::new();
    let creator = test.funded_keypair().await?;
    
    // Prepare mint token params
    let mint_params = MintTokenParams {
        ticker: "TEST".to_string(),
        name: "Test Token".to_string(),
        uri: "https://test.com/token.json".to_string(),
        creator_amount: 100_000_000_000, // 100k tokens
        recipients: RecipientList {
            recipients: vec![],
        },
    };
    
    // Derive PDAs
    let (buffer_pda, _) = Pubkey::find_program_address(
        &[BUFFER_SEED, token_mint.pubkey().as_ref()],
        &feels::id(),
    );
    let (buffer_authority, _) = Pubkey::find_program_address(
        &[BUFFER_AUTHORITY_SEED, buffer_pda.as_ref()],
        &feels::id(),
    );
    
    // Create associated token accounts
    let creator_token = get_associated_token_address(&creator.pubkey(), &token_mint.pubkey());
    let buffer_token_vault = get_associated_token_address(&buffer_authority, &token_mint.pubkey());
    let buffer_feelssol_vault = get_associated_token_address(&buffer_authority, &test.feelssol_mint);
    
    // Create metadata PDA (commented out as metadata is disabled in mint_token)
    // let (metadata_pda, _) = Pubkey::find_program_address(
    //     &[
    //         b"metadata",
    //         mpl_token_metadata::ID.as_ref(),
    //         token_mint.pubkey().as_ref(),
    //     ],
    //     &mpl_token_metadata::ID,
    // );
    
    // Create mint token instruction
    let mint_ix = mint_token(
        &feels::id(),
        &creator.pubkey(),
        &token_mint.pubkey(),
        &creator_token,
        &buffer_pda,
        &buffer_token_vault,
        &buffer_feelssol_vault,
        &buffer_authority,
        &creator.pubkey(), // mint authority (temporary)
        &test.feelssol_mint,
        mint_params,
    )?;
    
    // Send mint token transaction
    let mut mint_tx = Transaction::new_with_payer(&[mint_ix], Some(&creator.pubkey()));
    mint_tx.sign(&[&creator, &token_mint], test.context.last_blockhash);
    test.context.banks_client.process_transaction(mint_tx).await?;
    
    // Verify token was minted
    let creator_token_account = test
        .context
        .banks_client
        .get_account(creator_token)
        .await?
        .expect("Creator token account should exist");
    let creator_token_data: TokenAccount = 
        TokenAccount::try_deserialize(&mut creator_token_account.data.as_slice())?;
    assert_eq!(creator_token_data.amount, 100_000_000_000);
    
    // Verify buffer was created
    let buffer_account = test
        .context
        .banks_client
        .get_account(buffer_pda)
        .await?
        .expect("Buffer should exist");
    let buffer: Buffer = Buffer::try_deserialize(&mut buffer_account.data.as_slice())?;
    assert_eq!(buffer.feelssol_mint, test.feelssol_mint);
    
    // Now create a market for this token
    let market = Keypair::new();
    let (vault_0, _) = Pubkey::find_program_address(
        &[VAULT_SEED, market.pubkey().as_ref(), test.feelssol_mint.as_ref()],
        &feels::id(),
    );
    let (vault_1, _) = Pubkey::find_program_address(
        &[VAULT_SEED, market.pubkey().as_ref(), token_mint.pubkey().as_ref()],
        &feels::id(),
    );
    
    // Initialize market (simplified - in real test would use proper instruction)
    test.initialize_market(
        &market,
        &test.feelssol_mint,
        &token_mint.pubkey(),
        100, // tick_spacing
        30,  // base_fee_bps
    ).await?;
    
    // Update buffer to point to this market
    // (In production, this would be done via a proper instruction)
    
    // Fund buffer with some FeelsSOL for launch
    test.mint_feelssol(&buffer_feelssol_vault, 1_000_000_000).await?; // 1000 FeelsSOL
    
    // Create tick arrays for the launch
    let tick_array_lower = test.create_tick_array(&market.pubkey(), -10000).await?;
    let tick_array_upper = test.create_tick_array(&market.pubkey(), 0).await?;
    
    // Launch token
    let launch_ix = launch_token(
        &feels::id(),
        &creator.pubkey(),
        &market.pubkey(),
        &buffer_pda,
        &buffer_token_vault,
        &buffer_feelssol_vault,
        &vault_0,
        &vault_1,
        &buffer_authority,
        &tick_array_lower,
        &tick_array_upper,
    )?;
    
    let mut launch_tx = Transaction::new_with_payer(&[launch_ix], Some(&creator.pubkey()));
    launch_tx.sign(&[&creator], test.context.last_blockhash);
    test.context.banks_client.process_transaction(launch_tx).await?;
    
    // Verify liquidity was deployed
    let market_account = test
        .context
        .banks_client
        .get_account(market.pubkey())
        .await?
        .expect("Market should exist");
    let market_data: Market = Market::try_deserialize(&mut market_account.data.as_slice())?;
    assert!(market_data.floor_liquidity > 0, "Market should have floor liquidity");
    
    println!("Token minted and launched successfully!");
    println!("Token mint: {}", token_mint.pubkey());
    println!("Buffer: {}", buffer_pda);
    println!("Market: {}", market.pubkey());
    println!("Floor liquidity: {}", market_data.floor_liquidity);
    
    Ok(())
}

// Helper to create instruction builders
fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    anchor_spl::associated_token::get_associated_token_address(wallet, mint)
}

fn mint_token(
    program_id: &Pubkey,
    creator: &Pubkey,
    token_mint: &Pubkey,
    creator_token: &Pubkey,
    buffer: &Pubkey,
    buffer_token_vault: &Pubkey,
    buffer_feelssol_vault: &Pubkey,
    buffer_authority: &Pubkey,
    mint_authority: &Pubkey,
    feelssol_mint: &Pubkey,
    params: MintTokenParams,
) -> Result<solana_sdk::instruction::Instruction> {
    let accounts = vec![
        AccountMeta::new(*creator, true),
        AccountMeta::new(*token_mint, false),
        AccountMeta::new(*creator_token, false),
        AccountMeta::new(*buffer, false),
        AccountMeta::new(*buffer_token_vault, false),
        AccountMeta::new(*buffer_feelssol_vault, false),
        AccountMeta::new_readonly(*buffer_authority, false),
        AccountMeta::new_readonly(*mint_authority, false),
        AccountMeta::new_readonly(*feelssol_mint, false),
        AccountMeta::new_readonly(anchor_spl::associated_token::ID, false),
        AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
    ];
    
    let mut data = vec![0u8; 8]; // discriminator will be set by anchor
    data.extend_from_slice(&anchor_lang::AnchorSerialize::try_to_vec(&params)?);
    
    Ok(solana_sdk::instruction::Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

fn launch_token(
    program_id: &Pubkey,
    launcher: &Pubkey,
    market: &Pubkey,
    buffer: &Pubkey,
    buffer_token_vault: &Pubkey,
    buffer_feelssol_vault: &Pubkey,
    vault_0: &Pubkey,
    vault_1: &Pubkey,
    buffer_authority: &Pubkey,
    tick_array_lower: &Pubkey,
    tick_array_upper: &Pubkey,
) -> Result<solana_sdk::instruction::Instruction> {
    let accounts = vec![
        AccountMeta::new(*launcher, true),
        AccountMeta::new(*market, false),
        AccountMeta::new(*buffer, false),
        AccountMeta::new(*buffer_token_vault, false),
        AccountMeta::new(*buffer_feelssol_vault, false),
        AccountMeta::new(*vault_0, false),
        AccountMeta::new(*vault_1, false),
        AccountMeta::new_readonly(*buffer_authority, false),
        AccountMeta::new(*tick_array_lower, false),
        AccountMeta::new(*tick_array_upper, false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
    ];
    
    Ok(solana_sdk::instruction::Instruction {
        program_id: *program_id,
        accounts,
        data: vec![0u8; 8], // discriminator only, no params
    })
}