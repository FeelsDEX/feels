use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    system_program,
};

/// Build instruction to initialize the protocol
pub fn initialize_protocol(
    program_id: &Pubkey,
    protocol_state: &Pubkey,
    authority: &Pubkey,
    treasury: &Pubkey,
) -> Instruction {
    let accounts = feels::accounts::InitializeFeels {
        protocol_state: *protocol_state,
        authority: *authority,
        treasury: *treasury,
        system_program: system_program::ID,
    };
    
    let data = feels::instruction::InitializeFeels {};
    
    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to initialize FeelsSOL
pub fn initialize_feelssol(
    program_id: &Pubkey,
    feelssol: &Pubkey,
    feels_mint: &Pubkey,
    authority: &Pubkey,
    underlying_mint: Pubkey,
) -> Instruction {
    let accounts = feels::accounts::InitializeFeelsSOL {
        feelssol: *feelssol,
        feels_mint: *feels_mint,
        authority: *authority,
        token_program: spl_token_2022::ID,
        system_program: system_program::ID,
        rent: solana_sdk::sysvar::rent::ID,
    };
    
    let data = feels::instruction::InitializeFeelssol {
        underlying_mint,
    };
    
    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}