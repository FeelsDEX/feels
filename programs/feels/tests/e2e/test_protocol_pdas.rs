//! E2E: Verify protocol PDAs exist after initialization

use crate::common::*;

test_in_memory!(test_protocol_pdas_exist, |ctx: TestContext| async move {
    // Derive PDAs
    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol_config"], &PROGRAM_ID);
    let (protocol_oracle, _) = Pubkey::find_program_address(&[b"protocol_oracle"], &PROGRAM_ID);
    let (safety_controller, _) = Pubkey::find_program_address(&[b"safety_controller"], &PROGRAM_ID);

    // Fetch accounts (raw) to avoid deserialization constraints
    let cfg = ctx.get_account_raw(&protocol_config).await?;
    let _ = cfg; // ensure used
    let _orc = ctx.get_account_raw(&protocol_oracle).await?;
    let _sfty = ctx.get_account_raw(&safety_controller).await?;

    // Basic sanity
    assert_ne!(protocol_config, Pubkey::default());
    assert_ne!(protocol_oracle, Pubkey::default());
    assert_ne!(safety_controller, Pubkey::default());
    Ok::<(), Box<dyn std::error::Error>>(())
});
