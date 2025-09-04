pub mod initialize;

use anchor_lang::{prelude::*, system_program, InstructionData};
use feels_test_utils::constants::KEEPER_PDA_SEED;
pub struct InstructionBuilder;

impl InstructionBuilder {
    pub fn initialize(
        payer: &Pubkey,
        authority: &Pubkey,
        feelssol_to_lst_rate_numerator: u64,
        feelssol_to_lst_rate_denominator: u64,
    ) -> (
        anchor_lang::solana_program::instruction::Instruction,
        Pubkey,
    ) {
        let program_id = crate::id();
        let (keeper_pda, _) = Pubkey::find_program_address(&[KEEPER_PDA_SEED], &program_id);

        let accounts = crate::accounts::Initialize {
            keeper: keeper_pda,
            authority: *authority,
            payer: *payer,
            system_program: system_program::ID,
        };

        let instruction = anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::Initialize {
                feelssol_to_lst_rate_numerator,
                feelssol_to_lst_rate_denominator,
            }
            .data(),
        };

        (instruction, keeper_pda)
    }
}
