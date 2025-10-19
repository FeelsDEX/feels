// FeelsSOL hub commands

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use feels_sdk::FeelsClient;
use solana_sdk::signature::Signer;

use super::utils::{get_program_id, info, load_keypair, parse_pubkey, success};

#[derive(Args)]
pub struct HubCmd {
    #[command(subcommand)]
    command: HubSubcommand,
}

#[derive(Subcommand)]
enum HubSubcommand {
    /// Initialize FeelsSOL hub
    Init {
        /// JitoSOL mint address
        #[arg(long)]
        jitosol_mint: String,
    },

    /// Get hub information
    Info,
}

pub async fn execute(
    cmd: HubCmd,
    rpc_url: &str,
    wallet_path: &str,
    program_id_str: Option<&str>,
) -> Result<()> {
    match cmd.command {
        HubSubcommand::Init { jitosol_mint } => {
            info("Initializing FeelsSOL hub...");

            let wallet = load_keypair(wallet_path)?;
            let program_id = get_program_id(program_id_str)?;
            let jitosol_mint = parse_pubkey(&jitosol_mint)?;

            // Create client
            let client = if let Some(_pid_str) = program_id_str {
                FeelsClient::with_program_id(rpc_url, program_id).await?
            } else {
                FeelsClient::new(rpc_url).await?
            };

            // Build initialize hub instruction
            let ix = client
                .protocol
                .initialize_hub_ix(wallet.pubkey(), jitosol_mint)?;

            // Send transaction
            let signature = client
                .base
                .send_transaction(&[ix], &[&wallet])
                .await
                .context("Failed to send transaction")?;

            success(&format!(
                "FeelsSOL hub initialized! Signature: {}",
                signature
            ));
            info(&format!("JitoSOL mint: {}", jitosol_mint));

            Ok(())
        }

        HubSubcommand::Info => {
            info("Fetching hub information...");

            let client = if let Some(pid_str) = program_id_str {
                let program_id = get_program_id(Some(pid_str))?;
                FeelsClient::with_program_id(rpc_url, program_id).await?
            } else {
                FeelsClient::new(rpc_url).await?
            };

            let (hub_pda, _) = client.pda.feels_hub();

            info(&format!("FeelsSOL Hub PDA: {}", hub_pda));
            info("Use solana account command to view details");

            Ok(())
        }
    }
}
