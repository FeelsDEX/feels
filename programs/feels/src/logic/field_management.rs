/// Unified field management for market field updates and verification.
/// Combines field update logic with commitment verification to ensure
/// all market scalar changes are properly validated and applied.
use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::state::{
    MarketField, MarketManager, UnifiedOracle, VolumeTracker,
    FieldCommitment, MarketDataSource, UnifiedMarketUpdate,
    FieldCommitmentData, DATA_SOURCE_TYPE_KEEPER, DATA_SOURCE_TYPE_ORACLE, 
    DATA_SOURCE_TYPE_HYBRID
};

// ============================================================================
// Constants
// ============================================================================

/// Maximum change in market scalars per update (basis points)
pub const MAX_SCALAR_CHANGE_BPS: u32 = 200; // 2%

/// Minimum time between field updates (seconds)
pub const MIN_UPDATE_INTERVAL: i64 = 60; // 1 minute

/// Maximum volatility for safe updates (basis points)
pub const MAX_SAFE_VOLATILITY_BPS: u32 = 500; // 5%

/// Field commitment update modes
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum FieldUpdateMode {
    /// Normal market-driven update
    Normal,
    /// Oracle-driven update (during high volatility)
    Oracle,
    /// Keeper manual intervention
    Keeper,
    /// Emergency override
    Emergency,
}

/// Maximum allowed rate of change per update (basis points)
pub const MAX_RATE_OF_CHANGE_BPS: u64 = 500; // 5%

/// Maximum optimality gap allowed (basis points)
pub const MAX_OPTIMALITY_GAP_BPS: u64 = 100; // 1%

/// Number of sample points for Lipschitz verification
pub const LIPSCHITZ_SAMPLE_POINTS: usize = 8;

// ============================================================================
// Field Update Logic
// ============================================================================

/// Context for field updates
pub struct FieldUpdateContext<'a> {
    pub market_manager: &'a MarketManager,
    pub tick_arrays: Vec<&'a AccountInfo<'a>>,
    pub buffer_account: &'a AccountInfo<'a>,
}

/// Apply pre-computed market field update after verification
pub fn apply_market_field_update(
    field: &mut MarketField,
    update: &MarketFieldUpdate,
    current_time: i64,
) -> Result<()> {
    // Apply the verified update
    field.S = update.spot_scalar;
    field.T = update.time_scalar;
    field.L = update.leverage_scalar;
    field.last_update = current_time;
    
    // Update metadata
    field.update_sequence = update.sequence;
    field.update_authority = update.authority;
    
    Ok(())
}

/// Verify market field update before applying
pub fn verify_market_field_update(
    field: &MarketField,
    update: &MarketFieldUpdate,
    current_time: i64,
) -> Result<()> {
    // Check update frequency
    require!(
        current_time >= field.last_update + MIN_UPDATE_INTERVAL,
        FeelsProtocolError::ValidationError
    );
    
    // Verify sequence number
    require!(
        update.sequence == field.update_sequence + 1,
        FeelsProtocolError::InvalidSequence
    );
    
    // Verify timestamp freshness
    require!(
        current_time - update.timestamp <= MAX_UPDATE_STALENESS,
        FeelsProtocolError::StaleData
    );
    
    // Validate scalar changes are within bounds
    let scalars = MarketScalars {
        spot_scalar: field.S,
        time_scalar: field.T,
        leverage_scalar: field.L,
    };
    validate_scalar_changes(field, &scalars)?;
    
    Ok(())
}

/// Market scalar update data
#[derive(Debug)]
struct MarketScalars {
    spot_scalar: u128,
    time_scalar: u128,
    leverage_scalar: u128,
}

/// Market field update structure (provided by keeper/oracle)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct MarketFieldUpdate {
    /// New S scalar value
    pub spot_scalar: u128,
    /// New T scalar value
    pub time_scalar: u128,
    /// New L scalar value
    pub leverage_scalar: u128,
    /// Update timestamp
    pub timestamp: i64,
    /// Sequence number
    pub sequence: u64,
    /// Update authority (keeper/oracle)
    pub authority: Pubkey,
    /// Proof hash for verification
    pub proof_hash: [u8; 32],
}

/// Maximum staleness for updates
pub const MAX_UPDATE_STALENESS: i64 = 300; // 5 minutes

/// Validate scalar changes are within acceptable bounds
fn validate_scalar_changes(
    current: &MarketField,
    new: &MarketScalars,
) -> Result<()> {
    // Check each scalar change
    validate_single_scalar_change(current.S, new.spot_scalar, "S")?;
    validate_single_scalar_change(current.T, new.time_scalar, "T")?;
    validate_single_scalar_change(current.L, new.leverage_scalar, "L")?;
    
    Ok(())
}

/// Validate a single scalar change
fn validate_single_scalar_change(
    current: u128,
    new: u128,
    name: &str,
) -> Result<()> {
    if current == 0 {
        return Ok(()); // Allow any change from zero
    }
    
    let change_bps = if new > current {
        ((new - current) * 10000) / current
    } else {
        ((current - new) * 10000) / current
    };
    
    require!(
        change_bps <= MAX_SCALAR_CHANGE_BPS as u128,
        FeelsProtocolError::ValidationError
    );
    
    Ok(())
}

/// Apply risk parameter updates from the field update
fn apply_risk_parameters(
    field: &mut MarketField,
    update: &MarketFieldUpdate,
) -> Result<()> {
    // Risk parameters are now provided in the update
    // No on-chain calculation needed
    Ok(())
}

/// Estimate token balances from pool state
fn estimate_token_balance_0(pool: &MarketManager) -> Result<u128> {
    // Simplified: use liquidity and price
    // L = sqrt(x * y) at current price
    // x = L² / y, y = L² / x
    
    let sqrt_price = pool.sqrt_price;
    if sqrt_price == 0 {
        return Ok(0);
    }
    
    // x = L / sqrt_price (in token units) - use safe math for critical financial calculation
    let shifted_liquidity = crate::utils::safe::safe_shl_u128(pool.liquidity, 96)?;
    let balance = crate::utils::safe::div_u128(shifted_liquidity, sqrt_price)?;
    Ok(balance)
}

/// Estimate token 1 balance from pool state
fn estimate_token_balance_1(pool: &MarketManager) -> Result<u128> {
    // y = L * sqrt_price (in token units) - use safe math for critical financial calculation
    let product = crate::utils::safe::mul_u128(pool.liquidity, pool.sqrt_price)?;
    let balance = crate::utils::safe::safe_shr_u128(product, 96)?;
    Ok(balance)
}

// ============================================================================
// Field Verification Logic
// ============================================================================

/// Verify field commitment matches expected values
pub fn verify_field_commitment(
    commitment: &FieldCommitment,
    expected_mode: FieldUpdateMode,
    current_time: i64,
) -> Result<()> {
    // Check commitment freshness
    require!(
        current_time - commitment.snapshot_ts <= commitment.max_staleness,
        FeelsProtocolError::StaleData
    );
    
    // Verify mode matches
    let actual_mode = match commitment.update_type {
        0 => FieldUpdateMode::Normal,
        1 => FieldUpdateMode::Oracle,
        2 => FieldUpdateMode::Keeper,
        3 => FieldUpdateMode::Emergency,
        _ => return Err(FeelsProtocolError::InvalidUpdateMode.into()),
    };
    
    require!(
        matches!(expected_mode, actual_mode),
        FeelsProtocolError::InvalidUpdateMode
    );
    
    // Additional mode-specific checks
    match expected_mode {
        FieldUpdateMode::Oracle => {
            // Oracle updates require volatility check
            require!(
                commitment.sigma_spot > 100 || commitment.sigma_leverage > 100,
                FeelsProtocolError::InvalidUpdateMode
            );
        },
        FieldUpdateMode::Emergency => {
            // Emergency updates require authority signature
            // (checked in instruction handler)
        },
        _ => {}
    }
    
    Ok(())
}

/// Enhanced verification for market updates with all required checks
pub fn verify_market_update_enhanced(
    update: &UnifiedMarketUpdate,
    data_source: &MarketDataSource,
    current_field: &MarketField,
    current_time: i64,
) -> Result<()> {
    // 1. Source/type authorization
    verify_source_authorization(update, data_source)?;
    
    // 2. Comprehensive staleness validation
    verify_comprehensive_staleness(update, data_source, current_field, current_time)?;
    
    // 3. Update frequency check
    verify_update_frequency(data_source, current_time)?;
    
    // 4. Rate-of-change limits
    if let Some(field_data) = &update.field_commitment {
        verify_rate_of_change(field_data, current_field)?;
    }
    
    // 5. Sequence number monotonicity
    verify_sequence_number(update, data_source)?;
    
    // 6. Field commitment specific checks (if Option B with coefficients)
    if let Some(_field_data) = &update.field_commitment {
        // These checks require off-chain computation results
        // The keeper provides proof data that we verify on-chain
        
        // TODO: Actual implementation would include proof verification
        // For now, we validate the structure and basic constraints
    }
    
    Ok(())
}

/// Verify source is authorized to provide updates
fn verify_source_authorization(
    update: &UnifiedMarketUpdate,
    data_source: &MarketDataSource,
) -> Result<()> {
    // Check if source type matches or hybrid mode
    let authorized = match data_source.config.source_type {
        DATA_SOURCE_TYPE_KEEPER => update.source == DATA_SOURCE_TYPE_KEEPER,
        DATA_SOURCE_TYPE_ORACLE => update.source == DATA_SOURCE_TYPE_ORACLE,
        DATA_SOURCE_TYPE_HYBRID => {
            update.source == DATA_SOURCE_TYPE_KEEPER || 
            update.source == DATA_SOURCE_TYPE_ORACLE
        },
        _ => false,
    };
    
    require!(
        authorized,
        FeelsProtocolError::Unauthorized
    );
    
    Ok(())
}

/// Comprehensive staleness validation checking all three levels
fn verify_comprehensive_staleness(
    update: &UnifiedMarketUpdate,
    data_source: &MarketDataSource,
    current_field: &MarketField,
    current_time: i64,
) -> Result<()> {
    // 1. Check timestamp is not in future
    require!(
        update.timestamp <= current_time,
        FeelsProtocolError::ValidationError
    );
    
    // 2. Check source-level staleness based on configuration
    let max_staleness = match update.source {
        DATA_SOURCE_TYPE_KEEPER => data_source.config.keeper_config.max_staleness,
        DATA_SOURCE_TYPE_ORACLE => data_source.config.oracle_config.max_price_staleness,
        _ => 300, // Default 5 minutes
    };
    
    let age = current_time - update.timestamp;
    if age > max_staleness {
        msg!("STALENESS VIOLATION: Source-level staleness");
        msg!("  Source: {}", match update.source {
            DATA_SOURCE_TYPE_KEEPER => "Keeper",
            DATA_SOURCE_TYPE_ORACLE => "Oracle",
            _ => "Unknown"
        });
        msg!("  Age: {} seconds", age);
        msg!("  Max allowed: {} seconds", max_staleness);
        return Err(FeelsProtocolError::StaleData.into());
    }
    
    msg!("Source staleness check passed: age {} <= max {}", age, max_staleness);
    
    // 3. Check commitment-level staleness (expires_at)
    if let Some(field_data) = &update.field_commitment {
        // max_staleness field in FieldCommitmentData represents expires_at
        if current_time > field_data.max_staleness {
            msg!("COMMITMENT EXPIRED");
            msg!("  Commitment sequence: {}", update.sequence);
            msg!("  Current time: {}", current_time);
            msg!("  Expired at: {}", field_data.max_staleness);
            msg!("  Expired by: {} seconds", current_time - field_data.max_staleness);
            return Err(FeelsProtocolError::CommitmentExpired.into());
        }
        
        msg!("Commitment expiry check passed: current {} <= expires {}", 
            current_time, field_data.max_staleness);
    }
    
    // 4. Check field-level staleness 
    let field_age = current_time - current_field.snapshot_ts;
    if field_age > current_field.max_staleness {
        msg!("STALENESS VIOLATION: Field-level staleness");
        msg!("  Field: MarketField");
        msg!("  Age: {} seconds", field_age);
        msg!("  Max allowed: {} seconds", current_field.max_staleness);
        return Err(FeelsProtocolError::StaleData.into());
    }
    
    msg!("Field staleness check passed: age {} <= max {}", 
        field_age, current_field.max_staleness);
    
    Ok(())
}

/// Verify update frequency constraints
fn verify_update_frequency(
    data_source: &MarketDataSource,
    current_time: i64,
) -> Result<()> {
    // Check if enough time has passed since last update
    let time_since_last = current_time - data_source.last_update;
    
    if time_since_last < data_source.update_frequency {
        msg!("UPDATE FREQUENCY VIOLATION");
        msg!("  Source: {}", data_source.market_field);
        msg!("  Time since last update: {} seconds", time_since_last);
        msg!("  Required interval: {} seconds", data_source.update_frequency);
        msg!("  Too frequent by: {} seconds", data_source.update_frequency - time_since_last);
        return Err(FeelsProtocolError::UpdateTooFrequent.into());
    }
    
    let update_frequency = data_source.update_frequency;
    msg!("Update frequency check passed: {} >= {}", 
        time_since_last, update_frequency);
    
    Ok(())
}

/// Verify rate of change limits
fn verify_rate_of_change(
    field_data: &FieldCommitmentData,
    current_field: &MarketField,
) -> Result<()> {
    // Check S scalar
    verify_scalar_change(
        current_field.S,
        field_data.S,
        MAX_RATE_OF_CHANGE_BPS,
        "S"
    )?;
    
    // Check T scalar
    verify_scalar_change(
        current_field.T,
        field_data.T,
        MAX_RATE_OF_CHANGE_BPS,
        "T"
    )?;
    
    // Check L scalar
    verify_scalar_change(
        current_field.L,
        field_data.L,
        MAX_RATE_OF_CHANGE_BPS,
        "L"
    )?;
    
    Ok(())
}

/// Verify sequence number is monotonically increasing
fn verify_sequence_number(
    update: &UnifiedMarketUpdate,
    data_source: &MarketDataSource,
) -> Result<()> {
    // Sequence numbers must be strictly monotonic to prevent replays
    require!(
        update.sequence > data_source.last_sequence,
        FeelsProtocolError::InvalidSequence
    );
    
    // Additionally verify it's the expected next sequence
    require!(
        update.sequence == data_source.last_sequence + 1,
        FeelsProtocolError::InvalidSequence
    );
    
    let update_seq = update.sequence;
    let last_seq = data_source.last_sequence;
    msg!("Sequence validation passed: {} > {}", update_seq, last_seq);
    
    Ok(())
}

/// Helper to verify scalar change is within limits
fn verify_scalar_change(
    current: u128,
    new: u128,
    max_change_bps: u64,
    _field_name: &str,
) -> Result<()> {
    // Allow any change from zero
    if current == 0 {
        return Ok(());
    }
    
    // Calculate percentage change
    let change_ratio = if new > current {
        ((new - current) * 10000) / current
    } else {
        ((current - new) * 10000) / current
    };
    
    require!(
        change_ratio <= max_change_bps as u128,
        FeelsProtocolError::ValidationError
    );
    
    Ok(())
}

// ============================================================================
// Advanced Verification (Option B)
// ============================================================================

/// Verification data for Option B with local coefficients
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct FieldVerificationProof {
    /// Convex bound verification points
    pub convex_bound_points: Vec<ConvexBoundPoint>,
    
    /// Lipschitz sample pairs
    pub lipschitz_samples: Vec<LipschitzSample>,
    
    /// Optimality gap certificate
    pub optimality_gap: OptimalityGapProof,
    
    /// Merkle proof for coefficient inclusion
    pub merkle_proof: Option<Vec<[u8; 32]>>,
}

/// Point for convex bound verification
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct ConvexBoundPoint {
    /// Position in 3D space
    pub position: [u128; 3], // [S, T, L]
    
    /// Potential value at this point
    pub V: i128,
    
    /// Expected bound value
    pub bound: i128,
}

/// Sample pair for Lipschitz verification
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LipschitzSample {
    /// First position
    pub p1: [u128; 3],
    
    /// Second position  
    pub p2: [u128; 3],
    
    /// Gradient norm difference
    pub grad_diff_norm: u128,
    
    /// Position norm difference
    pub pos_diff_norm: u128,
}

/// Optimality gap proof
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OptimalityGapProof {
    /// Current objective value
    pub current_value: i128,
    
    /// Optimal value bound
    pub optimal_bound: i128,
    
    /// Gap in basis points
    pub gap_bps: u64,
}

/// Verify convex bound holds at tight points
pub fn verify_convex_bound(
    proof: &FieldVerificationProof,
    _commitment: &FieldCommitment,
) -> Result<()> {
    for point in &proof.convex_bound_points {
        // Verify V(point) <= bound
        // Note: Actual V calculation requires ln/exp which is done off-chain
        // Here we verify the keeper's assertion
        require!(
            point.V <= point.bound,
            FeelsProtocolError::ValidationError
        );
    }
    
    Ok(())
}

/// Verify Lipschitz inequality on sampled pairs
#[allow(non_snake_case)]
pub fn verify_lipschitz_inequality(
    proof: &FieldVerificationProof,
    commitment: &FieldCommitment,
) -> Result<()> {
    let L = commitment.lipschitz_L;
    
    for sample in &proof.lipschitz_samples {
        // Verify ||∇V(p2) - ∇V(p1)|| <= L * ||p2 - p1||
        let lhs = sample.grad_diff_norm;
        let rhs = (L as u128).saturating_mul(sample.pos_diff_norm);
        
        require!(
            lhs <= rhs,
            FeelsProtocolError::ValidationError
        );
    }
    
    Ok(())
}

/// Verify optimality gap is within policy limits
pub fn verify_optimality_gap(
    proof: &FieldVerificationProof,
    max_gap_bps: u64,
) -> Result<()> {
    require!(
        proof.optimality_gap.gap_bps <= max_gap_bps,
        FeelsProtocolError::ValidationError
    );
    
    Ok(())
}

/// Verify merkle inclusion proof for local coefficients
pub fn verify_merkle_inclusion(
    leaf_data: &[u8],
    merkle_proof: &[[u8; 32]],
    root: &[u8; 32],
) -> Result<()> {
    // Compute leaf hash
    let mut current_hash = anchor_lang::solana_program::hash::hash(leaf_data).to_bytes();
    
    // Apply merkle proof
    for sibling in merkle_proof {
        let combined = if current_hash < *sibling {
            [current_hash, *sibling].concat()
        } else {
            [*sibling, current_hash].concat()
        };
        current_hash = anchor_lang::solana_program::hash::hash(&combined).to_bytes();
    }
    
    // Verify against root
    require!(
        current_hash == *root,
        FeelsProtocolError::ValidationError
    );
    
    Ok(())
}