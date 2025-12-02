// Protocol configuration commands

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use feels_sdk::{
    instructions::{InitializeProtocolParams, ProtocolInstructionBuilder},
    protocol::PdaBuilder,
};
use solana_sdk::signature::Signer;

use super::{
    utils::{get_program_id, info, load_keypair, parse_pubkey, success},
    RpcHelper,
};

#[derive(Args)]
pub struct ProtocolCmd {
    #[command(subcommand)]
    command: ProtocolSubcommand,
}

#[derive(Subcommand)]
enum ProtocolSubcommand {
    /// Initialize protocol configuration (one-time setup)
    Init {
        /// Treasury address (defaults to wallet)
        #[arg(long)]
        treasury: Option<String>,

        /// DEX TWAP updater authority (defaults to wallet)
        #[arg(long)]
        dex_twap_updater: Option<String>,

        /// Mint fee in FeelsSOL lamports
        #[arg(long, default_value = "1000000")]
        mint_fee: u64,

        /// De-peg threshold in basis points
        #[arg(long, default_value = "100")]
        depeg_threshold_bps: u16,

        /// Consecutive breaches required to pause
        #[arg(long, default_value = "3")]
        depeg_required_obs: u8,

        /// Consecutive clears required to resume
        #[arg(long, default_value = "3")]
        clear_required_obs: u8,

        /// DEX TWAP window in seconds
        #[arg(long, default_value = "300")]
        dex_twap_window_secs: u32,

        /// DEX TWAP stale age in seconds
        #[arg(long, default_value = "600")]
        dex_twap_stale_age_secs: u32,
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
            treasury,
            dex_twap_updater,
            mint_fee,
            depeg_threshold_bps,
            depeg_required_obs,
            clear_required_obs,
            dex_twap_window_secs,
            dex_twap_stale_age_secs,
        } => {
            info("Initializing Feels Protocol...");

            let wallet = load_keypair(wallet_path)?;
            let program_id = get_program_id(program_id_str)?;
            let treasury = match treasury {
                Some(addr) => parse_pubkey(&addr)?,
                None => wallet.pubkey(),
            };
            let dex_twap_updater = match dex_twap_updater {
                Some(addr) => parse_pubkey(&addr)?,
                None => wallet.pubkey(),
            };

            // Create instruction builder
            let builder = ProtocolInstructionBuilder::new(program_id);
            let rpc = RpcHelper::new(rpc_url);

            info(&format!("Using program ID: {}", program_id));

            // Build initialize instruction
            let params = InitializeProtocolParams {
                mint_fee,
                treasury,
                default_protocol_fee_rate: Some(100), // 1%
                default_creator_fee_rate: Some(50),   // 0.5%
                max_protocol_fee_rate: Some(1000),    // 10%
                dex_twap_updater,
                depeg_threshold_bps,
                depeg_required_obs,
                clear_required_obs,
                dex_twap_window_secs,
                dex_twap_stale_age_secs,
                dex_whitelist: vec![],
            };

            let ix = builder
                .initialize_protocol(wallet.pubkey(), params)
                .context("Failed to build instruction")?;
            let signature = rpc
                .build_and_send_transaction(vec![ix], &wallet, &[])
                .context("Failed to send transaction")?;

            success(&format!("Protocol initialized! Signature: {}", signature));
            info(&format!("Treasury: {}", treasury));
            info(&format!("DEX TWAP Updater: {}", dex_twap_updater));

            Ok(())
        }

        ProtocolSubcommand::Info => {
            info("Fetching protocol configuration...");

            let program_id = get_program_id(program_id_str)?;
            let pda = PdaBuilder::new(program_id);
            let (protocol_config, _) = pda.protocol_config();

            info(&format!("Protocol Config PDA: {}", protocol_config));
            info(&format!("Program ID: {}", program_id));
            info("Use solana account command to view details");

            Ok(())
        }
    }
}
