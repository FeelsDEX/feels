use solana_sdk::pubkey::Pubkey;
use serde::{Serialize, Deserialize};

// Use shared types instead of duplicating them
use feels_types::{
    MarketState, FieldCommitmentData, FeelsResult, FeelsProtocolError,
    Position3D, Gradient3D, Hessian3D,
};
use feels_math::{safe_add_u128, safe_mul_u128, safe_div_u128};

use crate::hysteresis_controller::StressComponents;

/// Field computation engine for calculating 3D gradients and Hessian matrices
pub struct FieldComputer {
    /// Cache for recently computed fields
    field_cache: std::collections::HashMap<Pubkey, CachedField>,
}

// Types are now imported from feels_types - removed duplicate definitions

/// Cached field computation result
#[derive(Debug, Clone)]
struct CachedField {
    field_data: FieldCommitmentData,
    computed_at: i64,
    market_state_hash: u64,
}

// 3D types now imported from feels_types

impl FieldComputer {
    /// Create new field computation engine
    pub fn new() -> Self {
        Self {
            field_cache: std::collections::HashMap::new(),
        }
    }

    /// Compute stress components from market state
    pub fn compute_stress_components(&self, market_state: &MarketState) -> FeelsResult<StressComponents> {
        // Spot stress: |price - twap| / twap × 10000
        let spot_stress = self.compute_spot_stress(market_state)?;
        
        // Time stress: utilization × 10000
        let time_stress = self.compute_time_stress(market_state)?;
        
        // Leverage stress: |L_long - L_short| / (L_long + L_short) × 10000
        let leverage_stress = self.compute_leverage_stress(market_state)?;
        
        Ok(StressComponents {
            spot_stress,
            time_stress,
            leverage_stress,
        })
    }

    /// Compute field commitment from current market state
    pub fn compute_field_commitment(&mut self, market_state: &MarketState) -> FeelsResult<FieldCommitmentData> {
        // Check cache first
        let state_hash = self.hash_market_state(market_state);
        if let Some(cached) = self.field_cache.get(&market_state.market_pubkey) {
            if cached.market_state_hash == state_hash && 
               (market_state.last_update_ts - cached.computed_at) < 300 { // 5 min cache
                return Ok(cached.field_data.clone());
            }
        }

        log::debug!("Computing fresh field commitment for {}", market_state.market_pubkey);

        // Convert market state to 3D position
        let current_position = self.market_state_to_position(market_state)?;

        // Compute 3D potential field scalars
        let field_scalars = self.compute_field_scalars(&current_position, market_state)?;

        // Compute optimal domain weights
        let domain_weights = self.compute_domain_weights(&current_position, market_state)?;

        // Compute spot value weights
        let spot_weights = self.compute_spot_weights(market_state)?;

        // Compute risk parameters
        let risk_params = self.compute_risk_parameters(market_state)?;

        // Get base fee from hysteresis controller (passed in from caller)
        // For now, use a default value - will be set by the keeper main loop
        let base_fee_bps = market_state.base_fee_bps.unwrap_or(25);
        
        // Build field commitment
        let field_commitment = FieldCommitmentData {
            S: field_scalars.S,
            T: field_scalars.T, 
            L: field_scalars.L,
            w_s: domain_weights.w_s,
            w_t: domain_weights.w_t,
            w_l: domain_weights.w_l,
            w_tau: domain_weights.w_tau,
            omega_0: spot_weights.omega_0,
            omega_1: spot_weights.omega_1,
            sigma_price: risk_params.sigma_price,
            sigma_rate: risk_params.sigma_rate,
            sigma_leverage: risk_params.sigma_leverage,
            twap_0: market_state.twap_0,
            twap_1: market_state.twap_1,
            snapshot_ts: chrono::Utc::now().timestamp(),
            max_staleness: 1800, // 30 minutes
            sequence: self.get_next_sequence(&market_state.market_pubkey),
            base_fee_bps,
            local_coefficients: None,
            commitment_hash: None,
            lipschitz_L: None,
            gap_bps: None,
        };

        // Cache result
        self.field_cache.insert(market_state.market_pubkey, CachedField {
            field_data: field_commitment.clone(),
            computed_at: market_state.last_update_ts,
            market_state_hash: state_hash,
        });

        Ok(field_commitment)
    }

    /// Compute 3D gradient at position
    pub fn compute_gradient(&self, position: &Position3D, market_state: &MarketState) -> FeelsResult<Gradient3D> {
        // 3D potential function: V(S,T,L) = w_s*f_s(S) + w_t*f_t(T) + w_l*f_l(L) + cross terms
        
        // Spot dimension gradient: ∂V/∂S
        let ds = self.compute_spot_gradient(position.x, market_state)?;
        
        // Time dimension gradient: ∂V/∂T  
        let dt = self.compute_time_gradient(position.y, market_state)?;
        
        // Leverage dimension gradient: ∂V/∂L
        let dl = self.compute_leverage_gradient(position.z, market_state)?;

        Ok(Gradient3D::new(ds, dt, dl))
    }

    /// Compute 3D Hessian matrix at position
    pub fn compute_hessian(&self, position: &Position3D, market_state: &MarketState) -> FeelsResult<Hessian3D> {
        // Second derivatives of potential function
        
        let d2S = self.compute_spot_hessian(position.x, market_state)?;
        let d2T = self.compute_time_hessian(position.y, market_state)?;  
        let d2L = self.compute_leverage_hessian(position.z, market_state)?;
        
        // Cross derivatives (coupling terms)
        let dSdT = self.compute_spot_time_cross(position, market_state)?;
        let dSdL = self.compute_spot_leverage_cross(position, market_state)?;
        let dTdL = self.compute_time_leverage_cross(position, market_state)?;

        Ok(Hessian3D::new(d2S, d2T, d2L, dSdT, dSdL, dTdL))
    }

    // Private implementation methods

    /// Convert market state to 3D position
    fn market_state_to_position(&self, market_state: &MarketState) -> FeelsResult<Position3D> {
        // Convert sqrt price to spot coordinate (log scale)
        let s = (market_state.current_sqrt_price as f64).ln();
        
        // Time coordinate based on recent activity (simplified)
        let t = 1.0; // Would compute from duration metrics
        
        // Leverage coordinate based on liquidity concentration  
        let liquidity_ratio = (market_state.liquidity as f64) / (1e18); // Normalize
        let l = liquidity_ratio.max(0.01).min(100.0).ln_1p();

        Ok(Position3D::new(s, t, l))
    }

    /// Compute field scalars from 3D analysis
    fn compute_field_scalars(&self, position: &Position3D, market_state: &MarketState) -> FeelsResult<FieldScalars> {
        // Compute scalars using eigenvalue decomposition of local Hessian
        let hessian = self.compute_hessian(position, market_state)?;
        
        // Eigenvalues represent field curvature in each dimension
        let eigenvalues = self.compute_eigenvalues(&hessian)?;
        
        // Scale eigenvalues to get field scalars
        let S = self.eigenvalue_to_scalar(eigenvalues.0)?;
        let T = self.eigenvalue_to_scalar(eigenvalues.1)?;
        let L = self.eigenvalue_to_scalar(eigenvalues.2)?;

        Ok(FieldScalars { S, T, L })
    }

    /// Compute optimal domain weights
    fn compute_domain_weights(&self, position: &Position3D, market_state: &MarketState) -> FeelsResult<DomainWeights> {
        // Compute gradient magnitude in each dimension
        let gradient = self.compute_gradient(position, market_state)?;
        
        let grad_s_mag = gradient.ds.abs();
        let grad_t_mag = gradient.dt.abs(); 
        let grad_l_mag = gradient.dl.abs();
        
        // Total gradient magnitude
        let total_mag = grad_s_mag + grad_t_mag + grad_l_mag;
        
        if total_mag == 0.0 {
            // Equal weights if no gradient
            return Ok(DomainWeights {
                w_s: 3333,
                w_t: 3333,
                w_l: 3333,
                w_tau: 1, // Minimal tau weight
            });
        }
        
        // Weight proportional to gradient magnitude
        let w_s = ((grad_s_mag / total_mag) * 9900.0) as u32;
        let w_t = ((grad_t_mag / total_mag) * 9900.0) as u32;
        let w_l = ((grad_l_mag / total_mag) * 9900.0) as u32;
        
        // Ensure weights sum to 10000
        let w_tau = 10000u32.saturating_sub(w_s + w_t + w_l);

        Ok(DomainWeights { w_s, w_t, w_l, w_tau })
    }

    /// Compute spot value weights
    fn compute_spot_weights(&self, market_state: &MarketState) -> FeelsResult<SpotWeights> {
        // Simple geometric mean weighting based on TWAPs
        let total_value = market_state.twap_0 + market_state.twap_1;
        
        if total_value == 0 {
            return Ok(SpotWeights { omega_0: 5000, omega_1: 5000 });
        }
        
        let omega_0 = ((market_state.twap_0 * 10000) / total_value) as u32;
        let omega_1 = 10000u32.saturating_sub(omega_0);

        Ok(SpotWeights { omega_0, omega_1 })
    }

    /// Compute risk parameters  
    fn compute_risk_parameters(&self, market_state: &MarketState) -> FeelsResult<RiskParams> {
        // Compute volatility estimates from recent price movement
        // Simplified - would use rolling statistics
        
        let base_volatility = 0.01; // 1% base vol
        
        // Price volatility based on current liquidity
        let liquidity_factor = (market_state.liquidity as f64) / 1e18;
        let sigma_price = base_volatility / liquidity_factor.max(0.001);
        
        // Rate and leverage volatilities (simplified)
        let sigma_rate = sigma_price * 0.5;
        let sigma_leverage = sigma_price * 2.0;

        Ok(RiskParams {
            sigma_price: (sigma_price * (1u64 << 32) as f64) as u64,
            sigma_rate: (sigma_rate * (1u64 << 32) as f64) as u64,
            sigma_leverage: (sigma_leverage * (1u64 << 32) as f64) as u64,
        })
    }

    // Gradient computation methods

    fn compute_spot_gradient(&self, s: f64, market_state: &MarketState) -> FeelsResult<f64> {
        // ∂V/∂S = w_s * ∂f_s/∂S where f_s is spot potential function
        // Use concentrated liquidity curve derivative
        let liquidity = (market_state.liquidity as f64) / 1e18;
        let gradient = liquidity / (s + 1.0); // 1/x-like behavior
        Ok(gradient)
    }

    fn compute_time_gradient(&self, t: f64, _market_state: &MarketState) -> FeelsResult<f64> {
        // ∂V/∂T = w_t * ∂f_t/∂T where f_t is time potential
        // Use exponential time decay
        let gradient = -t * 0.01; // Decay with time
        Ok(gradient)
    }

    fn compute_leverage_gradient(&self, l: f64, _market_state: &MarketState) -> FeelsResult<f64> {
        // ∂V/∂L = w_l * ∂f_l/∂L where f_l is leverage potential  
        // Quadratic penalty for high leverage
        let gradient = l * 0.1;
        Ok(gradient)
    }

    // Hessian computation methods (second derivatives)

    fn compute_spot_hessian(&self, s: f64, market_state: &MarketState) -> FeelsResult<f64> {
        let liquidity = (market_state.liquidity as f64) / 1e18;
        let hessian = -liquidity / ((s + 1.0) * (s + 1.0));
        Ok(hessian)
    }

    fn compute_time_hessian(&self, _t: f64, _market_state: &MarketState) -> FeelsResult<f64> {
        Ok(-0.01) // Constant second derivative
    }

    fn compute_leverage_hessian(&self, _l: f64, _market_state: &MarketState) -> FeelsResult<f64> {
        Ok(0.1) // Constant second derivative
    }

    // Cross derivative methods

    fn compute_spot_time_cross(&self, _position: &Position3D, _market_state: &MarketState) -> FeelsResult<f64> {
        // ∂²V/∂S∂T - coupling between spot and time
        Ok(0.001) // Small coupling
    }

    fn compute_spot_leverage_cross(&self, _position: &Position3D, _market_state: &MarketState) -> FeelsResult<f64> {
        // ∂²V/∂S∂L - coupling between spot and leverage  
        Ok(0.005) // Moderate coupling
    }

    fn compute_time_leverage_cross(&self, _position: &Position3D, _market_state: &MarketState) -> FeelsResult<f64> {
        // ∂²V/∂T∂L - coupling between time and leverage
        Ok(0.002) // Small coupling
    }

    // Utility methods

    fn compute_eigenvalues(&self, hessian: &Hessian3D) -> FeelsResult<(f64, f64, f64)> {
        // Simplified eigenvalue computation for 3x3 symmetric matrix
        // In practice would use proper numerical methods
        Ok((hessian.d2x, hessian.d2y, hessian.d2z))
    }

    fn eigenvalue_to_scalar(&self, eigenvalue: f64) -> FeelsResult<u128> {
        // Convert eigenvalue to scalar suitable for field commitment
        let abs_val = eigenvalue.abs();
        let scaled = abs_val * 1e12; // Scale up
        Ok(scaled as u128)
    }

    fn hash_market_state(&self, market_state: &MarketState) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        market_state.current_sqrt_price.hash(&mut hasher);
        market_state.liquidity.hash(&mut hasher);
        market_state.tick_current.hash(&mut hasher);
        hasher.finish()
    }

    fn get_next_sequence(&mut self, market_pubkey: &Pubkey) -> u64 {
        if let Some(cached) = self.field_cache.get(market_pubkey) {
            cached.field_data.sequence + 1
        } else {
            1
        }
    }
    
    // Stress calculation methods
    
    fn compute_spot_stress(&self, market_state: &MarketState) -> FeelsResult<u64> {
        // Calculate current price from sqrt price
        let sqrt_price = market_state.current_sqrt_price;
        let current_price = (sqrt_price as u128 * sqrt_price as u128) >> 64;
        
        // Use geometric mean of TWAPs as reference
        let twap_reference = if market_state.twap_0 > 0 && market_state.twap_1 > 0 {
            // Approximate geometric mean
            let avg = (market_state.twap_0 + market_state.twap_1) / 2;
            avg
        } else {
            current_price
        };
        
        if twap_reference == 0 {
            return Ok(0);
        }
        
        // Calculate deviation: |price - twap| / twap × 10000
        let deviation = if current_price > twap_reference {
            ((current_price - twap_reference) * 10000) / twap_reference
        } else {
            ((twap_reference - current_price) * 10000) / twap_reference
        };
        
        Ok(deviation.min(10000) as u64) // Cap at 100%
    }
    
    fn compute_time_stress(&self, market_state: &MarketState) -> FeelsResult<u64> {
        // Time stress based on lending utilization
        // For now, use a proxy based on liquidity concentration
        // In production, this would read from lending state
        
        // Higher liquidity = lower stress
        let liquidity_normalized = (market_state.liquidity as u128) / (1u128 << 64);
        
        // Invert to get stress (low liquidity = high stress)
        let stress = if liquidity_normalized > 0 {
            10000u64.saturating_sub((liquidity_normalized.min(10000)) as u64)
        } else {
            10000 // Max stress if no liquidity
        };
        
        Ok(stress)
    }
    
    fn compute_leverage_stress(&self, market_state: &MarketState) -> FeelsResult<u64> {
        // Leverage stress based on long/short imbalance
        // In production, this would read from leverage positions
        
        // For now, use fee growth as proxy for directional pressure
        let fee_growth_delta = market_state.fee_growth_global_0
            .abs_diff(market_state.fee_growth_global_1);
        
        // Normalize to basis points
        let stress = (fee_growth_delta >> 54) as u64; // Scale down from Q64 to bps
        
        Ok(stress.min(10000)) // Cap at 100%
    }
}

// Helper structs

#[derive(Debug)]
struct FieldScalars {
    S: u128,
    T: u128, 
    L: u128,
}

#[derive(Debug)]
struct DomainWeights {
    w_s: u32,
    w_t: u32,
    w_l: u32,
    w_tau: u32,
}

#[derive(Debug)]
struct SpotWeights {
    omega_0: u32,
    omega_1: u32,
}

#[derive(Debug)]
struct RiskParams {
    sigma_price: u64,
    sigma_rate: u64,
    sigma_leverage: u64,
}

impl FieldCommitmentData {
    /// Convert to keeper update parameters format
    pub fn to_keeper_update_params(&self) -> KeeperUpdateParams {
        KeeperUpdateParams {
            S: self.S,
            T: self.T,
            L: self.L,
            w_s: self.w_s,
            w_t: self.w_t,
            w_l: self.w_l,
            w_tau: self.w_tau,
            omega_0: self.omega_0,
            omega_1: self.omega_1,
            sigma_price: self.sigma_price,
            sigma_rate: self.sigma_rate,
            sigma_leverage: self.sigma_leverage,
            twap_0: self.twap_0,
            twap_1: self.twap_1,
            local_coeffs: None, // TODO: Implement local coefficients
            micro_field: None,  // TODO: Implement micro-field
            max_staleness: self.max_staleness,
            sequence: self.sequence,
        }
    }
}

/// Keeper update parameters (matching the on-chain instruction)
#[derive(Debug, Serialize, Deserialize)]
pub struct KeeperUpdateParams {
    pub S: u128,
    pub T: u128,
    pub L: u128,
    pub w_s: u32,
    pub w_t: u32,
    pub w_l: u32,
    pub w_tau: u32,
    pub omega_0: u32,
    pub omega_1: u32,
    pub sigma_price: u64,
    pub sigma_rate: u64,
    pub sigma_leverage: u64,
    pub twap_0: u128,
    pub twap_1: u128,
    pub local_coeffs: Option<LocalCoefficients>,
    pub micro_field: Option<MicroFieldCommitment>,
    pub max_staleness: i64,
    pub sequence: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalCoefficients {
    pub c0_s: i128,
    pub c1_s: i128,
    pub c0_t: i128,
    pub c1_t: i128,
    pub c0_l: i128,
    pub c1_l: i128,
    pub valid_until: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MicroFieldCommitment {
    pub root: [u8; 32],
    pub lipschitz_L: u64,
    pub curvature_bounds: (u64, u64),
}

impl KeeperUpdateParams {
    /// Convert to instruction data bytes
    pub fn to_instruction_data(&self) -> FeelsResult<Vec<u8>> {
        // Would use proper Anchor serialization
        // For now return placeholder
        Ok(vec![0u8; 8]) // Instruction discriminator placeholder
    }
}