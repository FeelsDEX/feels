use anchor_lang::{prelude::*, system_program, InstructionData};

pub struct InstructionBuilder;

impl InstructionBuilder {
    pub fn initialize(
        payer: &Pubkey,
        default_protocol_fee_rate: u16,
        max_pool_fee_rate: u16,
    ) -> (
        anchor_lang::solana_program::instruction::Instruction,
        Pubkey,
        Pubkey,
    ) {
        let program_id = crate::id();
        let (protocol_pda, _) = Pubkey::find_program_address(&[b"protocol"], &program_id);
        let (treasury_pda, _) = Pubkey::find_program_address(&[b"treasury"], &program_id);

        let accounts = crate::accounts::Initialize {
            protocol_state: protocol_pda,
            treasury: treasury_pda,
            authority: *payer,
            system_program: system_program::ID,
        };

        let instruction = anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::Initialize {
                default_protocol_fee_rate,
                max_pool_fee_rate,
            }
            .data(),
        };

        (instruction, protocol_pda, treasury_pda)
    }
}
