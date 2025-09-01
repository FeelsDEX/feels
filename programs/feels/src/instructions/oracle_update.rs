/// Simplified oracle update instruction replacing complex keeper system.
/// Handles market parameter updates with basic validation.
use anchor_lang::prelude::*;
use crate::state::{Pool, MarketState};
use crate::logic::oracle::{OracleConfig, OracleUpdate, validate_oracle_update, apply_oracle_update};
use crate::logic::event::OracleUpdateEvent;
use crate::error::FeelsError;

// ============================================================================
// Oracle Update Instruction
// ============================================================================

#[derive(Accounts)]
pub struct UpdateOracle<'info> {
    /// Pool being updated
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    /// Market state for the pool
    #[account(
        mut,
        seeds = [b"market_state", pool.key().as_ref()],
        bump,
    )]
    pub market_state: Account<'info, MarketState>,
    
    /// Oracle configuration
    #[account(
        mut,
        seeds = [b"oracle_config", pool.key().as_ref()],
        bump,
    )]
    pub oracle_config: Account<'info, OracleConfig>,
    
    /// Oracle authority (must match config)
    pub oracle: Signer<'info>,
}

/// Update market parameters via oracle
pub fn update_oracle(
    ctx: Context<UpdateOracle>,
    update: OracleUpdate,
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // Validate the oracle update
    validate_oracle_update(
        &update,
        &ctx.accounts.oracle_config,
        current_time,
    )?;
    
    // Ensure oracle matches signer
    require!(
        update.oracle == ctx.accounts.oracle.key(),
        FeelsError::UnauthorizedError {
            action: "oracle_update".to_string(),
            reason: "Update oracle doesn't match signer".to_string(),
        }
    );
    
    // Apply the update
    apply_oracle_update(
        &mut ctx.accounts.market_state,
        &mut ctx.accounts.oracle_config,
        &update,
    )?;
    
    // Emit event
    emit!(OracleUpdateEvent {
        pool: ctx.accounts.pool.key(),
        oracle: update.oracle,
        timestamp: update.timestamp,
        spot_gradient: update.parameters.spot_gradient,
        rate_gradient: update.parameters.rate_gradient,
        leverage_gradient: update.parameters.leverage_gradient,
        market_curvature: update.parameters.market_curvature,
        risk_adjustment: update.parameters.risk_adjustment,
        volatility: update.parameters.volatility,
    });
    
    msg!("Oracle update successful");
    msg!("Pool: {}", ctx.accounts.pool.key());
    msg!("Oracle: {}", update.oracle);
    msg!("Timestamp: {}", update.timestamp);
    
    Ok(())
}

// ============================================================================
// Initialize Oracle Config
// ============================================================================

#[derive(Accounts)]
pub struct InitializeOracleConfig<'info> {
    /// Pool to configure oracle for
    pub pool: AccountLoader<'info, Pool>,
    
    /// Oracle configuration to initialize
    #[account(
        init,
        payer = payer,
        space = OracleConfig::SIZE,
        seeds = [b"oracle_config", pool.key().as_ref()],
        bump,
    )]
    pub oracle_config: Account<'info, OracleConfig>,
    
    /// Pool authority
    pub authority: Signer<'info>,
    
    /// Payer for account creation
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

/// Initialize oracle configuration for a pool
pub fn initialize_oracle_config(
    ctx: Context<InitializeOracleConfig>,
    primary_oracle: Pubkey,
    secondary_oracle: Pubkey,
    update_frequency: i64,
) -> Result<()> {
    let pool = ctx.accounts.pool.load()?;
    
    // Verify authority
    require!(
        ctx.accounts.authority.key() == pool.pool_creator,
        FeelsError::UnauthorizedError {
            action: "initialize_oracle".to_string(),
            reason: "Only pool creator can set oracle".to_string(),
        }
    );
    
    // Validate update frequency
    require!(
        update_frequency >= 30 && update_frequency <= 3600,
        FeelsError::ParameterError {
            parameter: "update_frequency".to_string(),
            reason: "Frequency must be 30-3600 seconds".to_string(),
        }
    );
    
    // Initialize config with default parameters
    let oracle_config = &mut ctx.accounts.oracle_config;
    oracle_config.primary_oracle = primary_oracle;
    oracle_config.secondary_oracle = secondary_oracle;
    oracle_config.update_frequency = update_frequency;
    oracle_config.last_update = 0;
    
    // Set initial parameters (neutral values)
    oracle_config.current_parameters = crate::logic::oracle::MarketParameters {
        spot_gradient: -(1i64 << 32),      // -1.0 in fixed point
        rate_gradient: -(1i64 << 32),      // -1.0
        leverage_gradient: -(1i64 << 32),  // -1.0
        market_curvature: 1u64 << 32,      // 1.0
        risk_adjustment: 100,               // 1% base risk
        volatility: 3000,                   // 30% base volatility
    };
    
    msg!("Oracle config initialized");
    msg!("Primary oracle: {}", primary_oracle);
    msg!("Secondary oracle: {}", secondary_oracle);
    msg!("Update frequency: {} seconds", update_frequency);
    
    Ok(())
}

// ============================================================================
// Update Oracle Config (Admin)
// ============================================================================

#[derive(Accounts)]
pub struct UpdateOracleConfig<'info> {
    /// Pool to update oracle for
    pub pool: AccountLoader<'info, Pool>,
    
    /// Oracle configuration
    #[account(
        mut,
        seeds = [b"oracle_config", pool.key().as_ref()],
        bump,
    )]
    pub oracle_config: Account<'info, OracleConfig>,
    
    /// Pool authority
    pub authority: Signer<'info>,
}

/// Update oracle configuration (change oracles or frequency)
pub fn update_oracle_config(
    ctx: Context<UpdateOracleConfig>,
    new_primary: Option<Pubkey>,
    new_secondary: Option<Pubkey>,
    new_frequency: Option<i64>,
) -> Result<()> {
    let pool = ctx.accounts.pool.load()?;
    
    // Verify authority
    require!(
        ctx.accounts.authority.key() == pool.pool_creator,
        FeelsError::UnauthorizedError {
            action: "update_oracle_config".to_string(),
            reason: "Only pool creator can update oracle".to_string(),
        }
    );
    
    let config = &mut ctx.accounts.oracle_config;
    
    // Update fields if provided
    if let Some(primary) = new_primary {
        config.primary_oracle = primary;
        msg!("Updated primary oracle: {}", primary);
    }
    
    if let Some(secondary) = new_secondary {
        config.secondary_oracle = secondary;
        msg!("Updated secondary oracle: {}", secondary);
    }
    
    if let Some(frequency) = new_frequency {
        require!(
            frequency >= 30 && frequency <= 3600,
            FeelsError::ParameterError {
                parameter: "update_frequency".to_string(),
                reason: "Frequency must be 30-3600 seconds".to_string(),
            }
        );
        config.update_frequency = frequency;
        msg!("Updated frequency: {} seconds", frequency);
    }
    
    Ok(())
}

// ============================================================================
// Emergency Oracle Override
// ============================================================================

#[derive(Accounts)]
pub struct EmergencyOracleOverride<'info> {
    /// Pool to override
    pub pool: AccountLoader<'info, Pool>,
    
    /// Market state to update directly
    #[account(
        mut,
        seeds = [b"market_state", pool.key().as_ref()],
        bump,
    )]
    pub market_state: Account<'info, MarketState>,
    
    /// Oracle configuration
    #[account(
        mut,
        seeds = [b"oracle_config", pool.key().as_ref()],
        bump,
    )]
    pub oracle_config: Account<'info, OracleConfig>,
    
    /// Protocol authority (for emergencies only)
    pub protocol_authority: Signer<'info>,
    
    /// Protocol state to verify authority
    #[account(
        seeds = [b"protocol"],
        bump,
    )]
    pub protocol_state: Account<'info, crate::state::ProtocolState>,
}

/// Emergency override for oracle parameters
pub fn emergency_oracle_override(
    ctx: Context<EmergencyOracleOverride>,
    parameters: crate::logic::oracle::MarketParameters,
) -> Result<()> {
    // Verify protocol authority
    require!(
        ctx.accounts.protocol_authority.key() == ctx.accounts.protocol_state.authority,
        FeelsError::UnauthorizedError {
            action: "emergency_override".to_string(),
            reason: "Only protocol authority can override".to_string(),
        }
    );
    
    // Validate parameters even in emergency
    parameters.validate()?;
    
    let clock = Clock::get()?;
    
    // Create emergency update
    let update = OracleUpdate {
        pool: ctx.accounts.pool.key(),
        parameters,
        timestamp: clock.unix_timestamp,
        oracle: ctx.accounts.protocol_authority.key(),
    };
    
    // Apply without normal validation
    apply_oracle_update(
        &mut ctx.accounts.market_state,
        &mut ctx.accounts.oracle_config,
        &update,
    )?;
    
    msg!("EMERGENCY: Oracle parameters overridden");
    msg!("Authority: {}", ctx.accounts.protocol_authority.key());
    
    Ok(())
}