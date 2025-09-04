use feels_test_utils::{constants::KEEPER_PROGRAM_PATH, to_sdk_instruction, TestApp};

use crate::{state::Keeper, tests::InstructionBuilder};

#[tokio::test]
async fn test_initialize_keeper_success() {
    let mut app = TestApp::new_with_program(crate::id(), KEEPER_PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, keeper_pda) =
        InstructionBuilder::initialize(&payer_pubkey, &payer_pubkey, 1, 1);

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    let keeper: Keeper = app.get_account_data(keeper_pda).await.unwrap();

    // State assertions
    assert_eq!(keeper.authority, payer_pubkey);
    assert_eq!(keeper.feelssol_to_lst_rate_numerator, 1);
    assert_eq!(keeper.feelssol_to_lst_rate_denominator, 1);
}

#[tokio::test]
async fn test_initialize_keeper_fails_zero_rates() {
    let mut app = TestApp::new_with_program(crate::id(), KEEPER_PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, _) = InstructionBuilder::initialize(&payer_pubkey, &payer_pubkey, 0, 1);

    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;
    assert!(result.is_err());
    let anchor_error_code: u32 = crate::error::KeeperError::ZeroRate.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));

    let (instruction, _) = InstructionBuilder::initialize(&payer_pubkey, &payer_pubkey, 1, 0);

    let result = app
        .process_instruction(to_sdk_instruction(instruction))
        .await;
    assert!(result.is_err());
    let anchor_error_code: u32 = crate::error::KeeperError::ZeroRate.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}
