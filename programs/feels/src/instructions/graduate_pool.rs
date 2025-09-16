//! Graduate pool to steady state
//!
//! Transitions a market from bonding curve to graduated phase

use crate::{
    error::FeelsError, 
    events::MarketPhaseTransitioned,
    state::{Market, MarketPhase, PhaseTrigger},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct GraduatePool<'info> {
    /// Market authority performing graduation
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Market to graduate
    #[account(
        mut,
        constraint = market.authority == authority.key() @ FeelsError::UnauthorizedSigner,
        constraint = market.is_initialized @ FeelsError::MarketNotInitialized,
    )]
    pub market: Account<'info, Market>,
}

pub fn graduate_pool(ctx: Context<GraduatePool>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let clock = Clock::get()?;
    
    // Get current phase
    let current_phase = match market.phase {
        0 => MarketPhase::Created,
        1 => MarketPhase::BondingCurve,
        2 => MarketPhase::Transitioning,
        3 => MarketPhase::SteadyState,
        4 => MarketPhase::Graduated,
        5 => MarketPhase::Paused,
        6 => MarketPhase::Deprecated,
        _ => return Err(FeelsError::InvalidPhase.into()),
    };
    
    // Can only graduate from steady state
    if current_phase != MarketPhase::SteadyState {
        return Err(FeelsError::InvalidPhaseTransition.into());
    }
    
    // Update to graduated phase
    market.phase = MarketPhase::Graduated as u8;
    market.phase_start_slot = clock.slot;
    market.phase_start_timestamp = clock.unix_timestamp;
    market.last_phase_transition_slot = clock.slot;
    market.last_phase_trigger = PhaseTrigger::Creator as u8;
    
    // Set graduation flags
    market.steady_state_seeded = true;
    market.cleanup_complete = true;
    
    // Emit event
    emit!(MarketPhaseTransitioned {
        market: market.key(),
        from_phase: current_phase as u8,
        to_phase: MarketPhase::Graduated as u8,
        trigger: PhaseTrigger::Creator as u8,
        total_volume: market.total_volume_token_0 + market.total_volume_token_1,
        total_liquidity: market.liquidity,
        timestamp: clock.unix_timestamp,
        slot: clock.slot,
    });
    
    Ok(())
}
