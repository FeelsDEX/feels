use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{read_keypair_file, Keypair},
        signer::Signer as _,
        system_program, sysvar,
    },
    Client, Cluster,
};
use anchor_lang::{prelude::*, solana_program::system_instruction::transfer};
use anchor_spl::{
    associated_token::{self, spl_associated_token_account},
    token_2022::spl_token_2022::{
        self,
        extension::{BaseStateWithExtensions, PodStateWithExtensions, StateWithExtensions},
        pod::PodMint,
    },
    token_interface::spl_token_metadata_interface::state::TokenMetadata,
};
use feels_test_utils::{to_sdk_instruction, TestApp};
use feels_token_factory::error::TokenFactoryError;
use solana_sdk::signature::Signer;

use crate::{
    accounts,
    error::ProtocolError,
    tests::{InstructionBuilder, FACTORY_PROGRAM_PATH, PROGRAM_PATH},
};

const TEST_KEYPAIR_PATH: &str = "../../test_keypair.json";
const PROTOCOL_KEYPAIR_PATH: &str = "../../target/deploy/feels_protocol-keypair.json";
const FACTORY_KEYPAIR_PATH: &str = "../../target/deploy/feels_token_factory-keypair.json";

const PROTOCOL_PDA_SEED: &[u8] = b"protocol";
const TREASURY_PDA_SEED: &[u8] = b"treasury";
const FACTORY_PDA_SEED: &[u8] = b"factory";

// Helper to create a TestApp that initializes both the protocol and the factory
async fn deploy_protocol_and_factory() -> (TestApp, Pubkey, Pubkey, Pubkey) {
    let mut app = TestApp::new_with_programs(vec![
        (crate::id(), PROGRAM_PATH),
        (feels_token_factory::id(), FACTORY_PROGRAM_PATH),
    ])
    .await;

    let payer_pubkey = app.payer_pubkey();

    // Initialize the protocol
    let (instruction, protocol_pda, treasury_pda) =
        InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Initialize the token factory
    let (instruction, factory_pda) =
        feels_token_factory::instruction_builder::InstructionBuilder::initialize(
            &payer_pubkey,
            crate::id(),
        );
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    (app, protocol_pda, treasury_pda, factory_pda)
}

struct TestComponents {
    payer: Keypair,
    protocol_program_id: Pubkey,
    factory_program_id: Pubkey,
}

fn setup_test_components() -> TestComponents {
    let current_dir = std::env::current_dir().unwrap();

    // Wallet keypair path
    let wallet_path = current_dir.join(TEST_KEYPAIR_PATH);
    let payer = read_keypair_file(&wallet_path).unwrap();

    // Read program IDs from keypair files
    let protocol_program_keypair_path = current_dir.join(PROTOCOL_KEYPAIR_PATH);
    let protocol_program_keypair = read_keypair_file(&protocol_program_keypair_path)
        .expect("Protocol Program keypair should exist");
    let protocol_program_id = protocol_program_keypair.pubkey();

    let factory_program_keypair_path = current_dir.join(FACTORY_KEYPAIR_PATH);
    let factory_program_keypair = read_keypair_file(&factory_program_keypair_path)
        .expect("Factory Program keypair should exist");
    let factory_program_id = factory_program_keypair.pubkey();

    TestComponents {
        payer,
        protocol_program_id,
        factory_program_id,
    }
}

impl TestComponents {
    fn client(&self) -> Client<&Keypair> {
        Client::new_with_options(
            Cluster::Localnet,
            &self.payer,
            CommitmentConfig::confirmed(),
        )
    }

    fn payer_pubkey(&self) -> Pubkey {
        self.payer.pubkey()
    }

    fn programs(
        &self,
    ) -> (
        anchor_client::Program<&Keypair>,
        anchor_client::Program<&Keypair>,
    ) {
        let client = self.client();
        (
            client.program(self.protocol_program_id).unwrap(),
            client.program(self.factory_program_id).unwrap(),
        )
    }
}

fn deploy_protocol_and_factory_test_validator(
    protocol: &anchor_client::Program<&Keypair>,
    factory: &anchor_client::Program<&Keypair>,
) -> (Pubkey, Pubkey, Pubkey) {
    let (protocol_pda, _) = Pubkey::find_program_address(&[PROTOCOL_PDA_SEED], &crate::id());
    let (treasury_pda, _) = Pubkey::find_program_address(&[TREASURY_PDA_SEED], &crate::id());
    let (factory_pda, _) =
        Pubkey::find_program_address(&[FACTORY_PDA_SEED], &feels_token_factory::id());

    // If this fails it's OK because it means it has already been initialized. It's useful for rerunning tests
    // Initializes the feels protocol
    let result = protocol
        .request()
        .accounts(accounts::Initialize {
            protocol_state: protocol_pda,
            treasury: treasury_pda,
            authority: protocol.payer(),
            payer: protocol.payer(),
            system_program: system_program::ID,
        })
        .args(crate::instruction::Initialize {
            default_protocol_fee_rate: 2000,
            max_pool_fee_rate: 10000,
        })
        .send();
    match result {
        Ok(_) => {}
        Err(_) => {
            println!("Failed to initialize protocol. Protocol may already be initialized.");
        }
    }

    // Initialize the feels token factory
    let result = factory
        .request()
        .accounts(feels_token_factory::accounts::Initialize {
            payer: factory.payer(),
            system_program: system_program::ID,
            token_factory: factory_pda,
        })
        .args(feels_token_factory::instruction::Initialize {
            feels_protocol: protocol.id(),
        })
        .send();
    match result {
        Ok(_) => {}
        Err(_) => {
            println!("Failed to initialize factory. Factory may already be initialized.");
        }
    }

    (protocol_pda, treasury_pda, factory_pda)
}

#[test]
fn test_create_token_via_factory_success() {
    let test_components = setup_test_components();
    let (protocol_program, factory_program) = test_components.programs();

    // Deploy the protocol and factory
    let (protocol_pda, _, factory_pda) =
        deploy_protocol_and_factory_test_validator(&protocol_program, &factory_program);

    // Create a random token mint and recipient
    let token_mint = Keypair::new();
    let token_mint_pubkey = token_mint.pubkey();
    let recipient = Keypair::new();
    let recipient_pubkey = recipient.pubkey();
    let recipient_token_account =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &recipient_pubkey,
            &token_mint_pubkey,
            &spl_token_2022::id(),
        );

    // Test metadata parameters
    let symbol = "TEST".to_string();
    let name = "Test Token".to_string();
    let uri = "https://example.com/metadata.json".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create the token
    protocol_program
        .request()
        .accounts(accounts::CreateToken {
            protocol: protocol_pda,
            factory: factory_pda,
            token_mint: token_mint_pubkey,
            recipient_token_account,
            recipient: recipient_pubkey,
            authority: test_components.payer_pubkey(),
            token_factory_program: test_components.factory_program_id,
            token_program: spl_token_2022::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
            instructions: sysvar::instructions::ID,
        })
        .args(crate::instruction::CreateToken {
            symbol: symbol.clone(),
            name: name.clone(),
            uri: uri.clone(),
            decimals,
            initial_supply,
        })
        .signer(&token_mint)
        .signer(&test_components.payer)
        .send()
        .unwrap();

    // Read the token information and metadata
    let mint_account = protocol_program
        .rpc()
        .get_account(&token_mint_pubkey)
        .unwrap();
    let mint_data = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_account.data)
        .unwrap()
        .base;
    assert_eq!(mint_data.decimals, decimals);
    assert_eq!(mint_data.supply, initial_supply);
    // We removed the authority to mint more
    assert_eq!(mint_data.mint_authority, None.into());

    // Read the metadata and verify it matches
    let mint_state_with_extensions = PodStateWithExtensions::<PodMint>::unpack(&mint_account.data)
        .expect("Failed to unpack mint account data");
    let token_metadata = mint_state_with_extensions
        .get_variable_len_extension::<TokenMetadata>()
        .expect("Failed to get TokenMetadata extension");
    assert_eq!(token_metadata.name, name);
    assert_eq!(token_metadata.symbol, symbol);
    assert_eq!(token_metadata.uri, uri);

    // Finally verify that we minted the correct amount to the recipient
    let recipient_account = protocol_program
        .rpc()
        .get_account(&recipient_token_account)
        .unwrap();
    let token_account_data =
        StateWithExtensions::<spl_token_2022::state::Account>::unpack(&recipient_account.data)
            .unwrap()
            .base;
    assert_eq!(token_account_data.amount, initial_supply);
    assert_eq!(token_account_data.mint, token_mint_pubkey);
    assert_eq!(token_account_data.owner, recipient_pubkey);
}

#[test]
fn test_create_token_via_factory_fail_reuse_mint() {
    let test_components = setup_test_components();
    let (protocol_program, factory_program) = test_components.programs();

    // Deploy the protocol and factory
    let (protocol_pda, _, factory_pda) =
        deploy_protocol_and_factory_test_validator(&protocol_program, &factory_program);

    // Create a random token mint and recipient
    let token_mint = Keypair::new();
    let token_mint_pubkey = token_mint.pubkey();
    let recipient = Keypair::new();
    let recipient_pubkey = recipient.pubkey();
    let recipient_token_account =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &recipient_pubkey,
            &token_mint_pubkey,
            &spl_token_2022::id(),
        );

    // Test metadata parameters
    let symbol = "TEST".to_string();
    let name = "Test Token".to_string();
    let uri = "https://example.com/metadata.json".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create the token - first time should succeed
    protocol_program
        .request()
        .accounts(accounts::CreateToken {
            protocol: protocol_pda,
            factory: factory_pda,
            token_mint: token_mint_pubkey,
            recipient_token_account,
            recipient: recipient_pubkey,
            authority: test_components.payer_pubkey(),
            token_factory_program: test_components.factory_program_id,
            token_program: spl_token_2022::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
            instructions: sysvar::instructions::ID,
        })
        .args(crate::instruction::CreateToken {
            symbol: symbol.clone(),
            name: name.clone(),
            uri: uri.clone(),
            decimals,
            initial_supply,
        })
        .signer(&token_mint)
        .signer(&test_components.payer)
        .send()
        .unwrap();

    // Create token again - should fail because it's already been initialized
    protocol_program
        .request()
        .accounts(accounts::CreateToken {
            protocol: protocol_pda,
            factory: factory_pda,
            token_mint: token_mint_pubkey,
            recipient_token_account,
            recipient: recipient_pubkey,
            authority: test_components.payer_pubkey(),
            token_factory_program: test_components.factory_program_id,
            token_program: spl_token_2022::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
            instructions: sysvar::instructions::ID,
        })
        .args(crate::instruction::CreateToken {
            symbol: symbol.clone(),
            name: name.clone(),
            uri: uri.clone(),
            decimals,
            initial_supply,
        })
        .signer(&token_mint)
        .signer(&test_components.payer)
        .send()
        .unwrap_err();
}

#[tokio::test]
async fn test_create_token_via_factory_fail_unauthorized() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    // Test parameters
    let symbol = "TEST".to_string();
    let name = "Test Token".to_string();
    let uri = "https://example.com/metadata.json".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    let fake_authority = solana_sdk::signer::keypair::Keypair::new();
    let fake_authority_pubkey = Pubkey::from(fake_authority.pubkey().to_bytes());

    // Create token instruction - PDAs calculated internally
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &fake_authority_pubkey,
        symbol.clone(),
        name.clone(),
        uri.clone(),
        decimals,
        initial_supply,
    );

    // Fund the fake authority
    let fund_instruction = transfer(&payer_pubkey, &fake_authority_pubkey, 1_000_000);
    app.process_instruction(to_sdk_instruction(fund_instruction))
        .await
        .unwrap();

    // Process the instruction
    let result = app
        .process_instruction_with_multiple_signers(
            to_sdk_instruction(instruction),
            &fake_authority,
            &[&token_mint],
        )
        .await;

    let anchor_error_code: u32 = ProtocolError::InvalidAuthority.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_invalid_token_format() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    // Empty symbol
    let symbol = "".to_string();
    let name = "Test Token".to_string();
    let uri = "https://example.com/metadata.json".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        symbol.clone(),
        name.clone(),
        uri.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction_with_multiple_signers(
            to_sdk_instruction(instruction),
            &app.context.payer.insecure_clone(),
            &[&token_mint],
        )
        .await;

    let anchor_error_code: u32 = TokenFactoryError::SymbolIsEmpty.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_symbol_not_alphanumeric() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    // Symbol not alphanumeric
    let symbol = "!!!".to_string();
    let name = "Test Token".to_string();
    let uri = "https://example.com/metadata.json".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        symbol.clone(),
        name.clone(),
        uri.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction_with_multiple_signers(
            to_sdk_instruction(instruction),
            &app.context.payer.insecure_clone(),
            &[&token_mint],
        )
        .await;

    let anchor_error_code: u32 = TokenFactoryError::SymbolNotAlphanumeric.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_symbol_too_long() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    let symbol = "AAAAAAAAAAAAAAAAAA".to_string(); // 17 chars, max is 12
    let name = "Test Token".to_string();
    let uri = "https://example.com/metadata.json".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        symbol.clone(),
        name.clone(),
        uri.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction_with_multiple_signers(
            to_sdk_instruction(instruction),
            &app.context.payer.insecure_clone(),
            &[&token_mint],
        )
        .await
        .unwrap_err();

    let anchor_error_code: u32 = TokenFactoryError::SymbolTooLong.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result.to_string().contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_symbol_not_uppercase() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    let symbol = "TiCkEr".to_string();
    let name = "Test Token".to_string();
    let uri = "https://example.com/metadata.json".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        symbol.clone(),
        name.clone(),
        uri.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction_with_multiple_signers(
            to_sdk_instruction(instruction),
            &app.context.payer.insecure_clone(),
            &[&token_mint],
        )
        .await;

    let anchor_error_code: u32 = TokenFactoryError::SymbolNotUppercase.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_name_empty() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    let symbol = "TEST".to_string();
    let name = "".to_string();
    let uri = "https://example.com/metadata.json".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        symbol.clone(),
        name.clone(),
        uri.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction_with_multiple_signers(
            to_sdk_instruction(instruction),
            &app.context.payer.insecure_clone(),
            &[&token_mint],
        )
        .await;

    let anchor_error_code: u32 = TokenFactoryError::NameIsEmpty.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_name_too_long() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    let symbol = "TEST".to_string();
    let name = "A".repeat(40); // 40 chars, max is 32
    let uri = "https://example.com/metadata.json".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        symbol.clone(),
        name.clone(),
        uri.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction_with_multiple_signers(
            to_sdk_instruction(instruction),
            &app.context.payer.insecure_clone(),
            &[&token_mint],
        )
        .await;

    let anchor_error_code: u32 = TokenFactoryError::NameTooLong.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_invalid_decimals() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    let symbol = "TEST".to_string();
    let name = "Test Token".to_string();
    let uri = "https://example.com/metadata.json".to_string();
    let decimals = 20u8; // Max is 18
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        symbol.clone(),
        name.clone(),
        uri.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction_with_multiple_signers(
            to_sdk_instruction(instruction),
            &app.context.payer.insecure_clone(),
            &[&token_mint],
        )
        .await;

    let anchor_error_code: u32 = TokenFactoryError::DecimalsTooLarge.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}
