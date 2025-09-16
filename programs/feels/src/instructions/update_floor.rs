use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};
use crate::{
    constants::VAULT_SEED,
    error::FeelsError,
    events::FloorRatcheted,
    state::{Market, Buffer},
};
use orca_whirlpools_core::tick_index_to_sqrt_price;
use ethnum::U256;

#[derive(Accounts)]
pub struct UpdateFloor<'info> {
    #[account(mut, constraint = market.is_initialized @ FeelsError::MarketNotInitialized)]
    pub market: Account<'info, Market>,
    
    /// Buffer must be associated with this market
    #[account(
        constraint = buffer.market == market.key() @ FeelsError::InvalidAuthority
    )]
    pub buffer: Account<'info, Buffer>,
    
    /// Vault 0 - must be the correct PDA for this market
    #[account(
        mut,
        seeds = [VAULT_SEED, market.token_0.as_ref(), market.token_1.as_ref(), b"0"],
        bump = market.vault_0_bump,
        constraint = vault_0.mint == market.token_0 @ FeelsError::InvalidVaultMint
    )]
    pub vault_0: Account<'info, TokenAccount>,
    
    /// Vault 1 - must be the correct PDA for this market
    #[account(
        mut,
        seeds = [VAULT_SEED, market.token_0.as_ref(), market.token_1.as_ref(), b"1"],
        bump = market.vault_1_bump,
        constraint = vault_1.mint == market.token_1 @ FeelsError::InvalidVaultMint
    )]
    pub vault_1: Account<'info, TokenAccount>,
    
    /// Project mint must be the non-FeelsSOL token in this market
    #[account(
        constraint = (project_mint.key() == market.token_0 && market.token_1 == market.feelssol_mint) ||
                    (project_mint.key() == market.token_1 && market.token_0 == market.feelssol_mint) 
                    @ FeelsError::InvalidProjectMint
    )]
    pub project_mint: Account<'info, Mint>,
    
    /// Optional: Pre-launch escrow token account (if tokens still in escrow)
    /// CHECK: Validated in handler if present
    pub escrow_token_account: Option<UncheckedAccount<'info>>,
    
    /// Optional: Other protocol-owned token accounts to exclude
    /// These would be accounts holding tokens that should not be considered circulating
    /// Note: This is handled as remaining_accounts in the instruction handler
    
    pub clock: Sysvar<'info, Clock>,
}

pub fn update_floor(ctx: Context<UpdateFloor>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let buffer = &ctx.accounts.buffer;
    let clock = &ctx.accounts.clock;

    // Identify FeelsSOL as token_0 or token_1
    let feelssol_is_token_0 = market.token_0 == market.feelssol_mint;
    let (feels_vault, project_vault) = if feelssol_is_token_0 {
        (&ctx.accounts.vault_0, &ctx.accounts.vault_1)
    } else {
        (&ctx.accounts.vault_1, &ctx.accounts.vault_0)
    };

    // Compute reserves and circulating supply
    let feels_reserve: u128 = buffer.tau_spot.saturating_add(feels_vault.amount as u128);
    let total_supply: u128 = ctx.accounts.project_mint.supply as u128;
    
    // Start with pool-owned tokens
    let mut non_circulating: u128 = project_vault.amount as u128;
    
    // Add escrow balance if provided
    if let Some(escrow_account) = &ctx.accounts.escrow_token_account {
        // Deserialize and validate escrow token account
        let escrow_token = TokenAccount::try_deserialize(
            &mut &escrow_account.to_account_info().data.borrow()[..]
        )?;
        
        // Verify it's for the correct mint
        require!(
            escrow_token.mint == ctx.accounts.project_mint.key(),
            FeelsError::InvalidMint
        );
        
        non_circulating = non_circulating.saturating_add(escrow_token.amount as u128);
    }
    
    // Add any other protocol-owned accounts from remaining_accounts
    for account_info in ctx.remaining_accounts {
        // Deserialize and validate as token account
        let token_account = TokenAccount::try_deserialize(
            &mut &account_info.data.borrow()[..]
        )?;
        
        // Verify it's for the correct mint
        require!(
            token_account.mint == ctx.accounts.project_mint.key(),
            FeelsError::InvalidMint
        );
        non_circulating = non_circulating.saturating_add(token_account.amount as u128);
    }
    
    // Check if there's a governance override for protocol-owned amount
    if buffer.protocol_owned_override > 0 {
        // Use the override value instead of dynamically calculated amount
        non_circulating = buffer.protocol_owned_override as u128;
    }
    
    // Calculate actual circulating supply
    let circulating: u128 = total_supply.saturating_sub(non_circulating).max(1);

    // Binary search tick for floor price where price = feels/circulating
    // Compare price_num * circulating <= feels << 128, where price_num = (sqrt_price_q64^2)
    let target = U256::from(feels_reserve) << 128;
    let min_tick = market.global_lower_tick.max(-887272);
    let max_tick = market.current_tick.min(887272);
    let mut lo = min_tick;
    let mut hi = max_tick;
    let mut best = lo;
    while lo <= hi {
        let mid = lo + ((hi - lo) / 2);
        let sqrt_q64 = tick_index_to_sqrt_price(mid);
        let sq = U256::from(sqrt_q64) * U256::from(sqrt_q64); // Q128.128
        let lhs = sq * U256::from(circulating);
        if lhs <= target { // price(mid) <= feels/circ
            best = mid; // move up
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    let candidate = best.saturating_sub(market.floor_buffer_ticks);

    if clock.unix_timestamp.saturating_sub(market.last_floor_ratchet_ts) >= market.floor_cooldown_secs
        && candidate > market.floor_tick
    {
        let old = market.floor_tick;
        market.floor_tick = candidate;
        market.last_floor_ratchet_ts = clock.unix_timestamp;
        emit!(FloorRatcheted {
            market: market.key(),
            old_floor_tick: old,
            new_floor_tick: market.floor_tick,
            timestamp: clock.unix_timestamp,
        });
    }
    Ok(())
}

