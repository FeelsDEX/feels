/// Conservation law verification for the market physics model.
/// Ensures all rebases preserve the weighted log-sum constraint to prevent
/// value creation or destruction: Σ w_i * ln(g_i) = 0
use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Fixed-point scale for weights (basis points)
pub const WEIGHT_SCALE: i128 = 10_000;

/// Fixed-point scale for logarithms (Q64)
pub const LN_SCALE: i128 = 1 << 64;

/// Conservation tolerance (1e-9 in fixed point)
pub const CONSERVATION_TOLERANCE: i128 = 18; // ~1e-9 when divided by LN_SCALE

// ============================================================================
// Conservation Verification
// ============================================================================

/// Verify conservation law for N growth factors
/// Checks that Σ w_i * ln(g_i) = 0 within tolerance
pub fn verify_conservation<const N: usize>(
    weights: &[u64; N],
    growth_factors: &[u128; N],
) -> Result<()> {
    // Calculate weighted log sum
    let mut log_sum: i128 = 0;
    
    for i in 0..N {
        // Skip zero weights
        if weights[i] == 0 {
            continue;
        }
        
        // Calculate ln(g_i) in fixed point
        let ln_g = calculate_ln_fixed_point(growth_factors[i])?;
        
        // Apply weight: w_i * ln(g_i)
        let weighted = (weights[i] as i128)
            .checked_mul(ln_g)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(WEIGHT_SCALE)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
        
        log_sum = log_sum
            .checked_add(weighted)
            .ok_or(FeelsProtocolError::MathOverflow)?;
    }
    
    // Check conservation within tolerance
    require!(
        log_sum.abs() < CONSERVATION_TOLERANCE,
        FeelsProtocolError::ConservationViolation
    );
    
    Ok(())
}

/// Verify conservation for sub-domain rebasing
/// Used when only a subset of domains participates
pub fn verify_subdomain_conservation(
    participant_weights: &[(usize, u64)], // (domain_index, weight)
    growth_factors: &[u128],              // Growth factors for each participant
) -> Result<()> {
    require!(
        participant_weights.len() == growth_factors.len(),
        FeelsProtocolError::InvalidInput
    );
    
    let mut log_sum: i128 = 0;
    
    for (i, (_, weight)) in participant_weights.iter().enumerate() {
        if *weight == 0 {
            continue;
        }
        
        let ln_g = calculate_ln_fixed_point(growth_factors[i])?;
        
        let weighted = (*weight as i128)
            .checked_mul(ln_g)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(WEIGHT_SCALE)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
        
        log_sum = log_sum
            .checked_add(weighted)
            .ok_or(FeelsProtocolError::MathOverflow)?;
    }
    
    require!(
        log_sum.abs() < CONSERVATION_TOLERANCE,
        FeelsProtocolError::ConservationViolation
    );
    
    Ok(())
}

// ============================================================================
// Conservation Solving
// ============================================================================

/// Given N-1 growth factors and weights, solve for the Nth factor
/// that preserves conservation: g_N = exp(-Σ w_i ln(g_i) / w_N)
pub fn solve_conservation_factor(
    weights: &[u64],        // All N weights
    growth_factors: &[u128], // First N-1 factors
    target_index: usize,    // Index to solve for
) -> Result<u128> {
    require!(
        weights.len() > target_index,
        FeelsProtocolError::InvalidInput
    );
    require!(
        growth_factors.len() == weights.len() - 1,
        FeelsProtocolError::InvalidInput
    );
    require!(
        weights[target_index] > 0,
        FeelsProtocolError::DivisionByZero
    );
    
    // Calculate sum of weighted logs for known factors
    let mut weighted_log_sum: i128 = 0;
    let mut factor_idx = 0;
    
    for i in 0..weights.len() {
        if i == target_index {
            continue;
        }
        
        if weights[i] == 0 {
            factor_idx += 1;
            continue;
        }
        
        let ln_g = calculate_ln_fixed_point(growth_factors[factor_idx])?;
        
        let weighted = (weights[i] as i128)
            .checked_mul(ln_g)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(WEIGHT_SCALE)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
        
        weighted_log_sum = weighted_log_sum
            .checked_add(weighted)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        factor_idx += 1;
    }
    
    // Solve for target: ln(g_N) = -weighted_log_sum / w_N
    let target_ln = weighted_log_sum
        .checked_neg()
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_mul(WEIGHT_SCALE)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(weights[target_index] as i128)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    // Convert back to growth factor
    calculate_exp_fixed_point(target_ln)
}

// ============================================================================
// Fixed-Point Math Helpers
// ============================================================================

/// Calculate natural logarithm of a Q64 fixed-point number
/// Returns ln(x) in Q64 format
fn calculate_ln_fixed_point(x: u128) -> Result<i128> {
    // x is in Q64 format (1.0 = 2^64)
    const Q64: u128 = 1 << 64;
    
    require!(x > 0, FeelsProtocolError::InvalidInput);
    
    // Handle x = 1 (ln(1) = 0)
    if x == Q64 {
        return Ok(0);
    }
    
    // Use Taylor series approximation around x = 1
    // ln(x) ≈ (x-1) - (x-1)²/2 + (x-1)³/3 - ...
    
    // Convert to signed and calculate x - 1
    let x_minus_1 = if x > Q64 {
        (x - Q64) as i128
    } else {
        -((Q64 - x) as i128)
    };
    
    // First term: (x-1)
    let mut result = x_minus_1;
    
    // Second term: -(x-1)²/2
    let x_minus_1_sq = x_minus_1
        .checked_mul(x_minus_1)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(Q64 as i128)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    result = result
        .checked_sub(x_minus_1_sq / 2)
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Third term: +(x-1)³/3 (only if x is close to 1)
    if x_minus_1.abs() < (Q64 as i128 / 10) {
        let x_minus_1_cubed = x_minus_1_sq
            .checked_mul(x_minus_1)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(Q64 as i128)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
        
        result = result
            .checked_add(x_minus_1_cubed / 3)
            .ok_or(FeelsProtocolError::MathOverflow)?;
    }
    
    Ok(result)
}

/// Calculate exponential of a Q64 fixed-point number
/// Returns e^x in Q64 format
fn calculate_exp_fixed_point(x: i128) -> Result<u128> {
    const Q64: i128 = 1 << 64;
    
    // Handle x = 0 (e^0 = 1)
    if x == 0 {
        return Ok(Q64 as u128);
    }
    
    // Use Taylor series: e^x = 1 + x + x²/2! + x³/3! + ...
    
    // Start with 1
    let mut result = Q64;
    
    // Add x
    result = result
        .checked_add(x)
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Add x²/2
    let x_squared = x
        .checked_mul(x)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(Q64)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    result = result
        .checked_add(x_squared / 2)
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Add x³/6 (only for small x)
    if x.abs() < Q64 / 10 {
        let x_cubed = x_squared
            .checked_mul(x)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(Q64)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
        
        result = result
            .checked_add(x_cubed / 6)
            .ok_or(FeelsProtocolError::MathOverflow)?;
    }
    
    // Result must be positive
    require!(result > 0, FeelsProtocolError::InvalidState);
    
    Ok(result as u128)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_exact_conservation() {
        // Test exact conservation: w1*ln(g1) + w2*ln(g2) = 0
        // If w1 = w2 = 5000, then g1 * g2 = 1
        let weights = [5000u64, 5000u64];
        let g1 = (1u128 << 64) + (1u128 << 60); // ~1.0625
        let g2 = ((1u128 << 64) * (1u128 << 64)) / g1; // 1/g1
        
        let growth_factors = [g1, g2];
        
        assert!(verify_conservation(&weights, &growth_factors).is_ok());
    }
    
    #[test]
    fn test_conservation_solver() {
        // Test solving for third factor
        let weights = [3000u64, 3000u64, 4000u64];
        let g1 = (1u128 << 64) + (1u128 << 62); // ~1.25
        let g2 = (1u128 << 64) - (1u128 << 62); // ~0.75
        
        let growth_factors = [g1, g2];
        
        // Solve for g3
        let g3 = solve_conservation_factor(&weights, &growth_factors, 2).unwrap();
        
        // Verify conservation with all three
        let all_factors = [g1, g2, g3];
        assert!(verify_conservation(&weights, &all_factors).is_ok());
    }
}