use anchor_lang::{prelude::Pubkey, solana_program::system_instruction::transfer};
use feels_test_utils::{to_sdk_instruction, TestApp};
use solana_sdk::signature::Signer;

use crate::{
    error::ProtocolError,
    state::protocol::ProtocolState,
    tests::{InstructionBuilder, PROGRAM_PATH},
};

#[tokio::test]
async fn test_update_protocol_success() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, protocol_pda, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    let instruction = InstructionBuilder::update_protocol(
        &payer_pubkey,
        Some(2500),
        Some(7000),
        Some(true),
        Some(false),
    );

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    let protocol_state: ProtocolState = app.get_account_data(protocol_pda).await.unwrap();
    assert_eq!(protocol_state.default_protocol_fee_rate, 2500);
    assert_eq!(protocol_state.max_pool_fee_rate, 7000);
    assert!(protocol_state.paused);
    assert!(!protocol_state.pool_creation_allowed);
}

#[tokio::test]
async fn test_update_protocol_fails_invalid_authority() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, _, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Create and fund a new account to impersonate the authority
    let fake_authority = solana_sdk::signer::keypair::Keypair::new();
    let fake_authority_pubkey = Pubkey::from(fake_authority.pubkey().to_bytes());

    // Fund the fake authority account so it can pay transaction fees
    let fund_instruction = transfer(
        &payer_pubkey,
        &fake_authority_pubkey,
        1_000_000, // 0.001 SOL for transaction fees
    );
    app.process_instruction(to_sdk_instruction(fund_instruction))
        .await
        .unwrap();

    // Try to update now
    let instruction = InstructionBuilder::update_protocol(
        &fake_authority_pubkey,
        Some(2500),
        Some(7000),
        Some(true),
        Some(false),
    );

    let result = app
        .process_instruction_as_signer(to_sdk_instruction(instruction), &fake_authority)
        .await;

    assert!(result.is_err());
    let anchor_error_code: u32 = ProtocolError::InvalidAuthority.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_update_protocol_fails_invalid_fees() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, _, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    let instruction = InstructionBuilder::update_protocol(
        &payer_pubkey,
        Some(2500),
        Some(15000),
        Some(true),
        Some(false),
    );

    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;
    assert!(result.is_err());
    let anchor_error_code: u32 = ProtocolError::PoolFeeTooHigh.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}
