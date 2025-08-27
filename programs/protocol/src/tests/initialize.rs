use crate::{
    error::ProtocolError,
    instructions::{MAX_POOL_FEE_RATE, MAX_PROTOCOL_FEE_RATE},
    state::{protocol::ProtocolState, treasury::Treasury},
    tests::{InstructionBuilder, PROGRAM_PATH},
};
use feels_test_utils::{to_sdk_instruction, TestApp};

#[tokio::test]
async fn test_initialize_protocol_success() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, protocol_pda, treasury_pda) =
        InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    let protocol_state: ProtocolState = app.get_account_data(protocol_pda).await.unwrap();
    let treasury: Treasury = app.get_account_data(treasury_pda).await.unwrap();

    // Protocol state assertions
    assert_eq!(protocol_state.authority, payer_pubkey);
    assert_eq!(protocol_state.treasury, treasury_pda);
    assert_eq!(protocol_state.default_protocol_fee_rate, 2000);
    assert_eq!(protocol_state.max_pool_fee_rate, 10000);
    assert!(!protocol_state.paused);
    assert!(protocol_state.pool_creation_allowed);
    assert!(protocol_state.initialized_at > 0);
    assert!(protocol_state.last_updated > 0);

    // Treasury state assertions
    assert_eq!(treasury.protocol, protocol_pda);
    assert_eq!(treasury.authority, payer_pubkey);
    assert_eq!(treasury.total_collected, 0);
    assert_eq!(treasury.total_withdrawn, 0);
    assert_eq!(treasury.last_withdrawal, 0);
}

#[tokio::test]
async fn test_initialize_protocol_fee_too_high() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, _, _) = InstructionBuilder::initialize(&payer_pubkey, 6000, 10000);

    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;
    assert!(result.is_err());
    let anchor_error_code: u32 = ProtocolError::ProtocolFeeTooHigh.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_initialize_max_pool_fee_too_high() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, _, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 15000);

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

#[tokio::test]
async fn test_initialize_edge_case_values() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, protocol_pda, _) =
        InstructionBuilder::initialize(&payer_pubkey, MAX_PROTOCOL_FEE_RATE, MAX_POOL_FEE_RATE);

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    let protocol_state: ProtocolState = app.get_account_data(protocol_pda).await.unwrap();
    assert_eq!(protocol_state.default_protocol_fee_rate, 5000);
    assert_eq!(protocol_state.max_pool_fee_rate, 10000);
}

#[tokio::test]
async fn test_initialize_zero_fees() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, protocol_pda, _) = InstructionBuilder::initialize(&payer_pubkey, 0, 0);

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    let protocol_state: ProtocolState = app.get_account_data(protocol_pda).await.unwrap();
    assert_eq!(protocol_state.default_protocol_fee_rate, 0);
    assert_eq!(protocol_state.max_pool_fee_rate, 0);
}
