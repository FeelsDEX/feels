use anyhow::{ensure, Result};
use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, 
    Swap, SwapAndAccountMetas, SwapParams, AmmProgramIdToLabel, AmmLabel,
    try_get_account_data,
};
use solana_program::{
    pubkey::Pubkey,
    instruction::AccountMeta,
};
use anchor_lang::prelude::*;
use spl_token::state::Account as TokenAccount;
use solana_program::program_pack::Pack;

use feels::state::Market;

pub struct FeelsAmm {
    key: Pubkey,
    market: Market,
    authority: Pubkey,
    program_id: Pubkey,
    reserve_mints: [Pubkey; 2],
    reserves: [u64; 2],
    vault_0: Pubkey,
    vault_1: Pubkey,
}

impl AmmProgramIdToLabel for FeelsAmm {
    const PROGRAM_ID_TO_LABELS: &'static [(Pubkey, AmmLabel)] = &[
        (feels::ID, "Feels"),
    ];
}

impl Clone for FeelsAmm {
    fn clone(&self) -> Self {
        FeelsAmm {
            key: self.key,
            market: self.market.clone(),
            authority: self.authority,
            program_id: self.program_id,
            reserve_mints: self.reserve_mints,
            reserves: self.reserves,
            vault_0: self.vault_0,
            vault_1: self.vault_1,
        }
    }
}

impl Amm for FeelsAmm {
    fn from_keyed_account(keyed_account: &KeyedAccount, _amm_context: &AmmContext) -> Result<Self> {
        // Check program ID
        ensure!(
            keyed_account.account.owner == feels::ID,
            "Invalid program owner for Feels market"
        );
        
        // Deserialize the market account
        let data = &keyed_account.account.data;
        // Try to deserialize the full account data (Anchor handles discriminator internally)
        let market = Market::try_deserialize_unchecked(&mut &data[..])?;
        
        // Verify market is initialized
        ensure!(market.is_initialized, "Market not initialized");
        ensure!(!market.is_paused, "Market is paused");
        
        // Derive PDAs
        let program_id = feels::ID;
        let market_key = keyed_account.key;
        let (authority, _) = market.derive_market_authority_with_key(&market_key, &program_id);
        let ((vault_0, _), (vault_1, _)) = market.get_vault_addresses(&market_key, &program_id);
        
        // Store reserve mints before moving market
        let reserve_mints = [market.token_0, market.token_1];
        
        Ok(Self {
            key: keyed_account.key,
            market,
            authority,
            program_id,
            reserve_mints,
            reserves: [0, 0], // Will be updated in update()
            vault_0,
            vault_1,
        })
    }

    fn label(&self) -> String {
        "Feels".to_string()
    }

    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    fn key(&self) -> Pubkey {
        self.key
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        self.reserve_mints.to_vec()
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        vec![self.vault_0, self.vault_1]
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        // Update reserve amounts from vault accounts
        let vault_0_account = try_get_account_data(account_map, &self.vault_0)?;
        let vault_0_token_account = TokenAccount::unpack(vault_0_account)?;
        
        let vault_1_account = try_get_account_data(account_map, &self.vault_1)?;
        let vault_1_token_account = TokenAccount::unpack(vault_1_account)?;
        
        self.reserves = [
            vault_0_token_account.amount,
            vault_1_token_account.amount,
        ];
        
        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        // Determine swap direction
        let (is_token_0_to_1, amount_in) = if quote_params.input_mint == self.reserve_mints[0] {
            (true, quote_params.amount)
        } else if quote_params.input_mint == self.reserve_mints[1] {
            (false, quote_params.amount)
        } else {
            anyhow::bail!("Invalid input mint");
        };
        
        // Calculate swap output using Feels concentrated liquidity math
        let (amount_out, fee_amount) = calculate_swap_output(
            &self.market,
            amount_in,
            is_token_0_to_1,
            self.reserves[0],
            self.reserves[1],
        )?;
        
        // Calculate fee percentage as Decimal
        use rust_decimal::Decimal;
        use std::str::FromStr;
        
        let fee_pct = if amount_in > 0 {
            let fee_ratio = fee_amount as f64 / amount_in as f64;
            Decimal::from_str(&format!("{}", fee_ratio * 100.0)).unwrap_or_default()
        } else {
            Decimal::ZERO
        };
        
        Ok(Quote {
            in_amount: amount_in,
            out_amount: amount_out,
            fee_amount,
            fee_mint: quote_params.input_mint,
            fee_pct,
        })
    }

    fn get_accounts_len(&self) -> usize {
        13 // Number of accounts needed for Feels swap
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let SwapParams {
            source_mint,
            destination_mint,
            source_token_account,
            destination_token_account,
            token_transfer_authority,
            ..
        } = swap_params;
        
        // Verify mints
        let is_token_0_to_1 = if *source_mint == self.market.token_0 && *destination_mint == self.market.token_1 {
            true
        } else if *source_mint == self.market.token_1 && *destination_mint == self.market.token_0 {
            false
        } else {
            anyhow::bail!("Invalid mint pair for swap");
        };
        
        // Get vaults in correct order
        let (_vault_in, _vault_out) = if is_token_0_to_1 {
            (self.vault_0, self.vault_1)
        } else {
            (self.vault_1, self.vault_0)
        };
        
        // Build account metas for Feels swap instruction
        // Order must match Swap struct from swap.rs
        let account_metas = vec![
            AccountMeta::new_readonly(feels::ID, false), // Program ID (required by Jupiter)
            AccountMeta::new(*token_transfer_authority, true), // user (signer)
            AccountMeta::new(self.key, false), // market
            AccountMeta::new_readonly(Pubkey::default(), false), // oracle (optional)
            AccountMeta::new(self.vault_0, false), // vault_0
            AccountMeta::new(self.vault_1, false), // vault_1
            AccountMeta::new(*source_token_account, false), // user_token_in
            AccountMeta::new(*destination_token_account, false), // user_token_out
            AccountMeta::new_readonly(self.authority, false), // market_authority
            AccountMeta::new(self.market.buffer, false), // buffer
            AccountMeta::new_readonly(spl_token::id(), false), // token_program
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false), // clock
        ];
        
        Ok(SwapAndAccountMetas {
            swap: Swap::Saber, // Jupiter interface uses Saber variant for generic swaps
            account_metas,
        })
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }

    fn has_dynamic_accounts(&self) -> bool {
        false
    }

    fn supports_exact_out(&self) -> bool {
        true
    }
}

// Helper function to calculate swap output using Feels concentrated liquidity math
fn calculate_swap_output(
    market: &Market,
    amount_in: u64,
    is_token_0_to_1: bool,
    reserve_0: u64,
    reserve_1: u64,
) -> Result<(u64, u64)> {
    // Calculate base fee
    let fee_bps = market.base_fee_bps as u64;
    let fee_amount = (amount_in as u128 * fee_bps as u128 / 10_000) as u64;
    let amount_after_fee = amount_in.saturating_sub(fee_amount);
    
    // Use simplified constant product formula for now
    // In production, this would use proper concentrated liquidity math
    let (reserve_in, reserve_out) = if is_token_0_to_1 {
        (reserve_0 as u128, reserve_1 as u128)
    } else {
        (reserve_1 as u128, reserve_0 as u128)
    };
    
    // Constant product: x * y = k
    // After swap: (x + dx) * (y - dy) = k
    // Therefore: dy = y - k / (x + dx)
    let k = reserve_in * reserve_out;
    let new_reserve_in = reserve_in + amount_after_fee as u128;
    
    // Prevent division by zero
    ensure!(new_reserve_in > 0, "Invalid reserve state");
    
    let new_reserve_out = k / new_reserve_in;
    let amount_out = reserve_out.saturating_sub(new_reserve_out) as u64;
    
    Ok((amount_out, fee_amount))
}