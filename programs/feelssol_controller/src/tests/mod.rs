pub mod initialize;

const PROGRAM_PATH: &str = "../../target/deploy/feelssol_controller.so";

use anchor_lang::{prelude::*, system_program, InstructionData};
use anchor_spl::token_2022::spl_token_2022;

pub struct InstructionBuilder;

pub const JITOSOL_MINT: &str = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";
pub const JITO_STAKE_POOL: &str = "Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb";
const FEELSSOL_PDA_SEED: &[u8] = b"feelssol";

// Example secret key that gives a Pubkey that starts with `Fee1s`.
pub const FEELS_PRIVATE_KEY: [u8; 32] = [
    208, 250, 243, 217, 178, 15, 248, 65, 233, 94, 242, 229, 196, 92, 156, 153, 172, 164, 14, 45,
    147, 20, 212, 158, 3, 235, 20, 9, 75, 178, 205, 35,
];

impl InstructionBuilder {
    pub fn initialize(
        payer: &Pubkey,
        token_mint_pubkey: Pubkey,
        underlying_mint: Pubkey,
        underlying_stake_pool: Pubkey,
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
                underlying_stake_pool,
                feels_protocol,
            }
            .data(),
        };

        (instruction, feelssol_pda)
    }
}
