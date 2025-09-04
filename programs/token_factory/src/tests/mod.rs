pub mod initialize;

use anchor_lang::{prelude::*, system_program, InstructionData};
use feels_test_utils::constants::FACTORY_PDA_SEED;

pub struct InstructionBuilder;

impl InstructionBuilder {
    pub fn initialize(
        payer: &Pubkey,
        feels_protocol: Pubkey,
    ) -> (
        anchor_lang::solana_program::instruction::Instruction,
        Pubkey,
    ) {
        let program_id = crate::id();
        let (factory_pda, _) = Pubkey::find_program_address(&[FACTORY_PDA_SEED], &program_id);

        let accounts = crate::accounts::Initialize {
            token_factory: factory_pda,
            payer: *payer,
            system_program: system_program::ID,
        };

        let instruction = anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::Initialize { feels_protocol }.data(),
        };

        (instruction, factory_pda)
    }
}
