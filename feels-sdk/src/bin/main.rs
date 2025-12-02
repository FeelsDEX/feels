// CLI tool for Feels Protocol
//
// This binary provides a command-line interface to the Feels Protocol,
// enabling both user operations and administrative setup tasks.

mod commands;
mod rpc_helper;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "feels")]
#[command(about = "Feels Protocol CLI", long_about = None)]
#[command(version)]
struct Cli {
    /// RPC URL to connect to
    #[arg(long, default_value = "http://localhost:8899")]
    rpc_url: String,

    /// Path to wallet keypair file
    #[arg(long, default_value = "~/.config/solana/id.json")]
    wallet: String,

    /// Program ID (defaults to declared program ID)
    #[arg(long)]
    program_id: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Protocol initialization and administration (requires admin privileges)
    #[command(subcommand)]
    Init(InitCommands),
}

#[derive(Subcommand)]
enum InitCommands {
    /// Initialize protocol configuration
    Protocol(commands::protocol::ProtocolCmd),

    /// Initialize FeelsSOL hub
    Hub(commands::hub::HubCmd),

    /// Create a new market
    Market(commands::market::MarketCmd),

    /// Complete end-to-end protocol setup
    Full(commands::full_setup::FullSetupCmd),
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute command
    match cli.command {
        Commands::Init(init_cmd) => match init_cmd {
            InitCommands::Protocol(cmd) => {
                commands::protocol::execute(
                    cmd,
                    &cli.rpc_url,
                    &cli.wallet,
                    cli.program_id.as_deref(),
                )
                .await
            }
            InitCommands::Hub(cmd) => {
                commands::hub::execute(cmd, &cli.rpc_url, &cli.wallet, cli.program_id.as_deref())
                    .await
            }
            InitCommands::Market(cmd) => {
                commands::market::execute(cmd, &cli.rpc_url, &cli.wallet, cli.program_id.as_deref())
                    .await
            }
            InitCommands::Full(cmd) => {
                commands::full_setup::execute(
                    cmd,
                    &cli.rpc_url,
                    &cli.wallet,
                    cli.program_id.as_deref(),
                )
                .await
            }
        },
    }
}
