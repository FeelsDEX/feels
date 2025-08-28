use anchor_lang::{prelude::*, solana_program::program_pack::Pack};
use anchor_spl::token_2022::spl_token_2022::{self, extension::StateWithExtensions};
use feels_test_utils::{to_sdk_instruction, TestApp};
use feels_token_factory::state::{factory::TokenFactory, metadata::TokenMetadata};
use solana_sdk::signature::Signer;

use crate::tests::{InstructionBuilder, FACTORY_PROGRAM_PATH, PROGRAM_PATH};

// Helper to create a TestApp that initializes both the protocol and the factory
async fn deploy_protocol_and_factory() -> (TestApp, Pubkey, Pubkey, Pubkey) {
    let mut app = TestApp::new_with_programs(vec![
        (crate::id(), PROGRAM_PATH),
        (feels_token_factory::id(), FACTORY_PROGRAM_PATH),
    ])
    .await;

    let payer_pubkey = app.payer_pubkey();

    // Initialize the protocol
    let (instruction, protocol_pda, treasury_pda) =
        InstructionBuilder::initialize(&payer_pubkey, 2000, 10000);
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    // Initialize the token factory
    let (instruction, factory_pda) =
        feels_token_factory::instruction_builder::InstructionBuilder::initialize(
            &payer_pubkey,
            crate::id(),
        );
    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    (app, protocol_pda, treasury_pda, factory_pda)
}

#[tokio::test]
async fn test_create_token_via_factory_success() {
    let (mut app, _, _, factory_pda) = deploy_protocol_and_factory().await;

    let payer_pubkey = app.payer_pubkey();
    let recipient = solana_sdk::signer::keypair::Keypair::new();
    let recipient_pubkey = Pubkey::from(recipient.pubkey().to_bytes());
    let token_mint = solana_sdk::signer::keypair::Keypair::new();
    let token_mint_pubkey = Pubkey::from(token_mint.pubkey().to_bytes());

    // Test parameters
    let ticker = "TEST".to_string();
    let name = "Test Token".to_string();
    let symbol = "TST".to_string();
    let decimals = 9u8;
    let initial_supply = 1_000_000u64;

    // Create token instruction - PDAs calculated internally
    let (instruction, recipient_token_account, token_metadata_account) =
        InstructionBuilder::create_token(
            &token_mint_pubkey,
            &recipient_pubkey,
            &payer_pubkey,
            ticker.clone(),
            name.clone(),
            symbol.clone(),
            decimals,
            initial_supply,
        );

    // Process the instruction
    app.process_instruction_with_multiple_signers(
        to_sdk_instruction(instruction),
        &app.context.payer.insecure_clone(),
        &[&token_mint],
    )
    .await
    .unwrap();

    // Verify results using the returned PDAs
    let factory: TokenFactory = app.get_account_data(factory_pda).await.unwrap();
    assert_eq!(factory.tokens_created, 1);

    let metadata: TokenMetadata = app.get_account_data(token_metadata_account).await.unwrap();
    assert_eq!(metadata.ticker, ticker);
    assert_eq!(metadata.name, name);
    assert_eq!(metadata.symbol, symbol);
    assert_eq!(metadata.mint, token_mint_pubkey);

    // Verify token mint and recipient account
    let mint_account = app.get_account(token_mint_pubkey).await.unwrap();
    let mint_data = spl_token_2022::state::Mint::unpack(&mint_account.data).unwrap();
    assert_eq!(mint_data.decimals, decimals);
    assert_eq!(mint_data.supply, initial_supply);

    let recipient_account = app.get_account(recipient_token_account).await.unwrap();
    let token_account_data =
        StateWithExtensions::<spl_token_2022::state::Account>::unpack(&recipient_account.data)
            .unwrap()
            .base;
    assert_eq!(token_account_data.amount, initial_supply);
    assert_eq!(token_account_data.mint, token_mint_pubkey);
    assert_eq!(token_account_data.owner, recipient_pubkey);
}
