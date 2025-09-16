//! Market phase transition
//!
//! Manages the lifecycle of markets through different phases

use crate::{
    error::FeelsError,
    events::MarketPhaseTransitioned,
    state::*,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TransitionMarketPhase<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ FeelsError::Unauthorized,
        constraint = market.hub_protocol == Some(protocol_config.key()) @ FeelsError::InvalidProtocol,
    )]
    pub market: Account<'info, Market>,

    #[account(
        constraint = protocol_config.key() == market.hub_protocol.unwrap_or_default() @ FeelsError::InvalidProtocol,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    
    #[account(
        constraint = oracle.key() == market.oracle @ FeelsError::InvalidOracle,
    )]
    pub oracle: Account<'info, OracleState>,
    
    #[account(
        mut,
        constraint = buffer.key() == market.buffer @ FeelsError::InvalidBuffer,
    )]
    pub buffer: Account<'info, Buffer>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct TransitionPhaseParams {
    /// Target phase to transition to
    pub target_phase: MarketPhase,
    /// Force transition even if criteria not met (governance only)
    pub force: bool,
}

pub fn transition_market_phase(
    ctx: Context<TransitionMarketPhase>,
    params: TransitionPhaseParams,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let _oracle = &ctx.accounts.oracle;
    let _buffer = &ctx.accounts.buffer;
    
    let clock = Clock::get()?;
    let current_slot = clock.slot;
    let current_timestamp = clock.unix_timestamp;

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

    // Validate transition
    if !params.force && !current_phase.can_transition_to(params.target_phase) {
        return Err(FeelsError::InvalidPhaseTransition.into());
    }

    // Check transition criteria based on target phase
    let trigger = if !params.force {
        match params.target_phase {
            MarketPhase::SteadyState => {
                // Check if bonding curve graduation criteria met
                if market.total_volume_token_0 + market.total_volume_token_1 >= GRADUATION_VOLUME_THRESHOLD {
                    PhaseTrigger::VolumeThreshold
                } else if market.liquidity >= GRADUATION_LIQUIDITY_THRESHOLD {
                    PhaseTrigger::LiquidityThreshold
                } else if current_timestamp - market.phase_start_timestamp >= GRADUATION_TIME_THRESHOLD {
                    PhaseTrigger::TimeElapsed
                } else if params.force {
                    PhaseTrigger::Governance
                } else {
                    return Err(FeelsError::GraduationCriteriaNotMet.into());
                }
            }
            MarketPhase::Paused => {
                // Safety controller triggered pause
                PhaseTrigger::SafetyTrigger
            }
            MarketPhase::Deprecated => {
                // Governance deprecation
                PhaseTrigger::Governance
            }
            _ => {
                // Default to creator/governance action
                if ctx.accounts.authority.key() == market.authority {
                    PhaseTrigger::Creator
                } else {
                    PhaseTrigger::Governance
                }
            }
        }
    } else {
        PhaseTrigger::Governance
    };

    // Execute phase-specific transitions
    match (current_phase, params.target_phase) {
        (MarketPhase::Created, MarketPhase::BondingCurve) => {
            // Initialize bonding curve state
            market.steady_state_seeded = false;
            market.cleanup_complete = false;
        }
        
        (MarketPhase::BondingCurve, MarketPhase::Transitioning) => {
            // Start transition to AMM
            // This is where we'd start moving liquidity from bonding curve to AMM
        }
        
        (MarketPhase::Transitioning, MarketPhase::SteadyState) => {
            // Complete transition
            market.steady_state_seeded = true;
        }
        
        (MarketPhase::SteadyState, MarketPhase::Graduated) => {
            // Cleanup bonding curve
            market.cleanup_complete = true;
        }
        
        (_, MarketPhase::Paused) => {
            // Pause trading
            market.is_paused = true;
        }
        
        (MarketPhase::Paused, _) => {
            // Unpause trading
            market.is_paused = false;
        }
        
        _ => {
            // Other transitions don't require special handling
        }
    }

    // Update phase tracking
    market.phase = params.target_phase as u8;
    market.phase_start_slot = current_slot;
    market.phase_start_timestamp = current_timestamp;
    market.last_phase_transition_slot = current_slot;
    market.last_phase_trigger = trigger as u8;

    // Emit event
    emit!(MarketPhaseTransitioned {
        market: market.key(),
        from_phase: current_phase as u8,
        to_phase: params.target_phase as u8,
        trigger: trigger as u8,
        total_volume: market.total_volume_token_0 + market.total_volume_token_1,
        total_liquidity: market.liquidity,
        timestamp: current_timestamp,
        slot: current_slot,
    });

    Ok(())
}

// Graduation thresholds
const GRADUATION_VOLUME_THRESHOLD: u64 = 1_000_000_000_000; // 1M tokens
const GRADUATION_LIQUIDITY_THRESHOLD: u128 = 100_000_000_000; // 100k tokens
const GRADUATION_TIME_THRESHOLD: i64 = 86400; // 24 hours