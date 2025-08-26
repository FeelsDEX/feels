use anchor_lang::{prelude::*, system_program, InstructionData};

pub struct InstructionBuilder;

const PROTOCOL_PDA_SEED: &[u8] = b"protocol";
const TREASURY_PDA_SEED: &[u8] = b"treasury";

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
}
