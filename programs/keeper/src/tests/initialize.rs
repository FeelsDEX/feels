use feels_test_utils::{constants::KEEPER_PROGRAM_PATH, to_sdk_instruction, TestApp};

use crate::{state::Keeper, tests::InstructionBuilder};

#[tokio::test]
async fn test_initialize_feelssol_controller_success() {
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
