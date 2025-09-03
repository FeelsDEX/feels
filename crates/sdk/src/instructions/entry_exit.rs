/// Entry and exit instruction builders for JitoSOL <-> FeelsSOL conversion
use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

/// Build instruction to enter the system (JitoSOL -> FeelsSOL)
pub fn enter_system(
    program_id: &Pubkey,
    user: &Pubkey,
    jitosol_mint: &Pubkey,
    feelssol_mint: &Pubkey,
    user_jitosol: &Pubkey,
    user_feelssol: &Pubkey,
    feelssol_state: &Pubkey,
    feelssol_vault: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Instruction {
    let accounts = feels::accounts::EntryExit {
        user: *user,
        user_jitosol: *user_jitosol,
        user_feelssol: *user_feelssol,
        jitosol_mint: *jitosol_mint,
        feelssol: *feelssol_state,
        feelssol_vault: *feelssol_vault,
        feelssol_mint: *feelssol_mint,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
    };

    let params = feels::EntryParams {
        amount_in,
        min_amount_out,
    };

    let data = feels::instruction::EnterSystem { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to exit the system (FeelsSOL -> JitoSOL)
pub fn exit_system(
    program_id: &Pubkey,
    user: &Pubkey,
    jitosol_mint: &Pubkey,
    feelssol_mint: &Pubkey,
    user_jitosol: &Pubkey,
    user_feelssol: &Pubkey,
    feelssol_state: &Pubkey,
    feelssol_vault: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Instruction {
    let accounts = feels::accounts::EntryExit {
        user: *user,
        user_jitosol: *user_jitosol,
        user_feelssol: *user_feelssol,
        jitosol_mint: *jitosol_mint,
        feelssol: *feelssol_state,
        feelssol_vault: *feelssol_vault,
        feelssol_mint: *feelssol_mint,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
    };

    let params = feels::ExitParams {
        amount_in,
        min_amount_out,
    };

    let data = feels::instruction::ExitSystem { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Helper to get FeelsSOL state PDA
pub fn get_feelssol_state(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"feelssol"], program_id)
}

/// Helper to get FeelsSOL vault PDA
pub fn get_feelssol_vault(program_id: &Pubkey, jitosol_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"feelssol_vault", jitosol_mint.as_ref()],
        program_id
    )
}