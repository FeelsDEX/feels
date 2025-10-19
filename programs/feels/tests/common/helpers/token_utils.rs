//! Low-level token operations and utilities

use super::super::*;
use solana_program_test::BanksClient;
use solana_sdk::transaction::Transaction;
use spl_token::instruction as token_instruction;

/// Create a mint account directly using BanksClient (low-level utility)
pub async fn create_mint_direct(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    decimals: u8,
) -> TestResult<Pubkey> {
    let mint = Keypair::new();
    let rent = banks_client.get_rent().await?;
    let lamports = rent.minimum_balance(82);

    let instructions = vec![
        solana_sdk::system_instruction::create_account(
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
        )?,
    ];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    transaction.sign(&[payer, &mint], recent_blockhash);
    banks_client.process_transaction(transaction).await?;

    Ok(mint.pubkey())
}

/// Create a token account directly using BanksClient (low-level utility)
pub async fn create_token_account_direct(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint: Pubkey,
    owner: Pubkey,
) -> TestResult<Pubkey> {
    let account = Keypair::new();
    let rent = banks_client.get_rent().await?;
    let lamports = rent.minimum_balance(165);

    let instructions = vec![
        solana_sdk::system_instruction::create_account(
            &payer.pubkey(),
            &account.pubkey(),
            lamports,
            165,
            &spl_token::id(),
        ),
        token_instruction::initialize_account(&spl_token::id(), &account.pubkey(), &mint, &owner)?,
    ];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    transaction.sign(&[payer, &account], recent_blockhash);
    banks_client.process_transaction(transaction).await?;

    Ok(account.pubkey())
}

/// Mint tokens directly using BanksClient (low-level utility)
pub async fn mint_to_direct(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint: Pubkey,
    account: Pubkey,
    amount: u64,
) -> TestResult<()> {
    let instruction = token_instruction::mint_to(
        &spl_token::id(),
        &mint,
        &account,
        &payer.pubkey(),
        &[],
        amount,
    )?;

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    transaction.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(transaction).await?;

    Ok(())
}