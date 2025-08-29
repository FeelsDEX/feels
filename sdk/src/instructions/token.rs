use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, system_program};

/// Build instruction to create a new token using Token-2022
#[allow(clippy::too_many_arguments)]
pub fn create_token(
    program_id: &Pubkey,
    token_mint: &Pubkey,
    token_metadata: &Pubkey,
    authority: &Pubkey,
    authority_token_account: &Pubkey,
    ticker: String,
    name: String,
    symbol: String,
    decimals: u8,
    initial_supply: u64,
) -> Instruction {
    let accounts = feels::accounts::CreateToken {
        token_mint: *token_mint,
        token_metadata: *token_metadata,
        authority_token_account: *authority_token_account,
        authority: *authority,
        token_program: spl_token_2022::ID,
        associated_token_program: spl_associated_token_account::ID,
        system_program: system_program::ID,
        rent: solana_sdk::sysvar::rent::ID,
    };

    let data = feels::instruction::CreateToken {
        ticker,
        name,
        symbol,
        decimals,
        initial_supply,
    };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}
