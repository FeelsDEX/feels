use anchor_lang::prelude::*;
use feels_test_utils::{constants::AMM_PROGRAM_PATH, to_sdk_instruction, TestApp};

use crate::{state::pool::Pool, tests::InstructionBuilder};

#[tokio::test]
async fn test_create_pool() {
    let mut app = TestApp::new_with_programs(vec![(crate::id(), AMM_PROGRAM_PATH)]).await;

    let token_a = app.create_mint(None, None, 6).await.unwrap();
    let token_b = app.create_mint(None, None, 6).await.unwrap();
    let payer_pubkey = app.payer_pubkey();

    let (instruction, pool_pda) = InstructionBuilder::create_pool(
        &payer_pubkey,
        &token_a,
        &token_b,
        1000,        // fee_bps
        500,         // protocol_fee_bps
        10,          // tick_spacing
        1u128 << 64, // initial_sqrt_price (1.0 in Q64.64 format)
    );

    app.process_instruction(to_sdk_instruction(instruction))
        .await
        .unwrap();

    let pool_account = app
        .get_account(pool_pda)
        .await
        .expect("Failed to fetch pool account");

    let pool_data = Pool::try_deserialize(&mut pool_account.data.as_ref())
        .expect("Failed to deserialize pool account data");
    assert_eq!(pool_data.fee_bps, 1000);
    assert_eq!(pool_data.protocol_fee_bps, 500);
    assert_eq!(pool_data.tick_spacing, 10);
    assert_eq!(pool_data.sqrt_price, 1u128 << 64);
}
