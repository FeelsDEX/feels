// Protocol configuration commands

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use feels_sdk::{instructions::InitializeProtocolParams, FeelsClient};
use solana_sdk::signature::Signer;

use super::utils::{get_program_id, info, load_keypair, parse_pubkey, success};

#[derive(Args)]
pub struct ProtocolCmd {
    #[command(subcommand)]
    command: ProtocolSubcommand,
}

#[derive(Subcommand)]
enum ProtocolSubcommand {
    /// Initialize protocol configuration (one-time setup)
    Init {
        /// Base fee in basis points (e.g., 30 = 0.3%)
        #[arg(long, default_value = "30")]
        base_fee_bps: u16,

        /// Maximum fee in basis points
        #[arg(long, default_value = "300")]
        max_fee_bps: u16,

        /// Fee growth rate
        #[arg(long, default_value = "100")]
        fee_growth_rate: u64,

        /// Protocol fee share in basis points
        #[arg(long, default_value = "20")]
        protocol_fee_share_bps: u16,

        /// Treasury address
        #[arg(long)]
        treasury: String,

        /// Oracle authority address (defaults to wallet)
        #[arg(long)]
        oracle_authority: Option<String>,
    },

    /// Get protocol configuration
    Info,
}

pub async fn execute(
    cmd: ProtocolCmd,
    rpc_url: &str,
    wallet_path: &str,
    program_id_str: Option<&str>,
) -> Result<()> {
    match cmd.command {
        ProtocolSubcommand::Init {
            base_fee_bps,
            max_fee_bps,
            fee_growth_rate,
            protocol_fee_share_bps,
            treasury,
            oracle_authority,
        } => {
            info("Initializing Feels Protocol...");

            let wallet = load_keypair(wallet_path)?;
            let program_id = get_program_id(program_id_str)?;
            let treasury = parse_pubkey(&treasury)?;
            let oracle_authority = match oracle_authority {
                Some(addr) => parse_pubkey(&addr)?,
                None => wallet.pubkey(),
            };

            // Create client
            let client = if let Some(_pid_str) = program_id_str {
                FeelsClient::with_program_id(rpc_url, program_id).await?
            } else {
                FeelsClient::new(rpc_url).await?
            };

            // Build initialize instruction
            let params = InitializeProtocolParams {
                base_fee_bps,
                max_fee_bps,
                fee_growth_rate,
                protocol_fee_share_bps,
                treasury,
                oracle_authority,
            };

            let ix = client
                .protocol
                .initialize_protocol_ix(wallet.pubkey(), params)?;

            // Send transaction
            let signature = client
                .base
                .send_transaction(&[ix], &[&wallet])
                .await
                .context("Failed to send transaction")?;

            success(&format!("Protocol initialized! Signature: {}", signature));
            info(&format!("Program ID: {}", program_id));
            info(&format!("Treasury: {}", treasury));
            info(&format!("Oracle Authority: {}", oracle_authority));

            Ok(())
        }

        ProtocolSubcommand::Info => {
            info("Fetching protocol configuration...");

            let client = if let Some(pid_str) = program_id_str {
                let program_id = get_program_id(Some(pid_str))?;
                FeelsClient::with_program_id(rpc_url, program_id).await?
            } else {
                FeelsClient::new(rpc_url).await?
            };

            let (protocol_config, _) = client.pda.protocol_config();

            info(&format!("Protocol Config PDA: {}", protocol_config));
            info("Use solana account command to view details");

            Ok(())
        }
    }
}
