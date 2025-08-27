use crate::{
    state::FeelsSOLWrapper,
    tests::{InstructionBuilder, PROGRAM_PATH},
};
use anchor_spl::token_interface::Mint;
use feels_test_utils::{to_sdk_instruction, TestApp};

const JITOSOL_MINT: &str = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";

#[tokio::test]
async fn test_initialize_protocol_success() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();

    let (instruction, feelssol_pda, feels_mint_pda) = InstructionBuilder::initialize(
        &payer_pubkey,
        JITOSOL_MINT.parse().unwrap(),
        feels_protocol::ID,
    );

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    let feels_sol: FeelsSOLWrapper = app.get_account_data(feelssol_pda).await.unwrap();
    let feels_mint: Mint = app.get_account_data(feels_mint_pda).await.unwrap();

    // State assertions
    assert_eq!(feels_sol.underlying_mint, JITOSOL_MINT.parse().unwrap());
    assert_eq!(feels_sol.total_wrapped, 0);
    assert_eq!(feels_sol.virtual_reserves, 0);
    assert_eq!(feels_sol.yield_accumulator, 0);
    assert_eq!(feels_sol.last_update_slot, 0);
    assert_eq!(feels_sol.feels_protocol, feels_protocol::ID);

    // Mint state assertions
    assert_eq!(feels_mint.decimals, 9);
    assert_eq!(feels_mint.supply, 0);
}
