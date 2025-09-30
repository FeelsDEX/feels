pub mod create_pool;

use anchor_lang::{prelude::*, system_program, InstructionData};
use anchor_spl::token::spl_token;
use feels_test_utils::constants::{POOL_PDA_SEED, TOKEN_VAULT_A_PDA_SEED, TOKEN_VAULT_B_PDA_SEED};

pub struct InstructionBuilder;

impl InstructionBuilder {
    pub fn create_pool(
        payer: &Pubkey,
        token_a: &Pubkey,
        token_b: &Pubkey,
        fee_bps: u16,
        protocol_fee_bps: u16,
        tick_spacing: u32,
        initial_sqrt_price: u128,
    ) -> (
        anchor_lang::solana_program::instruction::Instruction,
        Pubkey,
    ) {
        let program_id = crate::id();
        let (pool_pda, _) = Pubkey::find_program_address(
            &[
                POOL_PDA_SEED,
                token_a.key().as_ref(),
                token_b.key().as_ref(),
                &fee_bps.to_le_bytes(),
            ],
            &program_id,
        );
        let (token_vault_a_pda, _) = Pubkey::find_program_address(
            &[TOKEN_VAULT_A_PDA_SEED, pool_pda.key().as_ref()],
            &program_id,
        );
        let (token_vault_b_pda, _) = Pubkey::find_program_address(
            &[TOKEN_VAULT_B_PDA_SEED, pool_pda.key().as_ref()],
            &program_id,
        );

        let accounts = crate::accounts::CreatePool {
            pool: pool_pda,
            token_mint_a: *token_a,
            token_mint_b: *token_b,
            token_vault_a: token_vault_a_pda,
            token_vault_b: token_vault_b_pda,
            payer: *payer,
            system_program: system_program::ID,
            token_program: spl_token::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        };

        let instruction = anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::CreatePool {
                fee_bps,
                protocol_fee_bps,
                tick_spacing,
                initial_sqrt_price,
            }
            .data(),
        };

        (instruction, pool_pda)
    }
}
