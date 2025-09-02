use crate::{state::TokenFactory, tests::{InstructionBuilder, PROGRAM_PATH}};

use feels_test_utils::{to_sdk_instruction, TestApp};

#[tokio::test]
async fn test_initialize_feels_token_factory_success() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, factory_pda) =
        InstructionBuilder::initialize(&payer_pubkey, feels_protocol::ID);

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    let factory: TokenFactory = app.get_account_data(factory_pda).await.unwrap();

    // State assertions
    assert_eq!(factory.feels_protocol, feels_protocol::ID);
    assert_eq!(factory.tokens_created, 0);
}
