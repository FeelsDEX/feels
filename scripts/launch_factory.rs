// Minimal Launch Factory CLI (MVP)
// Note: Requires configuring SDK environment and payer keypair

use std::str::FromStr;
use clap::Parser;
use serde::{Serialize, Deserialize};
use std::fs;
use solana_client::rpc_client::RpcClient;
use feels_sdk::{FeelsClient, SdkConfig};
use solana_sdk::pubkey::Pubkey;

#[derive(Parser, Debug)]
#[command(author, version, about = "Feels Launch Factory (MVP)")]
struct Args {
    #[arg(long)]
    rpc_url: String,
    #[arg(long)]
    ws_url: String,
    #[arg(long)]
    program_id: String,
    #[arg(long)]
    payer_path: String,
    #[arg(long)]
    token_0: String,
    #[arg(long)]
    token_1: String,
    #[arg(long)]
    feelssol_mint: String,
    #[arg(long, default_value_t = 30)]
    base_fee_bps: u16,
    #[arg(long)]
    tick_spacing: u16,
    #[arg(long)]
    initial_sqrt_price: u128,
    #[arg(long, default_value_t = 0)]
    initial_buy: u64,
    #[arg(long, default_value_t = 100)]
    tick_step_size: i32,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let payer = feels_sdk::load_keypair(&args.payer_path)?;
    let config = SdkConfig::new(
        args.rpc_url,
        args.ws_url,
        Pubkey::from_str(&args.program_id)?,
        payer,
    );
    let client = FeelsClient::new(config)?;
    let token_0 = Pubkey::from_str(&args.token_0)?;
    let token_1 = Pubkey::from_str(&args.token_1)?;
    let feelssol = Pubkey::from_str(&args.feelssol_mint)?;

    // Preflight checks
    println!("Preflight checks:");
    if token_0 >= token_1 { anyhow::bail!("token_0 must be < token_1"); }
    if token_0 != feelssol && token_1 != feelssol { anyhow::bail!("One token must be FeelsSOL (hub)"); }
    let (market, _) = feels_sdk::find_market_address(&token_0, &token_1);
    let (buffer, _) = feels_sdk::find_buffer_address(&market);
    println!("  market PDA: {}", market);
    println!("  buffer PDA: {}", buffer);
    if args.base_fee_bps == 0 || args.base_fee_bps > 1000 { anyhow::bail!("base_fee_bps out of range"); }
    if args.tick_spacing == 0 { anyhow::bail!("tick_spacing must be > 0"); }
    if args.initial_sqrt_price == 0 { anyhow::bail!("initial_sqrt_price must be > 0"); }
    // Rent estimates
    let rpc = RpcClient::new(args.rpc_url.clone());
    let rent_market = rpc.get_minimum_balance_for_rent_exemption(feels::state::Market::LEN)?;
    let rent_buffer = rpc.get_minimum_balance_for_rent_exemption(feels::state::Buffer::LEN)?;
    let rent_oracle = rpc.get_minimum_balance_for_rent_exemption(feels::state::OracleState::LEN)?;
    let payer_balance = rpc.get_balance(&client.config.payer.pubkey())?;
    println!("  rent (market): {}", rent_market);
    println!("  rent (buffer): {}", rent_buffer);
    println!("  rent (oracle): {}", rent_oracle);
    println!("  payer balance: {}", payer_balance);
    let rent_total = rent_market + rent_buffer + rent_oracle;
    if payer_balance < rent_total { anyhow::bail!("insufficient payer balance for rent"); }
    
    println!("Launching pool...");
    let (sig_init, sig_deploy) = client.launch_pool(
        &token_0,
        &token_1,
        &feelssol,
        args.base_fee_bps,
        args.tick_spacing,
        args.initial_sqrt_price,
        args.initial_buy,
        None,
        None,
        args.tick_step_size,
    )?;
    println!("initialize_market: {}", sig_init);
    println!("deploy_initial_liquidity: {}", sig_deploy);

    // Persist launch state
    #[derive(Serialize, Deserialize)]
    struct LaunchState { market: String, buffer: String, sig_init: String, sig_deploy: String }
    let state = LaunchState { market: market.to_string(), buffer: buffer.to_string(), sig_init: sig_init.to_string(), sig_deploy: sig_deploy.to_string() };
    fs::write("launch_state.json", serde_json::to_vec_pretty(&state)?)?;
    Ok(())
}
