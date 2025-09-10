//! Example of how to use the Feels Jupiter AMM adapter

use feels_jupiter_adapter::FeelsAmm;
use jupiter_amm_interface::{Amm, AmmContext, KeyedAccount, QuoteParams};
use solana_sdk::{
    pubkey::Pubkey,
    account::Account,
};
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    // Example: Create a FeelsAmm instance from on-chain data
    
    // This would be the Feels market account pubkey
    let market_key = Pubkey::from_str("YourFeelsMarketPubkeyHere")?;
    
    // In practice, you'd fetch this from the chain
    // This is a mock account for demonstration
    let mock_market_account = Account {
        lamports: 0,
        data: vec![0; 1024], // Market account data would be here
        owner: feels::ID,
        executable: false,
        rent_epoch: 0,
    };
    
    let keyed_account = KeyedAccount {
        key: market_key,
        account: mock_market_account,
        params: None,
    };
    
    let amm_context = AmmContext {
        clock_ref: solana_sdk::sysvar::clock::Clock::default().into(),
    };
    
    // Create the AMM instance
    let feels_amm = FeelsAmm::from_keyed_account(&keyed_account, &amm_context)?;
    
    // Get a quote for swapping 1000000 units (1 token with 6 decimals)
    let quote_params = QuoteParams {
        amount: 1_000_000,
        input_mint: feels_amm.get_reserve_mints()[0],
        output_mint: feels_amm.get_reserve_mints()[1],
        swap_mode: jupiter_amm_interface::SwapMode::ExactIn,
    };
    
    let quote = feels_amm.quote(&quote_params)?;
    
    println!("Quote:");
    println!("  Input: {} {}", quote.in_amount, quote_params.input_mint);
    println!("  Output: {} {}", quote.out_amount, quote_params.output_mint);
    println!("  Fee: {} ({:.2}%)", quote.fee_amount, quote.fee_pct);
    
    Ok(())
}