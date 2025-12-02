// FeelsSOL hub commands

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use feels_sdk::{
    instructions::{InitializeHubParams, ProtocolInstructionBuilder},
    protocol::PdaBuilder,
};
use solana_sdk::signature::Signer;

use super::{
    utils::{get_program_id, info, load_keypair, parse_pubkey, success},
    RpcHelper,
};

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

            // Create instruction builder
            let builder = ProtocolInstructionBuilder::new(program_id);

            // Build initialize hub instruction
            let params = InitializeHubParams { jitosol_mint };
            let ix = builder
                .initialize_hub(wallet.pubkey(), params)
                .context("Failed to build instruction")?;

            // Send transaction
            let rpc = RpcHelper::new(rpc_url);
            let signature = rpc
                .build_and_send_transaction(vec![ix], &wallet, &[])
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

            let program_id = get_program_id(program_id_str)?;
            let pda = PdaBuilder::new(program_id);
            let (hub_pda, _) = pda.feels_hub();

            info(&format!("FeelsSOL Hub PDA: {}", hub_pda));
            info(&format!("Program ID: {}", program_id));
            info("Use solana account command to view details");

            Ok(())
        }
    }
}
