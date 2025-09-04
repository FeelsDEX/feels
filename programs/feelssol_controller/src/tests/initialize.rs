use crate::{state::FeelsSolController, tests::InstructionBuilder};
use anchor_lang::prelude::Pubkey;
use anchor_spl::token_interface::Mint;
use feels_test_utils::{
    constants::{FEELSSOL_PROGRAM_PATH, FEELS_PRIVATE_KEY, JITOSOL_MINT, JITO_STAKE_POOL},
    to_sdk_instruction, TestApp,
};
use solana_sdk::signer::{SeedDerivable, Signer};

#[tokio::test]
async fn test_initialize_feelssol_controller_success() {
    let mut app = TestApp::new_with_program(crate::id(), FEELSSOL_PROGRAM_PATH).await;
    let payer_pubkey = app.payer_pubkey();
    let token_mint = solana_sdk::signature::Keypair::from_seed(&FEELS_PRIVATE_KEY).unwrap();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    assert!(token_mint_pubkey.to_string().starts_with("FeeLs"));

    let (instruction, feelssol_pda) = InstructionBuilder::initialize(
        &payer_pubkey,
        token_mint_pubkey,
        JITOSOL_MINT.parse().unwrap(),
        JITO_STAKE_POOL.parse().unwrap(),
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
    assert_eq!(
        feels_sol.underlying_stake_pool,
        JITO_STAKE_POOL.parse().unwrap()
    );
    assert_eq!(feels_sol.feels_mint, token_mint_pubkey);
    assert_eq!(feels_sol.total_wrapped, 0);
    assert_eq!(feels_sol.feels_protocol, feels_protocol::ID);

    // Mint state assertions
    assert_eq!(feels_mint.decimals, 9);
    assert_eq!(feels_mint.supply, 0);
}
