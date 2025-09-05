use anchor_client::solana_sdk::system_program;
use anchor_lang::{prelude::*, solana_program::system_instruction::transfer, InstructionData};

use anchor_spl::{
    associated_token::spl_associated_token_account,
    token_interface::{Mint, TokenAccount},
};
use feels_test_utils::{
    constants::{FACTORY_PDA_SEED, FACTORY_PROGRAM_PATH, PROTOCOL_PROGRAM_PATH},
    to_sdk_instruction, TestApp,
};
use feels_token_factory::error::TokenFactoryError;
use solana_sdk::signature::Signer;

use crate::{error::ProtocolError, tests::InstructionBuilder};

// Helper to create a TestApp that initializes both the protocol and the factory
async fn deploy_protocol_and_factory() -> (TestApp, Pubkey, Pubkey, Pubkey) {
    let mut app = TestApp::new_with_programs(vec![
        (crate::id(), PROTOCOL_PROGRAM_PATH),
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
    let (factory_pda, _) =
        Pubkey::find_program_address(&[FACTORY_PDA_SEED], &feels_token_factory::id());

    let accounts = feels_token_factory::accounts::Initialize {
        token_factory: factory_pda,
        payer: payer_pubkey,
        system_program: system_program::ID,
    };

    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: feels_token_factory::id(),
        accounts: accounts.to_account_metas(None),
        data: feels_token_factory::instruction::Initialize {
            feels_protocol: crate::id(),
        }
        .data(),
    };

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    (app, protocol_pda, treasury_pda, factory_pda)
}

#[tokio::test]
async fn test_create_token_via_factory_success() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();

    // Create a random token mint and recipient
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        decimals,
        initial_supply,
        false,
    );

    // Create the token
    app.process_instruction_with_multiple_signers(
        to_sdk_instruction(instruction),
        &app.context.payer.insecure_clone(),
        &[&token_mint],
    )
    .await
    .unwrap();

    // Read the token information and metadata
    let mint_account: Mint = app.get_account_data(token_mint_pubkey).await.unwrap();
    assert_eq!(mint_account.decimals, decimals);
    assert_eq!(mint_account.supply, initial_supply);
    // We removed the authority to mint more
    assert_eq!(mint_account.mint_authority, None.into());

    // Finally verify that we minted the correct amount to the recipient
    let recipient_token_account = spl_associated_token_account::get_associated_token_address(
        &recipient_pubkey,
        &token_mint_pubkey,
    );

    let recipient_token_account_data: TokenAccount =
        app.get_account_data(recipient_token_account).await.unwrap();

    assert_eq!(recipient_token_account_data.amount, initial_supply);
    assert_eq!(recipient_token_account_data.mint, token_mint_pubkey);
    assert_eq!(recipient_token_account_data.owner, recipient_pubkey);
}

#[tokio::test]
async fn test_create_token_via_factory_fail_reuse_mint() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();

    // Create a random token mint and recipient
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        decimals,
        initial_supply,
        false,
    );

    // Create the token
    app.process_instruction_with_multiple_signers(
        to_sdk_instruction(instruction.clone()),
        &app.context.payer.insecure_clone(),
        &[&token_mint],
    )
    .await
    .unwrap();

    // Move forward in time to get a new blockhash
    app.warp_forward_seconds(1000).await;

    app.process_instruction_with_multiple_signers(
        to_sdk_instruction(instruction),
        &app.context.payer.insecure_clone(),
        &[&token_mint],
    )
    .await
    .unwrap_err();
}

#[tokio::test]
async fn test_create_token_via_factory_fail_protocol_paused() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Update the protocol to pause it
    let pause_instruction = InstructionBuilder::update_protocol(
        &payer_pubkey,
        Some(0),
        Some(0),
        Some(true),
        Some(false),
    );

    app.process_instruction(to_sdk_instruction(pause_instruction))
        .await
        .unwrap();

    // Create token instruction
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        decimals,
        initial_supply,
        false,
    );

    // Process the instruction - should fail because protocol is paused
    let result = app
        .process_instruction_with_multiple_signers(
            to_sdk_instruction(instruction),
            &app.context.payer.insecure_clone(),
            &[&token_mint],
        )
        .await;

    let anchor_error_code: u32 = ProtocolError::ProtocolPaused.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_create_token_via_factory_fail_invalid_factory() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Update the protocol to pause it
    let pause_instruction = InstructionBuilder::update_protocol(
        &payer_pubkey,
        Some(0),
        Some(0),
        Some(true),
        Some(false),
    );

    app.process_instruction(to_sdk_instruction(pause_instruction))
        .await
        .unwrap();

    // Create token instruction
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        decimals,
        initial_supply,
        true,
    );

    // Process the instruction - should fail because an invalid factory was passed
    let result = app
        .process_instruction_with_multiple_signers(
            to_sdk_instruction(instruction),
            &app.context.payer.insecure_clone(),
            &[&token_mint],
        )
        .await;

    let anchor_error_code: u32 = ProtocolError::InvalidTokenFactory.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
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
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    let fake_authority = solana_sdk::signer::keypair::Keypair::new();
    let fake_authority_pubkey = Pubkey::from(fake_authority.pubkey().to_bytes());

    // Create token instruction - PDAs calculated internally
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &fake_authority_pubkey,
        decimals,
        initial_supply,
        false,
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
async fn test_create_token_via_factory_fail_invalid_decimals() {
    let (mut app, _, _, _) = deploy_protocol_and_factory().await;
    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    let decimals = 20u8; // Max is 18
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, _) = InstructionBuilder::create_token(
        &token_mint_pubkey,
        &recipient_pubkey,
        &payer_pubkey,
        decimals,
        initial_supply,
        false,
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
