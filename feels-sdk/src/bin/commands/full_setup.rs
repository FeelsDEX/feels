// Complete end-to-end protocol setup

use anyhow::{Context, Result};
use clap::Args;
use feels_sdk::{
    instructions::{InitializeHubParams, InitializeProtocolParams, ProtocolInstructionBuilder},
};
use solana_sdk::signature::Signer;
use std::fs;

use super::{
    utils::{get_program_id, info, load_keypair, parse_pubkey, success, warn},
    RpcHelper,
};

#[derive(Args)]
pub struct FullSetupCmd {
    /// JitoSOL mint address
    #[arg(long)]
    jitosol_mint: String,

    /// FeelsSOL mint address  
    #[arg(long)]
    feelssol_mint: String,

    /// Treasury address (defaults to wallet)
    #[arg(long)]
    treasury: Option<String>,

    /// Base fee in basis points
    #[arg(long, default_value = "30")]
    base_fee_bps: u16,

    /// Output configuration file path
    #[arg(long)]
    config_output: Option<String>,
}

pub async fn execute(
    cmd: FullSetupCmd,
    rpc_url: &str,
    wallet_path: &str,
    program_id_str: Option<&str>,
) -> Result<()> {
    info("Starting complete protocol setup...");
    info("This will initialize protocol, hub, and create test markets");

    let wallet = load_keypair(wallet_path)?;
    let program_id = get_program_id(program_id_str)?;
    let jitosol_mint = parse_pubkey(&cmd.jitosol_mint)?;
    let feelssol_mint = parse_pubkey(&cmd.feelssol_mint)?;
    let treasury = match cmd.treasury {
        Some(ref addr) => parse_pubkey(addr)?,
        None => wallet.pubkey(),
    };

    // Create instruction builder and RPC helper
    let builder = ProtocolInstructionBuilder::new(program_id);
    let rpc = RpcHelper::new(rpc_url);

    // Step 1: Initialize protocol
    info("\n[1/2] Initializing protocol...");
    let protocol_params = InitializeProtocolParams {
        mint_fee: 1_000_000, // 1 FeelsSOL
        treasury,
        default_protocol_fee_rate: Some(100), // 1%
        default_creator_fee_rate: Some(50),   // 0.5%
        max_protocol_fee_rate: Some(1000),    // 10%
        dex_twap_updater: wallet.pubkey(),
        depeg_threshold_bps: 100,
        depeg_required_obs: 3,
        clear_required_obs: 3,
        dex_twap_window_secs: 300,
        dex_twap_stale_age_secs: 600,
        dex_whitelist: vec![],
    };

    let protocol_ix = builder
        .initialize_protocol(wallet.pubkey(), protocol_params)
        .context("Failed to build protocol instruction")?;

    match rpc.build_and_send_transaction(vec![protocol_ix], &wallet, &[]) {
        Ok(sig) => success(&format!("Protocol initialized: {}", sig)),
        Err(e) => {
            if e.to_string().contains("already in use") {
                warn("Protocol already initialized, skipping...");
            } else {
                return Err(e.into());
            }
        }
    }

    // Step 2: Initialize hub
    info("\n[2/2] Initializing FeelsSOL hub...");
    let hub_params = InitializeHubParams { jitosol_mint };
    let hub_ix = builder
        .initialize_hub(wallet.pubkey(), hub_params)
        .context("Failed to build hub instruction")?;

    match rpc.build_and_send_transaction(vec![hub_ix], &wallet, &[]) {
        Ok(sig) => success(&format!("Hub initialized: {}", sig)),
        Err(e) => {
            if e.to_string().contains("already in use") {
                warn("Hub already initialized, skipping...");
            } else {
                return Err(e.into());
            }
        }
    }

    success("\nProtocol setup complete!");
    info(&format!("Program ID: {}", program_id));
    info(&format!("JitoSOL: {}", jitosol_mint));
    info(&format!("FeelsSOL: {}", feelssol_mint));
    info(&format!("Treasury: {}", treasury));

    // Save configuration if requested
    if let Some(output_path) = cmd.config_output {
        let config = serde_json::json!({
            "programId": program_id.to_string(),
            "jitosolMint": jitosol_mint.to_string(),
            "feelssolMint": feelssol_mint.to_string(),
            "treasury": treasury.to_string(),
            "baseFee": cmd.base_fee_bps,
        });

        fs::write(&output_path, serde_json::to_string_pretty(&config)?)
            .with_context(|| format!("Failed to write config to {}", output_path))?;

        success(&format!("Configuration saved to: {}", output_path));
    }

    Ok(())
}
