use anchor_lang::{prelude::*, system_program, InstructionData};
use anchor_spl::{associated_token::spl_associated_token_account, token_2022::spl_token_2022};

pub struct InstructionBuilder;

const FACTORY_PDA_SEED: &[u8] = b"factory";

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

    pub fn create_token(
        payer: &Pubkey,
        token_mint: &Pubkey,
        recipient: &Pubkey,
        initial_supply: u64,
    ) -> anchor_lang::solana_program::instruction::Instruction {
        let program_id = crate::id();

        let (factory_pda, _) = Pubkey::find_program_address(&[FACTORY_PDA_SEED], &program_id);

        let recipient_token_account =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                recipient,
                token_mint,
                &spl_token_2022::id(),
            );

        let accounts = crate::accounts::CreateToken {
            factory: factory_pda,
            token_mint: *token_mint,
            recipient_token_account,
            recipient: *recipient,
            payer: *payer,
            token_program: spl_token_2022::id(),
            associated_token_program: spl_associated_token_account::id(),
            system_program: system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
            instructions: anchor_lang::solana_program::sysvar::instructions::ID,
        };

        anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::CreateToken {
                initial_supply,
                symbol: "TEST".to_string(),
                name: "TT".to_string(),
                uri: "h".to_string(),
                decimals: 9,
            }
            .data(),
        }
    }
}
