// Market management commands

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use feels_sdk::FeelsClient;

use super::utils::{get_program_id, info, load_keypair, parse_pubkey, success};

#[derive(Args)]
pub struct MarketCmd {
    #[command(subcommand)]
    command: MarketSubcommand,
}

#[derive(Subcommand)]
enum MarketSubcommand {
    /// Create a new market
    Create {
        /// FeelsSOL mint address (must be token_0)
        #[arg(long)]
        feelssol_mint: String,

        /// Other token mint address (token_1)
        #[arg(long)]
        token_mint: String,

        /// Base fee in basis points
        #[arg(long, default_value = "30")]
        base_fee_bps: u16,

        /// Tick spacing
        #[arg(long, default_value = "64")]
        tick_spacing: u16,

        /// Initial sqrt price (Q64.64, default is 1:1)
        #[arg(long, default_value = "79228162514264337593543950336")]
        initial_sqrt_price: u128,

        /// Initial buy amount in FeelsSOL lamports
        #[arg(long, default_value = "0")]
        initial_buy_amount: u64,
    },

    /// Get market information
    Info {
        /// Market address or derive from tokens
        #[arg(long, group = "market_id")]
        market: Option<String>,

        /// Token 0 address (for derivation)
        #[arg(long, requires = "token1", group = "market_id")]
        token0: Option<String>,

        /// Token 1 address (for derivation)
        #[arg(long, requires = "token0", group = "market_id")]
        token1: Option<String>,
    },
}

pub async fn execute(
    cmd: MarketCmd,
    rpc_url: &str,
    wallet_path: &str,
    program_id_str: Option<&str>,
) -> Result<()> {
    match cmd.command {
        MarketSubcommand::Create {
            feelssol_mint,
            token_mint,
            base_fee_bps,
            tick_spacing,
            initial_sqrt_price,
            initial_buy_amount,
        } => {
            info("Creating market...");

            let wallet = load_keypair(wallet_path)?;
            let program_id = get_program_id(program_id_str)?;
            let feelssol_mint = parse_pubkey(&feelssol_mint)?;
            let token_mint = parse_pubkey(&token_mint)?;

            // Create client
            let client = if let Some(_pid_str) = program_id_str {
                FeelsClient::with_program_id(rpc_url, program_id).await?
            } else {
                FeelsClient::new(rpc_url).await?
            };

            // Initialize market
            let result = client
                .liquidity
                .initialize_market(
                    &wallet,
                    feelssol_mint,
                    token_mint,
                    base_fee_bps,
                    tick_spacing,
                    initial_sqrt_price,
                    initial_buy_amount,
                )
                .await
                .context("Failed to initialize market")?;

            success(&format!("Market created! Address: {}", result.market));
            info(&format!("Signature: {}", result.signature));
            info(&format!("Token 0 (FeelsSOL): {}", feelssol_mint));
            info(&format!("Token 1: {}", token_mint));

            Ok(())
        }

        MarketSubcommand::Info {
            market,
            token0,
            token1,
        } => {
            info("Fetching market information...");

            let client = if let Some(pid_str) = program_id_str {
                let program_id = get_program_id(Some(pid_str))?;
                FeelsClient::with_program_id(rpc_url, program_id).await?
            } else {
                FeelsClient::new(rpc_url).await?
            };

            let market_addr = if let Some(addr) = market {
                parse_pubkey(&addr)?
            } else if let (Some(t0), Some(t1)) = (token0, token1) {
                let token0 = parse_pubkey(&t0)?;
                let token1 = parse_pubkey(&t1)?;
                client
                    .market
                    .get_market_by_tokens(&token0, &token1)
                    .await?
                    .address
            } else {
                anyhow::bail!("Must provide either --market or both --token0 and --token1");
            };

            info(&format!("Market address: {}", market_addr));
            info("Use solana account command or SDK to view market details");

            Ok(())
        }
    }
}
