// Complete end-to-end protocol setup

use anyhow::{Context, Result};
use clap::Args;
use feels_sdk::{instructions::InitializeProtocolParams, FeelsClient};
use solana_sdk::signature::Signer;
use std::fs;

use super::utils::{get_program_id, info, load_keypair, parse_pubkey, success, warn};

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

    // Create client
    let client = if let Some(_pid_str) = program_id_str {
        FeelsClient::with_program_id(rpc_url, program_id).await?
    } else {
        FeelsClient::new(rpc_url).await?
    };

    // Step 1: Initialize protocol
    info("\n[1/2] Initializing protocol...");
    let protocol_params = InitializeProtocolParams {
        base_fee_bps: cmd.base_fee_bps,
        max_fee_bps: 300,
        fee_growth_rate: 100,
        protocol_fee_share_bps: 20,
        treasury,
        oracle_authority: wallet.pubkey(),
    };

    let protocol_ix = client
        .protocol
        .initialize_protocol_ix(wallet.pubkey(), protocol_params)?;

    match client
        .base
        .send_transaction(&[protocol_ix], &[&wallet])
        .await
    {
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
    let hub_ix = client
        .protocol
        .initialize_hub_ix(wallet.pubkey(), jitosol_mint)?;

    match client.base.send_transaction(&[hub_ix], &[&wallet]).await {
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
