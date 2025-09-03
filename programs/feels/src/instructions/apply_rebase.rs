/// Instruction for applying exact exponential rebase updates.
/// 
/// This instruction allows keepers to update rebase indices with pre-computed
/// exact exponential factors that satisfy conservation laws.
use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::state::rebase::{RebaseAccumulator, WeightRebaseFactors, DomainWeights, DomainValues};
use crate::state::{ProtocolState, MarketDataSource};
use crate::logic::conservation_check::{ConservationProof, verify_rebase_conservation};
use crate::logic::event::{RebaseEvent, RebaseEventType};

/// Parameters for rebase operation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum RebaseOperation {
    /// Update lending/funding indices with exact factors
    UpdateIndices {
        growth_factor_0: Option<u128>,
        growth_factor_1: Option<u128>,
        growth_factor_long: Option<u128>,
        growth_factor_short: Option<u128>,
        conservation_proof: ConservationProof,
    },
    
    /// Apply weight rebase when domain weights change
    ApplyWeightRebase {
        new_weights: DomainWeights,
        rebase_factors: WeightRebaseFactors,
        conservation_proof: ConservationProof,
    },
}

#[derive(Accounts)]
#[instruction(operation: RebaseOperation)]
pub struct ApplyRebase<'info> {
    /// Global rebase accumulator
    #[account(
        mut,
        seeds = [b"rebase_accumulator", pool.key().as_ref()],
        bump
    )]
    pub rebase_accumulator: AccountLoader<'info, RebaseAccumulator>,
    
    /// Market data source for keeper authorization
    #[account(
        seeds = [b"market_data_source", pool.key().as_ref()],
        bump,
        constraint = market_data_source.load()?.is_active != 0 @ FeelsProtocolError::StateError
    )]
    pub market_data_source: AccountLoader<'info, MarketDataSource>,
    
    /// Protocol state (for weight rebase admin check)
    #[account(
        seeds = [b"protocol"],
        bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    
    /// Pool reference
    /// CHECK: Used for PDA derivation only
    pub pool: UncheckedAccount<'info>,
    
    /// Authority performing the update (keeper or admin)
    pub authority: Signer<'info>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<ApplyRebase>,
    operation: RebaseOperation,
) -> Result<()> {
    let current_time = Clock::get()?.unix_timestamp;
    let mut accumulator_data = ctx.accounts.rebase_accumulator.load_mut()?;
    let data_source = ctx.accounts.market_data_source.load()?;
    
    match operation {
        RebaseOperation::UpdateIndices {
            growth_factor_0,
            growth_factor_1,
            growth_factor_long,
            growth_factor_short,
            conservation_proof,
        } => {
            // Verify keeper authorization
            require!(
                ctx.accounts.authority.key() == data_source.primary_provider ||
                ctx.accounts.authority.key() == data_source.secondary_provider,
                FeelsProtocolError::Unauthorized
            );
            
            // Determine operation type for logging
            let operation_type = if growth_factor_0.is_some() || growth_factor_1.is_some() {
                "lending"
            } else if growth_factor_long.is_some() || growth_factor_short.is_some() {
                "leverage"
            } else {
                return Err(FeelsProtocolError::InvalidInput.into());
            };
            
            msg!("Applying {} rebase update", operation_type);
            msg!("  Time elapsed: {} seconds", current_time - accumulator_data.last_update);
            
            // Update indices with conservation check
            accumulator_data.update_indices_with_factors(
                current_time,
                growth_factor_0,
                growth_factor_1,
                growth_factor_long,
                growth_factor_short,
                Some(&conservation_proof),
            )?;
            
            // Emit event
            emit!(RebaseEvent {
                pool: ctx.accounts.pool.key(),
                event_type: if operation_type == "lending" {
                    RebaseEventType::LendingRebase
                } else {
                    RebaseEventType::LeverageRebase
                },
                index_0: accumulator_data.index_0,
                index_1: accumulator_data.index_1,
                funding_index_long: accumulator_data.funding_index_long,
                funding_index_short: accumulator_data.funding_index_short,
                timestamp: current_time,
                authority: ctx.accounts.authority.key(),
            });
            
            msg!("Rebase indices updated successfully");
            if let Some(g_a) = growth_factor_0 {
                msg!("  Token 0 growth: {}", g_a);
            }
            if let Some(g_b) = growth_factor_1 {
                msg!("  Token 1 growth: {}", g_b);
            }
            if let Some(g_long) = growth_factor_long {
                msg!("  Long funding growth: {}", g_long);
            }
            if let Some(g_short) = growth_factor_short {
                msg!("  Short funding growth: {}", g_short);
            }
        }
        
        RebaseOperation::ApplyWeightRebase {
            new_weights,
            rebase_factors,
            conservation_proof,
        } => {
            // Weight rebase requires admin authority
            require!(
                ctx.accounts.authority.key() == ctx.accounts.protocol_state.authority,
                FeelsProtocolError::Unauthorized
            );
            
            msg!("Applying weight rebase");
            let weight_a = accumulator_data.weight_a;
            let weight_b = accumulator_data.weight_b;
            let weight_long = accumulator_data.weight_long;
            let weight_short = accumulator_data.weight_short;
            msg!("  Old weights: S={}, T={}, L={}, tau={}", 
                weight_a, weight_b, weight_long, weight_short);
            msg!("  New weights: S={}, T={}, L={}, tau={}", 
                new_weights.w_s, new_weights.w_t, 
                new_weights.w_l, new_weights.w_tau);
            
            // Verify conservation
            verify_rebase_conservation("weight_rebase", &conservation_proof)?;
            
            // Create validation params
            let params = crate::state::rebase::WeightRebaseParams {
                old_weights: DomainWeights {
                    w_s: accumulator_data.weight_a,
                    w_t: accumulator_data.weight_b,
                    w_l: accumulator_data.weight_long,
                    w_tau: 10000 - accumulator_data.weight_a - accumulator_data.weight_b - accumulator_data.weight_long,
                },
                new_weights: new_weights.clone(),
                domain_values: DomainValues::default(), // Not needed for validation
            };
            
            // Validate factors
            rebase_factors.validate(&params)?;
            
            // Update weights in accumulator
            accumulator_data.weight_a = new_weights.w_s;
            accumulator_data.weight_b = new_weights.w_t;
            accumulator_data.weight_long = new_weights.w_l;
            accumulator_data.weight_short = new_weights.w_tau;
            accumulator_data.last_update = current_time;
            
            // Emit event
            emit!(RebaseEvent {
                pool: ctx.accounts.pool.key(),
                event_type: RebaseEventType::WeightRebase,
                index_0: accumulator_data.index_0,
                index_1: accumulator_data.index_1,
                funding_index_long: accumulator_data.funding_index_long,
                funding_index_short: accumulator_data.funding_index_short,
                timestamp: current_time,
                authority: ctx.accounts.authority.key(),
            });
            
            msg!("Weight rebase completed successfully");
            msg!("  Rebase factors: h_S={}, h_T={}, h_L={}, h_tau={}", 
                rebase_factors.h_S, rebase_factors.h_T, 
                rebase_factors.h_L, rebase_factors.h_tau);
        }
    }
    
    Ok(())
}