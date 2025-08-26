use anchor_lang::{prelude::Pubkey, solana_program::system_instruction::transfer};
use feels_test_utils::{to_sdk_instruction, TestApp};
use solana_sdk::signature::Signer;

use crate::{
    error::ProtocolError,
    instructions::AUTHORITY_TRANSFER_DELAY,
    state::protocol::ProtocolState,
    tests::{instructions::InstructionBuilder, PROGRAM_PATH},
};

#[tokio::test]
async fn test_initiate_authority_transfer_success() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();
    let new_authority = solana_sdk::signer::keypair::Keypair::new();
    let new_authority_pubkey = Pubkey::from(new_authority.pubkey().to_bytes());

    // Initialize protocol first
    let (instruction, protocol_pda, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Initiate authority transfer
    let instruction =
        InstructionBuilder::initiate_authority_transfer(&payer_pubkey, &new_authority_pubkey);

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Verify the transfer was initiated
    let protocol_state: ProtocolState = app.get_account_data(protocol_pda).await.unwrap();
    assert_eq!(protocol_state.pending_authority, Some(new_authority_pubkey));
    assert!(protocol_state.authority_transfer_initiated_at.is_some());
}

#[tokio::test]
async fn test_initiate_authority_transfer_fails_invalid_authority() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();
    let new_authority = solana_sdk::signer::keypair::Keypair::new();
    let fake_authority = solana_sdk::signer::keypair::Keypair::new();
    let fake_authority_pubkey = Pubkey::from(fake_authority.pubkey().to_bytes());

    // Initialize protocol first
    let (instruction, _, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Fund the fake authority
    let fund_instruction = transfer(&payer_pubkey, &fake_authority_pubkey, 1_000_000);
    let mut fund_transaction = solana_sdk::transaction::Transaction::new_with_payer(
        &[to_sdk_instruction(fund_instruction)],
        Some(&app.context.payer.pubkey()),
    );
    fund_transaction.sign(&[&app.context.payer], app.context.last_blockhash);
    app.context
        .banks_client
        .process_transaction(fund_transaction)
        .await
        .unwrap();

    // Try to initiate transfer with fake authority
    let instruction = InstructionBuilder::initiate_authority_transfer(
        &fake_authority_pubkey,
        &Pubkey::from(new_authority.pubkey().to_bytes()),
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
async fn test_initiate_authority_transfer_fails_pending_exists() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();
    let new_authority1 = solana_sdk::signer::keypair::Keypair::new();
    let new_authority2 = solana_sdk::signer::keypair::Keypair::new();
    let new_authority1_pubkey = Pubkey::from(new_authority1.pubkey().to_bytes());
    let new_authority2_pubkey = Pubkey::from(new_authority2.pubkey().to_bytes());

    // Initialize protocol first
    let (instruction, _, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Initiate first transfer
    let instruction =
        InstructionBuilder::initiate_authority_transfer(&payer_pubkey, &new_authority1_pubkey);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Try to initiate second transfer while first is pending
    let instruction =
        InstructionBuilder::initiate_authority_transfer(&payer_pubkey, &new_authority2_pubkey);

    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;
    assert!(result.is_err());
    let anchor_error_code: u32 = ProtocolError::PendingAuthorityTransferExists.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_cancel_authority_transfer_success() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();
    let new_authority = solana_sdk::signer::keypair::Keypair::new();
    let new_authority_pubkey = Pubkey::from(new_authority.pubkey().to_bytes());

    // Initialize protocol first
    let (instruction, protocol_pda, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Initiate authority transfer
    let instruction =
        InstructionBuilder::initiate_authority_transfer(&payer_pubkey, &new_authority_pubkey);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Cancel the transfer
    let instruction = InstructionBuilder::cancel_authority_transfer(&payer_pubkey);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Verify the transfer was cancelled
    let protocol_state: ProtocolState = app.get_account_data(protocol_pda).await.unwrap();
    assert_eq!(protocol_state.pending_authority, None);
    assert_eq!(protocol_state.authority_transfer_initiated_at, None);
}

#[tokio::test]
async fn test_cancel_authority_transfer_fails_no_pending() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    // Initialize protocol first
    let (instruction, _, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Try to cancel without any pending transfer
    let instruction = InstructionBuilder::cancel_authority_transfer(&payer_pubkey);

    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;
    assert!(result.is_err());
    let anchor_error_code: u32 = ProtocolError::NoPendingAuthorityTransfer.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_accept_authority_transfer_success() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();
    let new_authority = solana_sdk::signer::keypair::Keypair::new();
    let new_authority_pubkey = Pubkey::from(new_authority.pubkey().to_bytes());

    // Initialize protocol first
    let (instruction, protocol_pda, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Fund the new authority
    let fund_instruction = transfer(&payer_pubkey, &new_authority_pubkey, 1_000_000);
    let mut fund_transaction = solana_sdk::transaction::Transaction::new_with_payer(
        &[to_sdk_instruction(fund_instruction)],
        Some(&app.context.payer.pubkey()),
    );
    fund_transaction.sign(&[&app.context.payer], app.context.last_blockhash);
    app.context
        .banks_client
        .process_transaction(fund_transaction)
        .await
        .unwrap();

    // Initiate authority transfer
    let instruction =
        InstructionBuilder::initiate_authority_transfer(&payer_pubkey, &new_authority_pubkey);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Fast-forward time by more than the delay period
    app.warp_forward_seconds(AUTHORITY_TRANSFER_DELAY + 1000)
        .await;

    // Accept the transfer
    let instruction = InstructionBuilder::accept_authority_transfer(&new_authority_pubkey);
    app.process_instruction_as_signer(to_sdk_instruction(instruction), &new_authority)
        .await
        .unwrap();

    // Verify the transfer was completed
    let protocol_state: ProtocolState = app.get_account_data(protocol_pda).await.unwrap();
    assert_eq!(protocol_state.authority, new_authority_pubkey);
    assert_eq!(protocol_state.pending_authority, None);
    assert_eq!(protocol_state.authority_transfer_initiated_at, None);
}

#[tokio::test]
async fn test_accept_authority_transfer_fails_delay_not_met() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();
    let new_authority = solana_sdk::signer::keypair::Keypair::new();
    let new_authority_pubkey = Pubkey::from(new_authority.pubkey().to_bytes());

    // Initialize protocol first
    let (instruction, _, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Fund the new authority
    let fund_instruction = transfer(&payer_pubkey, &new_authority_pubkey, 1_000_000);
    let mut fund_transaction = solana_sdk::transaction::Transaction::new_with_payer(
        &[to_sdk_instruction(fund_instruction)],
        Some(&app.context.payer.pubkey()),
    );
    fund_transaction.sign(&[&app.context.payer], app.context.last_blockhash);
    app.context
        .banks_client
        .process_transaction(fund_transaction)
        .await
        .unwrap();

    // Initiate authority transfer
    let instruction =
        InstructionBuilder::initiate_authority_transfer(&payer_pubkey, &new_authority_pubkey);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Try to accept immediately (without waiting for delay)
    let instruction = InstructionBuilder::accept_authority_transfer(&new_authority_pubkey);
    let result = app
        .process_instruction_as_signer(to_sdk_instruction(instruction), &new_authority)
        .await;

    assert!(result.is_err());
    let anchor_error_code: u32 = ProtocolError::AuthorityTransferDelayNotMet.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_accept_authority_transfer_fails_wrong_signer() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();
    let new_authority = solana_sdk::signer::keypair::Keypair::new();
    let wrong_signer = solana_sdk::signer::keypair::Keypair::new();
    let new_authority_pubkey = Pubkey::from(new_authority.pubkey().to_bytes());
    let wrong_signer_pubkey = Pubkey::from(wrong_signer.pubkey().to_bytes());

    // Initialize protocol first
    let (instruction, _, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Fund the wrong signer
    let fund_instruction = transfer(&payer_pubkey, &wrong_signer_pubkey, 1_000_000);
    let mut fund_transaction = solana_sdk::transaction::Transaction::new_with_payer(
        &[to_sdk_instruction(fund_instruction)],
        Some(&app.context.payer.pubkey()),
    );
    fund_transaction.sign(&[&app.context.payer], app.context.last_blockhash);
    app.context
        .banks_client
        .process_transaction(fund_transaction)
        .await
        .unwrap();

    // Initiate authority transfer
    let instruction =
        InstructionBuilder::initiate_authority_transfer(&payer_pubkey, &new_authority_pubkey);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Fast-forward time
    app.warp_forward_seconds(AUTHORITY_TRANSFER_DELAY + 100)
        .await;

    // Try to accept with wrong signer
    let instruction = InstructionBuilder::accept_authority_transfer(&wrong_signer_pubkey);
    let result = app
        .process_instruction_as_signer(to_sdk_instruction(instruction), &wrong_signer)
        .await;

    assert!(result.is_err());
    let anchor_error_code: u32 = ProtocolError::NotPendingAuthority.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[tokio::test]
async fn test_accept_authority_transfer_fails_no_pending() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();
    let new_authority = solana_sdk::signer::keypair::Keypair::new();
    let new_authority_pubkey = Pubkey::from(new_authority.pubkey().to_bytes());

    // Initialize protocol first
    let (instruction, _, _) = InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Fund the new authority
    let fund_instruction = transfer(&payer_pubkey, &new_authority_pubkey, 1_000_000);
    let mut fund_transaction = solana_sdk::transaction::Transaction::new_with_payer(
        &[to_sdk_instruction(fund_instruction)],
        Some(&app.context.payer.pubkey()),
    );
    fund_transaction.sign(&[&app.context.payer], app.context.last_blockhash);
    app.context
        .banks_client
        .process_transaction(fund_transaction)
        .await
        .unwrap();

    // Try to accept without any pending transfer
    let instruction = InstructionBuilder::accept_authority_transfer(&new_authority_pubkey);
    let result = app
        .process_instruction_as_signer(to_sdk_instruction(instruction), &new_authority)
        .await;

    assert!(result.is_err());
    let anchor_error_code: u32 = ProtocolError::NoPendingAuthorityTransfer.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}
