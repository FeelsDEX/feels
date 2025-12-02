// Market management commands

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use feels_sdk::{
    instructions::MarketInstructionBuilder,
    protocol::PdaBuilder,
};

use super::{
    utils::{get_program_id, info, load_keypair, parse_pubkey, success},
    RpcHelper,
};

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
            base_fee_bps: _,
            tick_spacing: _,
            initial_sqrt_price: _,
            initial_buy_amount: _,
        } => {
            info("Creating market...");

            let _wallet = load_keypair(wallet_path)?;
            let program_id = get_program_id(program_id_str)?;
            let feelssol_mint_pk = parse_pubkey(&feelssol_mint)?;
            let token_mint_pk = parse_pubkey(&token_mint)?;

            // Note: Market initialization requires liquidity provision which is complex
            // For CLI simplicity, we'll just show the market address that would be created
            let pda = PdaBuilder::new(program_id);
            let (market_address, _) = pda.market(&feelssol_mint_pk, &token_mint_pk);

            info(&format!("Market would be created at: {}", market_address));
            info(&format!("FeelsSOL: {}", feelssol_mint_pk));
            info(&format!("Token: {}", token_mint_pk));
            info("Note: Full market initialization requires liquidity provision");
            info("Use the protocol GUI or advanced SDK for complete setup");

            Ok(())
        }

        MarketSubcommand::Info {
            market,
            token0,
            token1,
        } => {
            info("Fetching market information...");

            let program_id = get_program_id(program_id_str)?;
            let pda = PdaBuilder::new(program_id);

            let market_addr = if let Some(addr) = market {
                parse_pubkey(&addr)?
            } else if let (Some(t0), Some(t1)) = (token0, token1) {
                let token0_pk = parse_pubkey(&t0)?;
                let token1_pk = parse_pubkey(&t1)?;
                let (market_pda, _) = pda.market(&token0_pk, &token1_pk);
                market_pda
            } else {
                anyhow::bail!("Must provide either --market or both --token0 and --token1");
            };

            info(&format!("Market address: {}", market_addr));
            info(&format!("Program ID: {}", program_id));
            info("Use solana account command or SDK to view market details");

            Ok(())
        }
    }
}
