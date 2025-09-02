pub mod authority_transfer;
pub mod create_token;
pub mod deposit;
pub mod initialize;
pub mod update_protocol;

const PROGRAM_PATH: &str = "../../target/deploy/feels_protocol.so";
const FACTORY_PROGRAM_PATH: &str = "../../target/deploy/feels_token_factory.so";

use anchor_client::solana_sdk::{program_pack::Pack, signature::Keypair};
use anchor_lang::{prelude::*, system_program, InstructionData};
use anchor_spl::{
    associated_token::spl_associated_token_account,
    token::spl_token,
    token_2022::spl_token_2022::{self, extension::StateWithExtensions},
};

pub struct InstructionBuilder;

pub const TEST_KEYPAIR_PATH: &str = "../../test_keypair.json";
pub const PROTOCOL_KEYPAIR_PATH: &str = "../../target/deploy/feels_protocol-keypair.json";
pub const FACTORY_KEYPAIR_PATH: &str = "../../target/deploy/feels_token_factory-keypair.json";
pub const FEELSSOL_CONTROLLER_KEYPAIR_PATH: &str =
    "../../target/deploy/feelssol_controller-keypair.json";

pub const PROTOCOL_PDA_SEED: &[u8] = b"protocol";
pub const TREASURY_PDA_SEED: &[u8] = b"treasury";
pub const FACTORY_PDA_SEED: &[u8] = b"factory";
pub const FEELSSOL_PDA_SEED: &[u8] = b"feelssol";
pub const VAULT_PDA_SEED: &[u8] = b"vault";

pub const JITOSOL_MINT: &str = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";
pub const JITO_STAKE_POOL: &str = "Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb";

// Example secret key that gives a Pubkey that starts with `Fee1s`.
pub const FEELS_PRIVATE_KEY: [u8; 32] = [
    208, 250, 243, 217, 178, 15, 248, 65, 233, 94, 242, 229, 196, 92, 156, 153, 172, 164, 14, 45,
    147, 20, 212, 158, 3, 235, 20, 9, 75, 178, 205, 35,
];

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
                token_factory: feels_token_factory::id(),
                feelssol_controller: Pubkey::new_unique(),
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
        symbol: String,
        name: String,
        uri: String,
        decimals: u8,
        initial_supply: u64,
        invalid_factory: bool,
    ) -> (
        anchor_lang::solana_program::instruction::Instruction,
        Pubkey,
    ) {
        let program_id = crate::id();
        let factory_program_id = feels_token_factory::id();

        // Calculate PDAs using seeds
        let (protocol_pda, _) = Pubkey::find_program_address(&[PROTOCOL_PDA_SEED], &program_id);
        let (factory_pda, _) =
            Pubkey::find_program_address(&[FACTORY_PDA_SEED], &factory_program_id);

        let recipient_token =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                recipient,
                token_mint,
                &spl_token_2022::id(),
            );

        let accounts = crate::accounts::CreateToken {
            protocol: protocol_pda,
            factory: factory_pda,
            token_mint: *token_mint,
            recipient_token,
            recipient: *recipient,
            authority: *payer,
            token_factory: if invalid_factory {
                Pubkey::new_unique()
            } else {
                factory_program_id
            },
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
                symbol,
                name,
                uri,
                decimals,
                initial_supply,
            }
            .data(),
        };

        (instruction, recipient_token)
    }
}

pub fn get_token_balance(
    program: &anchor_client::Program<&Keypair>,
    token_account: &Pubkey,
) -> u64 {
    match program.rpc().get_account(token_account) {
        Ok(account_info) => {
            match spl_token::state::Account::unpack(&account_info.data) {
                Ok(token_account_data) => token_account_data.amount,
                Err(_) => 0, // Account exists but isn't a valid token account
            }
        }
        Err(_) => 0, // Account doesn't exist
    }
}

pub fn get_token2022_balance(
    program: &anchor_client::Program<&Keypair>,
    token_account: &Pubkey,
) -> u64 {
    match program.rpc().get_account(token_account) {
        Ok(account_info) => {
            // First try with extensions (the proper Token2022 way)
            match StateWithExtensions::<spl_token_2022::state::Account>::unpack(&account_info.data)
            {
                Ok(account_state) => account_state.base.amount,
                Err(_) => {
                    // Fallback: try without extensions
                    match spl_token_2022::state::Account::unpack(&account_info.data) {
                        Ok(token_account_data) => token_account_data.amount,
                        Err(_) => 0, // Return 0 instead of panicking
                    }
                }
            }
        }
        Err(_) => 0, // Return 0 if account doesn't exist instead of panicking
    }
}
