use crate::{
    state::FeelsSolController,
    tests::{InstructionBuilder, PROGRAM_PATH},
};
use anchor_lang::prelude::Pubkey;
use anchor_spl::token_interface::Mint;
use feels_test_utils::{to_sdk_instruction, TestApp};
use solana_sdk::signer::Signer;

const JITOSOL_MINT: &str = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";
// Example secret key that gives a Pubkey that starts with `fee1`.
const FEELS_PRIVATE_KEY: [u8; 32] = [
    173, 242, 88, 79, 86, 15, 167, 224, 211, 34, 196, 239, 85, 249, 117, 35, 95, 153, 145, 178,
    209, 92, 54, 144, 124, 118, 221, 73, 100, 141, 228, 193,
];

#[tokio::test]
async fn test_initialize_feelssol_controller_success() {
    let mut app = TestApp::new_with_program(crate::id(), PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();
    let secret_key: [u8; 32] = FEELS_PRIVATE_KEY[0..32].try_into().unwrap();
    let token_mint = solana_sdk::signer::keypair::Keypair::new_from_array(secret_key);
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    assert_eq!(
        token_mint_pubkey.to_string()[0..4].to_string(),
        "fee1".to_string()
    );

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
