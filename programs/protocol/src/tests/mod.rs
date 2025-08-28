pub mod authority_transfer;
pub mod create_token;
pub mod initialize;
pub mod update_protocol;

const PROGRAM_PATH: &str = "../../target/deploy/feels_protocol.so";
const FACTORY_PROGRAM_PATH: &str = "../../target/deploy/feels_token_factory.so";

use anchor_lang::{prelude::*, system_program, InstructionData};
use anchor_spl::{associated_token::spl_associated_token_account, token_2022::spl_token_2022};

pub struct InstructionBuilder;

const PROTOCOL_PDA_SEED: &[u8] = b"protocol";
const TREASURY_PDA_SEED: &[u8] = b"treasury";
const FACTORY_PDA_SEED: &[u8] = b"factory";
const TOKEN_METADATA_PDA_SEED: &[u8] = b"metadata";

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
        let (protocol_pda, _) = Pubkey::find_program_address(&[PROTOCOL_PDA_SEED], &program_id);
        let (treasury_pda, _) = Pubkey::find_program_address(&[TREASURY_PDA_SEED], &program_id);

        let accounts = crate::accounts::Initialize {
            protocol_state: protocol_pda,
            treasury: treasury_pda,
            authority: *payer,
            payer: *payer,
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

    pub fn update_protocol(
        payer: &Pubkey,
        new_default_protocol_fee_rate: Option<u16>,
        new_max_pool_fee_rate: Option<u16>,
        new_paused: Option<bool>,
        new_pool_creation_allowed: Option<bool>,
    ) -> anchor_lang::solana_program::instruction::Instruction {
        let program_id = crate::id();
        let (protocol_pda, _) = Pubkey::find_program_address(&[PROTOCOL_PDA_SEED], &program_id);

        let accounts = crate::accounts::UpdateProtocol {
            protocol_state: protocol_pda,
            authority: *payer,
        };

        anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::UpdateProtocol {
                new_default_protocol_fee_rate,
                new_max_pool_fee_rate,
                new_paused,
                new_pool_creation_allowed,
            }
            .data(),
        }
    }

    pub fn initiate_authority_transfer(
        payer: &Pubkey,
        new_authority: &Pubkey,
    ) -> anchor_lang::solana_program::instruction::Instruction {
        let program_id = crate::id();
        let (protocol_pda, _) = Pubkey::find_program_address(&[PROTOCOL_PDA_SEED], &program_id);

        let accounts = crate::accounts::InitiateAuthorityTransfer {
            protocol_state: protocol_pda,
            authority: *payer,
            new_authority: *new_authority,
        };

        anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::InitiateAuthorityTransfer {}.data(),
        }
    }

    pub fn cancel_authority_transfer(
        payer: &Pubkey,
    ) -> anchor_lang::solana_program::instruction::Instruction {
        let program_id = crate::id();
        let (protocol_pda, _) = Pubkey::find_program_address(&[PROTOCOL_PDA_SEED], &program_id);

        let accounts = crate::accounts::CancelAuthorityTransfer {
            protocol_state: protocol_pda,
            authority: *payer,
        };

        anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::CancelAuthorityTransfer {}.data(),
        }
    }

    pub fn accept_authority_transfer(
        payer: &Pubkey,
    ) -> anchor_lang::solana_program::instruction::Instruction {
        let program_id = crate::id();
        let (protocol_pda, _) = Pubkey::find_program_address(&[PROTOCOL_PDA_SEED], &program_id);
        let (treasury_pda, _) = Pubkey::find_program_address(&[TREASURY_PDA_SEED], &program_id);

        let accounts = crate::accounts::AcceptAuthorityTransfer {
            protocol_state: protocol_pda,
            treasury: treasury_pda,
            new_authority: *payer,
        };

        anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::AcceptAuthorityTransfer {}.data(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_token(
        token_mint: &Pubkey,
        recipient: &Pubkey,
        payer: &Pubkey,
        ticker: String,
        name: String,
        symbol: String,
        decimals: u8,
        initial_supply: u64,
    ) -> (
        anchor_lang::solana_program::instruction::Instruction,
        Pubkey,
        Pubkey,
    ) {
        let program_id = crate::id();
        let factory_program_id = feels_token_factory::id();

        // Calculate PDAs using seeds
        let (protocol_pda, _) = Pubkey::find_program_address(&[PROTOCOL_PDA_SEED], &program_id);
        let (factory_pda, _) =
            Pubkey::find_program_address(&[FACTORY_PDA_SEED], &factory_program_id);

        let (token_metadata_pda, _) = Pubkey::find_program_address(
            &[TOKEN_METADATA_PDA_SEED, token_mint.as_ref()],
            &factory_program_id,
        );

        let recipient_token_account =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                recipient,
                token_mint,
                &spl_token_2022::id(),
            );

        let accounts = crate::accounts::CreateToken {
            protocol: protocol_pda,
            factory: factory_pda,
            token_mint: *token_mint,
            token_metadata: token_metadata_pda,
            recipient_token_account,
            recipient: *recipient,
            authority: *payer,
            token_factory_program: factory_program_id,
            token_program: anchor_spl::token_2022::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
            instructions: anchor_lang::solana_program::sysvar::instructions::ID,
        };

        let instruction = anchor_lang::solana_program::instruction::Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::CreateToken {
                ticker,
                name,
                symbol,
                decimals,
                initial_supply,
            }
            .data(),
        };

        (instruction, recipient_token_account, token_metadata_pda)
    }
}
