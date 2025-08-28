use anchor_lang::{
    prelude::*,
    solana_program::{program_pack::Pack, system_instruction::transfer},
};
use anchor_spl::token_2022::spl_token_2022::{self, extension::StateWithExtensions};
use feels_test_utils::{to_sdk_instruction, TestApp};
use feels_token_factory::{
    error::TokenFactoryError,
    state::{factory::TokenFactory, metadata::TokenMetadata},
};
use solana_sdk::signature::Signer;

use crate::{
    error::ProtocolError,
    tests::{InstructionBuilder, FACTORY_PROGRAM_PATH, PROGRAM_PATH},
};

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

#[tokio::test]
async fn test_create_token_via_factory_success() {
    let (mut app, _, _, factory_pda) = deploy_protocol_and_factory().await;

    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());

    // Test parameters
    let ticker = "TEST".to_string();
    let name = "Test Token".to_string();
    let symbol = "TST".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, recipient_token_account, token_pda, token_metadata_account) =
        InstructionBuilder::create_token(
            &recipient_pubkey,
            &payer_pubkey,
            ticker.clone(),
            name.clone(),
            symbol.clone(),
            decimals,
            initial_supply,
        );

    // Process the instruction
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Verify results using the returned PDAs
    let factory: TokenFactory = app.get_account_data(factory_pda).await.unwrap();
    assert_eq!(factory.tokens_created, 1);

    let metadata: TokenMetadata = app.get_account_data(token_metadata_account).await.unwrap();
    assert_eq!(metadata.ticker, ticker);
    assert_eq!(metadata.name, name);
    assert_eq!(metadata.symbol, symbol);
    assert_eq!(metadata.mint, token_pda);

    // Verify token mint and recipient account
    let mint_account = app.get_account(token_pda).await.unwrap();
    let mint_data = spl_token_2022::state::Mint::unpack(&mint_account.data).unwrap();
    assert_eq!(mint_data.decimals, decimals);
    assert_eq!(mint_data.supply, initial_supply);

    let recipient_account = app.get_account(recipient_token_account).await.unwrap();
    let token_account_data =
        StateWithExtensions::<spl_token_2022::state::Account>::unpack(&recipient_account.data)
            .unwrap()
            .base;
    assert_eq!(token_account_data.amount, initial_supply);
    assert_eq!(token_account_data.mint, token_pda);
    assert_eq!(token_account_data.owner, recipient_pubkey);
}

#[tokio::test]
async fn test_create_token_via_factory_fail_unauthorized() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());

    // Test parameters
    let ticker = "TEST".to_string();
    let name = "Test Token".to_string();
    let symbol = "TST".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    let fake_authority = solana_sdk::signer::keypair::Keypair::new();
    let fake_authority_pubkey = Pubkey::from(fake_authority.pubkey().to_bytes());

    // Create token instruction - PDAs calculated internally
    let (instruction, _, _, _) = InstructionBuilder::create_token(
        &recipient_pubkey,
        &fake_authority_pubkey,
        ticker.clone(),
        name.clone(),
        symbol.clone(),
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
        .process_instruction_as_signer(to_sdk_instruction(instruction), &fake_authority)
        .await;

    let anchor_error_code: u32 = ProtocolError::InvalidAuthority.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_reuse_mint() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    // Test parameters
    let ticker = "TEST".to_string();
    let name = "Test Token".to_string();
    let symbol = "TST".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _, _, _) = InstructionBuilder::create_token(
        &recipient_pubkey,
        &payer_pubkey,
        ticker.clone(),
        name.clone(),
        symbol.clone(),
        decimals,
        initial_supply,
    );

    // Process the first instruction
    app.process_instruction(to_sdk_instruction(instruction.clone()))
        .await
        .unwrap();

    // IMPORTANT: Advance the blockchain to get a new blockhash and be able to rerun the TX
    app.warp_forward_seconds(10).await;

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap_err();
}

#[tokio::test]
async fn test_create_token_via_factory_fail_invalid_token_format() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());

    // Empty ticker
    let ticker = "".to_string();
    let name = "Test Token".to_string();
    let symbol = "TST".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _, _, _) = InstructionBuilder::create_token(
        &recipient_pubkey,
        &payer_pubkey,
        ticker.clone(),
        name.clone(),
        symbol.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;

    let anchor_error_code: u32 = TokenFactoryError::TickerIsEmpty.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_ticker_not_alphanumeric() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());

    // Ticker not alphanumeric
    let ticker = "!!!".to_string();
    let name = "Test Token".to_string();
    let symbol = "TST".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _, _, _) = InstructionBuilder::create_token(
        &recipient_pubkey,
        &payer_pubkey,
        ticker.clone(),
        name.clone(),
        symbol.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;

    let anchor_error_code: u32 = TokenFactoryError::TickerNotAlphanumeric.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_ticker_too_long() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());

    let ticker = "AAAAAAAAAAAAAAAAAA".to_string(); // 17 chars, max is 12
    let name = "Test Token".to_string();
    let symbol = "TST".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _, _, _) = InstructionBuilder::create_token(
        &recipient_pubkey,
        &payer_pubkey,
        ticker.clone(),
        name.clone(),
        symbol.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;

    let anchor_error_code: u32 = TokenFactoryError::TickerTooLong.into();
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

    // Empty ticker
    let ticker = "Ticker".to_string();
    let name = "".to_string();
    let symbol = "TST".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _, _, _) = InstructionBuilder::create_token(
        &recipient_pubkey,
        &payer_pubkey,
        ticker.clone(),
        name.clone(),
        symbol.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction(to_sdk_instruction(instruction))
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

    // Empty ticker
    let ticker = "Ticker".to_string();
    let name = "A".repeat(40); // 40 chars, max is 32
    let symbol = "TST".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _, _, _) = InstructionBuilder::create_token(
        &recipient_pubkey,
        &payer_pubkey,
        ticker.clone(),
        name.clone(),
        symbol.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;

    let anchor_error_code: u32 = TokenFactoryError::NameTooLong.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_symbol_empty() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());

    // Empty ticker
    let ticker = "Ticker".to_string();
    let name = "Token Name".to_string();
    let symbol = "".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _, _, _) = InstructionBuilder::create_token(
        &recipient_pubkey,
        &payer_pubkey,
        ticker.clone(),
        name.clone(),
        symbol.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;

    let anchor_error_code: u32 = TokenFactoryError::SymbolIsEmpty.into();
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

    // Empty ticker
    let ticker = "Ticker".to_string();
    let name = "Token Name".to_string();
    let symbol = "A".repeat(40); // 40 chars, max is 12
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _, _, _) = InstructionBuilder::create_token(
        &recipient_pubkey,
        &payer_pubkey,
        ticker.clone(),
        name.clone(),
        symbol.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;

    let anchor_error_code: u32 = TokenFactoryError::SymbolTooLong.into();
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

    // Empty ticker
    let ticker = "Ticker".to_string();
    let name = "Test Token".to_string();
    let symbol = "TST".to_string();
    let decimals = 20u8; // Max is 18
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _, _, _) = InstructionBuilder::create_token(
        &recipient_pubkey,
        &payer_pubkey,
        ticker.clone(),
        name.clone(),
        symbol.clone(),
        decimals,
        initial_supply,
    );

    // Process the instruction
    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;

    let anchor_error_code: u32 = TokenFactoryError::DecimalsTooLarge.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}
