/// Weight rebase instruction for atomic weight updates in the market physics model.
/// Weight changes must apply atomically with all rebases in one transaction.
use anchor_lang::prelude::*;
use crate::state::{Pool, MarketState, BufferAccount, DomainWeights};
use crate::logic::rebase::{calculate_weight_rebase, WeightRebaseFactors};
use crate::error::FeelsProtocolError;

// ============================================================================
// Instruction Context
// ============================================================================

#[derive(Accounts)]
pub struct ExecuteWeightRebase<'info> {
    /// Pool being rebalanced
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    /// Market state account
    #[account(
        mut,
        seeds = [b"market_state", pool.key().as_ref()],
        bump,
    )]
    pub market_state: AccountLoader<'info, MarketState>,
    
    /// Buffer account
    #[account(
        mut,
        seeds = [b"buffer", pool.key().as_ref()],
        bump,
    )]
    pub buffer: AccountLoader<'info, BufferAccount>,
    
    /// Pool authority (governance or admin)
    pub authority: Signer<'info>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

// ============================================================================
// Instruction Handler
// ============================================================================

/// Execute atomic weight rebase
pub fn execute_weight_rebase(
    ctx: Context<ExecuteWeightRebase>,
    new_weights: DomainWeights,
) -> Result<()> {
    // Load accounts
    let pool = &mut ctx.accounts.pool.load_mut()?;
    let market_state = &mut ctx.accounts.market_state.load_mut()?;
    let buffer = &mut ctx.accounts.buffer.load_mut()?;
    
    // Verify authority
    require!(
        ctx.accounts.authority.key() == pool.authority,
        FeelsProtocolError::InvalidAuthority
    );
    
    // Get current weights
    let old_weights = market_state.get_weights();
    
    // Calculate rebase factors
    let factors = calculate_weight_rebase(
        &old_weights,
        &new_weights,
        market_state,
    )?;
    
    // Apply rebases atomically to all balances
    apply_weight_rebases(market_state, buffer, &factors)?;
    
    // Update weights
    market_state.set_weights(new_weights);
    
    // Update pool state
    pool.last_weight_update = Clock::get()?.unix_timestamp;
    
    // Emit event
    emit!(WeightRebaseEvent {
        pool: pool.key(),
        old_weights,
        new_weights,
        factors: factors.clone(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}

// ============================================================================
// Rebase Application
// ============================================================================

/// Apply weight rebases to all dimensional values
fn apply_weight_rebases(
    market_state: &mut MarketState,
    buffer: &mut BufferAccount,
    factors: &WeightRebaseFactors,
) -> Result<()> {
    // Apply to dimensional values
    market_state.S = apply_rebase_factor(market_state.S, factors.h_S)?;
    market_state.T = apply_rebase_factor(market_state.T, factors.h_T)?;
    market_state.L = apply_rebase_factor(market_state.L, factors.h_L)?;
    
    // Apply to buffer
    buffer.tau_value = apply_rebase_factor(buffer.tau_value, factors.h_tau)?;
    
    // Update invariants
    market_state.update_invariants()?;
    
    // Update timestamp
    market_state.last_update = Clock::get()?.unix_timestamp;
    
    Ok(())
}

/// Apply a single rebase factor
fn apply_rebase_factor(value: u128, factor: u128) -> Result<u128> {
    value
        .checked_mul(factor)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(1u128 << 64)
        .ok_or(FeelsProtocolError::DivisionByZero.into())
}

// ============================================================================
// Governance Integration
// ============================================================================

#[derive(Accounts)]
pub struct ProposeWeightChange<'info> {
    /// Governance proposal account
    /// TODO: Would integrate with actual governance system
    #[account(
        init,
        payer = proposer,
        space = 8 + WeightProposal::SIZE,
        seeds = [b"weight_proposal", pool.key().as_ref(), proposal_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub proposal: Account<'info, WeightProposal>,
    
    /// Pool for weight change
    pub pool: AccountLoader<'info, Pool>,
    
    /// Proposer
    #[account(mut)]
    pub proposer: Signer<'info>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

/// Weight change proposal
#[account]
pub struct WeightProposal {
    /// Pool being modified
    pub pool: Pubkey,
    
    /// Proposal ID
    pub proposal_id: u64,
    
    /// Proposed new weights
    pub new_weights: DomainWeights,
    
    /// Current weights snapshot
    pub current_weights: DomainWeights,
    
    /// Proposer
    pub proposer: Pubkey,
    
    /// Creation time
    pub created_at: i64,
    
    /// Execution time (if approved)
    pub execute_after: i64,
    
    /// Status
    pub status: ProposalStatus,
    
    /// Vote counts (simplified)
    pub votes_for: u64,
    pub votes_against: u64,
}

impl WeightProposal {
    pub const SIZE: usize = 32 + 8 + 16 + 16 + 32 + 8 + 8 + 1 + 8 + 8;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
    Cancelled,
}

// ============================================================================
// Emergency Weight Update
// ============================================================================

#[derive(Accounts)]
pub struct EmergencyWeightUpdate<'info> {
    /// Pool being updated
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    /// Market state
    #[account(mut)]
    pub market_state: AccountLoader<'info, MarketState>,
    
    /// Buffer account
    #[account(mut)]
    pub buffer: AccountLoader<'info, BufferAccount>,
    
    /// Emergency authority (multisig)
    pub emergency_authority: Signer<'info>,
    
    /// Additional signers for multisig
    /// TODO: In production, would verify threshold signatures
}

/// Emergency weight update with safeguards
pub fn emergency_weight_update(
    ctx: Context<EmergencyWeightUpdate>,
    new_weights: DomainWeights,
    justification: String,
) -> Result<()> {
    let pool = &ctx.accounts.pool.load()?;
    
    // Verify emergency authority
    require!(
        ctx.accounts.emergency_authority.key() == pool.emergency_authority,
        FeelsProtocolError::InvalidAuthority
    );
    
    // Limit emergency changes to smaller adjustments
    let market_state = &ctx.accounts.market_state.load()?;
    let current_weights = market_state.get_weights();
    
    // Check each weight change is within emergency limits (5%)
    let emergency_limit = 500u32;
    
    require!(
        (new_weights.w_s as i32 - current_weights.w_s as i32).abs() <= emergency_limit as i32,
        FeelsProtocolError::ExcessiveWeightChange
    );
    require!(
        (new_weights.w_t as i32 - current_weights.w_t as i32).abs() <= emergency_limit as i32,
        FeelsProtocolError::ExcessiveWeightChange
    );
    require!(
        (new_weights.w_l as i32 - current_weights.w_l as i32).abs() <= emergency_limit as i32,
        FeelsProtocolError::ExcessiveWeightChange
    );
    
    // Execute weight rebase
    let mut rebase_ctx = Context::new(
        ctx.program_id,
        &mut ExecuteWeightRebase {
            pool: ctx.accounts.pool.clone(),
            market_state: ctx.accounts.market_state.clone(),
            buffer: ctx.accounts.buffer.clone(),
            authority: ctx.accounts.emergency_authority.clone(),
            system_program: ctx.accounts.system_program.clone(),
        },
        &[],
        BTreeMap::new(),
    );
    
    execute_weight_rebase(rebase_ctx, new_weights)?;
    
    // Emit emergency event
    emit!(EmergencyWeightUpdateEvent {
        pool: pool.key(),
        old_weights: current_weights,
        new_weights,
        justification,
        authority: ctx.accounts.emergency_authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct WeightRebaseEvent {
    pub pool: Pubkey,
    pub old_weights: DomainWeights,
    pub new_weights: DomainWeights,
    pub factors: WeightRebaseFactors,
    pub timestamp: i64,
}

#[event]
pub struct EmergencyWeightUpdateEvent {
    pub pool: Pubkey,
    pub old_weights: DomainWeights,
    pub new_weights: DomainWeights,
    pub justification: String,
    pub authority: Pubkey,
    pub timestamp: i64,
}

// ============================================================================
// Validation Helpers
// ============================================================================

/// Validate proposed weight configuration
pub fn validate_weight_proposal(weights: &DomainWeights) -> Result<()> {
    // Ensure weights sum to 10000
    let sum = weights.w_s + weights.w_t + weights.w_l + weights.w_tau;
    require!(
        sum == DomainWeights::SCALE,
        FeelsProtocolError::InvalidWeights
    );
    
    // Ensure minimum weights (1% each)
    let min_weight = 100u32;
    require!(
        weights.w_s >= min_weight,
        FeelsProtocolError::InvalidWeights
    );
    require!(
        weights.w_t >= min_weight,
        FeelsProtocolError::InvalidWeights
    );
    require!(
        weights.w_l >= min_weight,
        FeelsProtocolError::InvalidWeights
    );
    
    // Ensure tau doesn't dominate (max 50%)
    require!(
        weights.w_tau <= 5000,
        FeelsProtocolError::InvalidWeights
    );
    
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_weight_validation() {
        // Valid weights
        let valid = DomainWeights::new(2500, 2500, 2500, 2500).unwrap();
        assert!(validate_weight_proposal(&valid).is_ok());
        
        // Invalid sum
        let invalid_sum = DomainWeights {
            w_s: 2500,
            w_t: 2500,
            w_l: 2500,
            w_tau: 2600, // Sum = 10100
        };
        assert!(validate_weight_proposal(&invalid_sum).is_err());
        
        // Weight too small
        let too_small = DomainWeights {
            w_s: 50, // < 1%
            w_t: 3300,
            w_l: 3300,
            w_tau: 3350,
        };
        assert!(validate_weight_proposal(&too_small).is_err());
    }
}