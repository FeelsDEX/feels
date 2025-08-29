pub mod initialize;

const PROGRAM_PATH: &str = "../../target/deploy/feelssol_controller.so";

use anchor_lang::{prelude::*, system_program, InstructionData};
use anchor_spl::token_2022::spl_token_2022;

pub struct InstructionBuilder;

const FEELSSOL_PDA_SEED: &[u8] = b"feelssol";

impl InstructionBuilder {
    pub fn initialize(
        payer: &Pubkey,
        token_mint_pubkey: Pubkey,
        underlying_mint: Pubkey,
        feels_protocol: Pubkey,
    ) -> (
        anchor_lang::solana_program::instruction::Instruction,
        Pubkey,
    ) {
        let program_id = crate::id();
        let (feelssol_pda, _) = Pubkey::find_program_address(&[FEELSSOL_PDA_SEED], &program_id);

        let accounts = crate::accounts::Initialize {
            feelssol: feelssol_pda,
            feels_mint: token_mint_pubkey,
            payer: *payer,
            system_program: system_program::ID,
            token_program: spl_token_2022::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        };

        let instruction = anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::Initialize {
                underlying_mint,
                feels_protocol,
            }
            .data(),
        };

        (instruction, feelssol_pda)
    }
}
