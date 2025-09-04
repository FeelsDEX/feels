use crate::{state::FeelsSolController, tests::InstructionBuilder};
use anchor_lang::{prelude::*, system_program, InstructionData};
use anchor_spl::token_interface::Mint;
use feels_test_utils::{
    constants::{
        FEELSSOL_PROGRAM_PATH, FEELS_PRIVATE_KEY, JITOSOL_MINT, KEEPER_PDA_SEED,
        KEEPER_PROGRAM_PATH,
    },
    to_sdk_instruction, TestApp,
};
use solana_sdk::signer::{SeedDerivable, Signer};

#[tokio::test]
async fn test_initialize_feelssol_controller_success() {
    let mut app = TestApp::new_with_programs(vec![
        (crate::id(), FEELSSOL_PROGRAM_PATH),
        (feels_keeper::id(), KEEPER_PROGRAM_PATH),
    ])
    .await;
    let payer_pubkey = app.payer_pubkey();
    let token_mint = solana_sdk::signature::Keypair::from_seed(&FEELS_PRIVATE_KEY).unwrap();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    assert!(token_mint_pubkey.to_string().starts_with("FeeLs"));

    // Initialize the keeper first
    let (keeper_pda, _) = Pubkey::find_program_address(&[KEEPER_PDA_SEED], &feels_keeper::id());

    let accounts = feels_keeper::accounts::Initialize {
        keeper: keeper_pda,
        authority: payer_pubkey,
        payer: payer_pubkey,
        system_program: system_program::ID,
    };

    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: feels_keeper::id(),
        accounts: accounts.to_account_metas(None),
        data: feels_keeper::instruction::Initialize {
            feelssol_to_lst_rate_numerator: 1,
            feelssol_to_lst_rate_denominator: 1,
        }
        .data(),
    };

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    let (instruction, feelssol_pda) = InstructionBuilder::initialize(
        &payer_pubkey,
        token_mint_pubkey,
        JITOSOL_MINT.parse().unwrap(),
        keeper_pda,
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
    assert_eq!(feels_sol.keeper, keeper_pda);
    assert_eq!(feels_sol.feels_mint, token_mint_pubkey);
    assert_eq!(feels_sol.total_wrapped, 0);
    assert_eq!(feels_sol.feels_protocol, feels_protocol::ID);

    // Mint state assertions
    assert_eq!(feels_mint.decimals, 9);
    assert_eq!(feels_mint.supply, 0);
}
