//! Test helpers

use anchor_lang::prelude::*;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use solana_program_test::BanksClient;
use spl_token::instruction as token_instruction;
use feels::instructions::*;

pub async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    decimals: u8,
) -> Pubkey {
    let mint = Keypair::new();
    let rent = banks_client.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(82);
    
    let instructions = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            lamports,
            82,
            &spl_token::id(),
        ),
        token_instruction::initialize_mint(
            &spl_token::id(),
            &mint.pubkey(),
            &payer.pubkey(),
            None,
            decimals,
        ).unwrap(),
    ];
    
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[payer, &mint], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
    
    mint.pubkey()
}

pub async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint: Pubkey,
    owner: Pubkey,
) -> Pubkey {
    let account = Keypair::new();
    let rent = banks_client.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(165);
    
    let instructions = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            &account.pubkey(),
            lamports,
            165,
            &spl_token::id(),
        ),
        token_instruction::initialize_account(
            &spl_token::id(),
            &account.pubkey(),
            &mint,
            &owner,
        ).unwrap(),
    ];
    
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[payer, &account], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
    
    account.pubkey()
}

pub async fn mint_to(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint: Pubkey,
    account: Pubkey,
    amount: u64,
) {
    let instruction = token_instruction::mint_to(
        &spl_token::id(),
        &mint,
        &account,
        &payer.pubkey(),
        &[],
        amount,
    ).unwrap();
    
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

pub fn find_market_address(token_0: &Pubkey, token_1: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"market", token_0.as_ref(), token_1.as_ref()],
        &feels::id(),
    )
}

pub fn find_buffer_address(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"buffer", market.as_ref()],
        &feels::id(),
    )
}

pub fn mint_token_instruction(
    creator: Pubkey,
    token_mint: Pubkey,
    feelssol_mint: Pubkey,
    params: MintTokenParams,
) -> Instruction {
    // This would normally come from the SDK
    // For now, create a simplified version
    Instruction {
        program_id: feels::id(),
        accounts: vec![], // Add accounts as needed
        data: vec![], // Add serialized data
    }
}

pub fn initialize_market_instruction(
    creator: Pubkey,
    token_0: Pubkey,
    token_1: Pubkey,
    feelssol_mint: Pubkey,
    params: InitializeMarketParams,
) -> Instruction {
    Instruction {
        program_id: feels::id(),
        accounts: vec![], // Add accounts as needed
        data: vec![], // Add serialized data
    }
}

pub fn deploy_initial_liquidity_instruction(
    deployer: Pubkey,
    market: Pubkey,
    token_0: Pubkey,
    token_1: Pubkey,
    feelssol_mint: Pubkey,
    params: DeployInitialLiquidityParams,
) -> Instruction {
    Instruction {
        program_id: feels::id(),
        accounts: vec![], // Add accounts as needed
        data: vec![], // Add serialized data
    }
}