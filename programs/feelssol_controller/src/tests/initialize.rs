use crate::{
    state::FeelsSolController,
    tests::{InstructionBuilder, PROGRAM_PATH},
};
use anchor_lang::prelude::Pubkey;
use anchor_spl::token_interface::Mint;
use feels_test_utils::{to_sdk_instruction, TestApp};
use solana_sdk::signer::{SeedDerivable, Signer};

const JITOSOL_MINT: &str = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";
// Example secret key that gives a Pubkey that starts with `Fee1s`.
const FEELS_PRIVATE_KEY: [u8; 32] = [
    175, 231, 9, 4, 171, 54, 207, 154, 207, 24, 149, 237, 50, 226, 208, 61, 57, 155, 22, 83, 47,
    86, 18, 123, 18, 154, 127, 87, 224, 112, 101, 180,
];

#[tokio::test]
async fn test_initialize_feelssol_controller_success() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();
    let token_mint = solana_sdk::signature::Keypair::from_seed(&FEELS_PRIVATE_KEY).unwrap();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    assert!(token_mint_pubkey.to_string().starts_with("Fee1s"));

    let (instruction, feelssol_pda) = InstructionBuilder::initialize(
        &payer_pubkey,
        token_mint_pubkey,
        JITOSOL_MINT.parse().unwrap(),
        feels_protocol::ID,
    );

    app.process_instruction_with_multiple_signers(
        to_sdk_instruction(instruction),
        &app.context.payer.insecure_clone(),
        &[&token_mint],
    )
    .await
    .unwrap();

    let feels_sol: FeelsSolController = app.get_account_data(feelssol_pda).await.unwrap();
    let feels_mint: Mint = app.get_account_data(token_mint_pubkey).await.unwrap();

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
