//! Example: Initialize tranche ticks (crank) and graduate + cleanup
//!
//! This example shows how to:
//! 1) Derive the set of TickArray PDAs needed for N stair steps
//! 2) Submit the initialize_tranche_ticks crank
//! 3) Graduate the pool and cleanup bonding-curve plan
//!
//! Note: This is a sketch that relies on an Anchor client setup. In a real
//! script, wire your RPC URL and payer keypair via SdkConfig/FeelsClient.

use feels_sdk::{instructions, utils::derive_tranche_tick_arrays, FeelsClient, SdkConfig};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

fn main() -> anyhow::Result<()> {
    // Configure client (adjust to your environment)
    let payer = Keypair::new();
    let cfg = SdkConfig::localnet(payer);
    let client = FeelsClient::new(cfg)?;

    // Inputs
    let market: Pubkey = Pubkey::new_unique();
    let tick_step_size: i32 = 1000; // match deploy parameter
    let num_steps: u8 = 10;

    // Fetch market state to derive arrays
    let market_state: feels::state::Market = client.program.account(market)?;
    let current_tick = market_state.current_tick;
    let tick_spacing = market_state.tick_spacing;

    // Derive TickArray PDAs for tranche boundaries
    let tick_arrays = derive_tranche_tick_arrays(
        &market,
        current_tick,
        tick_spacing,
        tick_step_size,
        num_steps,
    );

    // Build crank instruction
    let crank = client.config.payer.as_ref().pubkey();
    let ix_crank = instructions::initialize_tranche_ticks(
        crank,
        market,
        tick_step_size,
        num_steps,
        tick_arrays,
    );
    client.program.request().instruction(ix_crank).send()?;

    // Graduate pool (requires market authority = payer in this sketch)
    let ix_grad = instructions::graduate_pool(crank, market);
    client.program.request().instruction(ix_grad).send()?;

    // Cleanup bonding curve (closes tranche plan)
    let ix_cleanup = instructions::cleanup_bonding_curve(crank, market);
    client.program.request().instruction(ix_cleanup).send()?;

    println!("Crank, graduate, and cleanup submitted.");
    Ok(())
}
