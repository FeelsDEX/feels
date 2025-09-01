/// On-chain optimality verification for keeper gradient updates.
/// Cheaply verifies keeper solutions are near-optimal using convex bounds.
use anchor_lang::prelude::*;
use crate::state::{Pool, MarketState, GradientCache};
use crate::logic::keepers::keeper_gradient::{
    GradientUpdate, verify_optimality_certificate, MAX_OPTIMALITY_GAP_BPS
};
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Maximum points to verify on-chain
pub const MAX_VERIFICATION_POINTS: usize = 5;

/// Maximum Lipschitz samples to check
pub const MAX_LIPSCHITZ_SAMPLES: usize = 3;

// ============================================================================
// Verify Market Update
// ============================================================================

#[derive(Accounts)]
pub struct VerifyMarketUpdate<'info> {
    /// Pool being updated
    pub pool: AccountLoader<'info, Pool>,
    
    /// Market state for verification
    #[account(
        seeds = [b"market_state", pool.key().as_ref()],
        bump,
    )]
    pub market_state: AccountLoader<'info, MarketState>,
    
    /// Gradient cache to update
    #[account(
        mut,
        seeds = [b"gradient_cache", pool.key().as_ref()],
        bump,
    )]
    pub gradient_cache: AccountLoader<'info, GradientCache>,
    
    /// Keeper submitting update
    pub keeper: Signer<'info>,
    
    /// Clock for timestamp verification
    pub clock: Sysvar<'info, Clock>,
}

/// Verify and apply market update from keeper
pub fn verify_market_update(
    ctx: Context<VerifyMarketUpdate>,
    update: MarketUpdate,
) -> Result<()> {
    let gradient_cache = &mut ctx.accounts.gradient_cache.load_mut()?;
    let market_state = &ctx.accounts.market_state.load()?;
    let current_time = ctx.accounts.clock.unix_timestamp;
    
    // 1. Basic validation
    validate_update_basic(&update, current_time)?;
    
    // 2. Spot-check convex bounds
    verify_convex_bounds(&update, market_state)?;
    
    // 3. Verify Lipschitz continuity
    verify_lipschitz_continuity(&update)?;
    
    // 4. Check optimality gap
    require!(
        update.certificate.gap_bps <= MAX_OPTIMALITY_GAP_BPS,
        FeelsProtocolError::ExcessiveOptimalityGap
    );
    
    // 5. Apply update to cache
    apply_verified_update(gradient_cache, &update)?;
    
    emit!(MarketUpdateVerifiedEvent {
        pool: ctx.accounts.pool.key(),
        keeper: ctx.accounts.keeper.key(),
        gap_bps: update.certificate.gap_bps,
        timestamp: current_time,
    });
    
    Ok(())
}

// ============================================================================
// Verification Functions
// ============================================================================

/// Basic validation of update
fn validate_update_basic(update: &MarketUpdate, current_time: i64) -> Result<()> {
    // Check timestamp freshness
    require!(
        current_time - update.timestamp <= 60, // 1 minute
        FeelsProtocolError::StaleGradientUpdate
    );
    
    // Check has gradients
    require!(
        !update.gradients.is_empty(),
        FeelsProtocolError::NoActiveTicks
    );
    
    // Check certificate fields
    require!(
        update.certificate.solution_value > 0,
        FeelsProtocolError::InvalidOptimalityBound
    );
    
    Ok(())
}

/// Verify convex bounds are valid
fn verify_convex_bounds(
    update: &MarketUpdate,
    market_state: &MarketState,
) -> Result<()> {
    // Get tight points from certificate
    let tight_points = &update.certificate.proof.tight_points;
    require!(
        !tight_points.is_empty(),
        FeelsProtocolError::NoTightPoints
    );
    
    // Verify up to MAX_VERIFICATION_POINTS
    for point in tight_points.iter().take(MAX_VERIFICATION_POINTS) {
        // Calculate actual potential at point
        let actual_v = calculate_potential_at_point(market_state, point)?;
        
        // Evaluate convex bound
        let convex_v = update.certificate.lower_bound.evaluate(point)?;
        
        // Convex bound must be <= actual value
        require!(
            convex_v <= actual_v,
            FeelsProtocolError::InvalidOptimalityBound
        );
        
        // Check tightness (within 1%)
        let gap = ((actual_v - convex_v) * 10000) / actual_v;
        require!(
            gap <= 100, // 1%
            FeelsProtocolError::InvalidOptimalityBound
        );
    }
    
    Ok(())
}

/// Verify Lipschitz continuity of gradients
fn verify_lipschitz_continuity(update: &MarketUpdate) -> Result<()> {
    let samples = &update.certificate.proof.lipschitz_samples;
    let lipschitz_constant = update.lipschitz_constant;
    
    // Check up to MAX_LIPSCHITZ_SAMPLES
    for (p1, p2) in samples.iter().take(MAX_LIPSCHITZ_SAMPLES) {
        // Get gradients at these points
        let g1 = get_gradient_at_point(&update.gradients, p1)?;
        let g2 = get_gradient_at_point(&update.gradients, p2)?;
        
        // Calculate gradient difference norm
        let grad_diff_norm = calculate_gradient_diff_norm(&g1, &g2)?;
        
        // Calculate position difference norm
        let pos_diff_norm = calculate_position_diff_norm(p1, p2)?;
        
        // Verify Lipschitz condition: ||∇f(x) - ∇f(y)|| <= L * ||x - y||
        let bound = (lipschitz_constant as u128 * pos_diff_norm) >> 32;
        require!(
            grad_diff_norm <= bound,
            FeelsProtocolError::InvalidGradient
        );
    }
    
    Ok(())
}

/// Calculate potential at a specific point
fn calculate_potential_at_point(
    market_state: &MarketState,
    point: &VerificationPoint,
) -> Result<i64> {
    // V = -Σ ŵᵢ ln(xᵢ)
    // Simplified calculation for verification
    
    let w_hat = market_state.get_weights().get_hat_weights();
    
    // Use logarithm approximation for efficiency
    let ln_s = ln_approximation(point.s)?;
    let ln_t = ln_approximation(point.t)?;
    let ln_l = ln_approximation(point.l)?;
    
    let v = -((w_hat.0 as i64 * ln_s) / 10000 +
              (w_hat.1 as i64 * ln_t) / 10000 +
              (w_hat.2 as i64 * ln_l) / 10000);
    
    Ok(v)
}

/// Logarithm approximation for on-chain efficiency
fn ln_approximation(x: u64) -> Result<i64> {
    // Simple linear approximation around x = 1
    // ln(x) ≈ (x - 1) for x near 1
    // Scale by 2^32 for precision
    
    require!(x > 0, FeelsProtocolError::DivisionByZero);
    
    let scale = 1u64 << 32;
    if x >= scale / 2 && x <= scale * 2 {
        // Near 1, use linear approximation
        Ok((x as i64 - scale as i64))
    } else {
        // Far from 1, use table lookup or other approximation
        // Simplified: return rough estimate
        if x > scale {
            Ok((x / scale) as i64 * scale as i64)
        } else {
            Ok(-(scale / x) as i64 * scale as i64)
        }
    }
}

/// Get gradient at specific point from update
fn get_gradient_at_point(
    gradients: &[GradientEntry],
    point: &VerificationPoint,
) -> Result<VerifiedGradient> {
    // Find nearest gradient in update
    // TODO: In production, would interpolate
    
    let nearest = gradients
        .iter()
        .min_by_key(|g| {
            let ds = (g.position.s as i64 - point.s as i64).abs();
            let dt = (g.position.t as i64 - point.t as i64).abs();
            let dl = (g.position.l as i64 - point.l as i64).abs();
            (ds + dt + dl) as u64
        })
        .ok_or(FeelsProtocolError::NoActiveTicks)?;
    
    Ok(VerifiedGradient {
        dv_ds: nearest.gradient.dv_ds,
        dv_dt: nearest.gradient.dv_dt,
        dv_dl: nearest.gradient.dv_dl,
    })
}

/// Calculate norm of gradient difference
fn calculate_gradient_diff_norm(g1: &VerifiedGradient, g2: &VerifiedGradient) -> Result<u128> {
    let ds = (g1.dv_ds as i128 - g2.dv_ds as i128).abs() as u128;
    let dt = (g1.dv_dt as i128 - g2.dv_dt as i128).abs() as u128;
    let dl = (g1.dv_dl as i128 - g2.dv_dl as i128).abs() as u128;
    
    // L2 norm squared, scaled down
    Ok((ds * ds + dt * dt + dl * dl) >> 64)
}

/// Calculate norm of position difference
fn calculate_position_diff_norm(p1: &VerificationPoint, p2: &VerificationPoint) -> Result<u128> {
    let ds = (p1.s as i128 - p2.s as i128).abs() as u128;
    let dt = (p1.t as i128 - p2.t as i128).abs() as u128;
    let dl = (p1.l as i128 - p2.l as i128).abs() as u128;
    
    // L2 norm squared, scaled down
    Ok((ds * ds + dt * dt + dl * dl) >> 64)
}

/// Apply verified update to gradient cache
fn apply_verified_update(
    cache: &mut GradientCache,
    update: &MarketUpdate,
) -> Result<()> {
    // Clear old data
    cache.clear_gradients();
    
    // Apply new gradients
    for entry in &update.gradients {
        if entry.tick_index < crate::constant::MAX_TICKS as u32 {
            cache.set_gradient(entry.tick_index, &entry.gradient.to_gradient_3d())?;
        }
    }
    
    // Update metadata
    cache.last_update = update.timestamp;
    cache.keeper = update.keeper;
    
    // Update certificate
    cache.certificate = crate::state::gradient_cache::OptimalityCertificate {
        convex_lower_bound: update.certificate.lower_bound.constant as u128,
        solution_upper_bound: update.certificate.solution_value as u128,
        gap_bps: update.certificate.gap_bps as u16,
        lipschitz_constant: update.lipschitz_constant,
        verification_point_1: update.certificate.proof.tight_points
            .get(0)
            .map(|p| p.s as i32)
            .unwrap_or(0),
        verification_point_2: update.certificate.proof.tight_points
            .get(1)
            .map(|p| p.s as i32)
            .unwrap_or(0),
        timestamp: update.timestamp,
        _reserved: [0; 6],
    };
    
    Ok(())
}

// ============================================================================
// Data Structures
// ============================================================================

/// Market update from keeper
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct MarketUpdate {
    /// Keeper submitting update
    pub keeper: Pubkey,
    
    /// Gradients for active ticks
    pub gradients: Vec<GradientEntry>,
    
    /// Optimality certificate
    pub certificate: UpdateCertificate,
    
    /// Lipschitz constant
    pub lipschitz_constant: u64,
    
    /// Update timestamp
    pub timestamp: i64,
}

/// Gradient entry in update
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct GradientEntry {
    /// Tick index
    pub tick_index: u32,
    
    /// Position in 3D space
    pub position: VerificationPoint,
    
    /// Gradient at this position
    pub gradient: VerifiedGradient,
}

/// Verification point in 3D space
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct VerificationPoint {
    pub s: u64,
    pub t: u64,
    pub l: u64,
}

/// Verified gradient values
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct VerifiedGradient {
    pub dv_ds: u128,
    pub dv_dt: u128,
    pub dv_dl: u128,
}

impl VerifiedGradient {
    /// Convert to on-chain Gradient3D format
    pub fn to_gradient_3d(&self) -> crate::state::gradient_cache::Gradient3D {
        crate::state::gradient_cache::Gradient3D {
            dV_dS: self.dv_ds,
            dV_dT: self.dv_dt,
            dV_dL: self.dv_dl,
        }
    }
}

/// Update certificate with proof
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UpdateCertificate {
    /// Lower bound from convex relaxation
    pub lower_bound: ConvexBound,
    
    /// Solution value
    pub solution_value: u64,
    
    /// Optimality gap in basis points
    pub gap_bps: u32,
    
    /// Proof of validity
    pub proof: VerificationProof,
}

/// Convex bound for verification
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ConvexBound {
    /// Linear coefficients
    pub linear_coeffs: [i64; 3],
    
    /// Constant term
    pub constant: i64,
}

impl ConvexBound {
    /// Evaluate bound at point
    pub fn evaluate(&self, point: &VerificationPoint) -> Result<i64> {
        let value = self.constant
            + (self.linear_coeffs[0] * point.s as i64) / (1 << 32)
            + (self.linear_coeffs[1] * point.t as i64) / (1 << 32)
            + (self.linear_coeffs[2] * point.l as i64) / (1 << 32);
        
        Ok(value)
    }
}

/// Proof for verification
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct VerificationProof {
    /// Points where bound is tight
    pub tight_points: Vec<VerificationPoint>,
    
    /// Sample points for Lipschitz verification
    pub lipschitz_samples: Vec<(VerificationPoint, VerificationPoint)>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct MarketUpdateVerifiedEvent {
    pub pool: Pubkey,
    pub keeper: Pubkey,
    pub gap_bps: u32,
    pub timestamp: i64,
}