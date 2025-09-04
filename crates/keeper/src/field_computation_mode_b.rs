/// Mode B field computation with local approximations and global bounds
/// Supports hub-constrained routing with efficient on-chain verification
use solana_sdk::pubkey::Pubkey;
use serde::{Serialize, Deserialize};

use feels_core::{
    types::{
        field::FieldCommitmentData,
        market::extended::MarketState,
    },
    errors::extended::ExtendedResult as FeelsResult,
};
use crate::field_computation::{FieldComputer, KeeperUpdateParams};

/// Mode B specific structures for local approximations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalApproximation {
    /// Center point of approximation
    pub center: Position3D,
    /// Taylor coefficients (constant, linear, quadratic)
    pub coefficients: TaylorCoefficients,
    /// Validity radius in each dimension
    pub radius: Radius3D,
    /// Lipschitz bound for error estimation
    pub lipschitz_bound: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaylorCoefficients {
    /// Constant term
    pub c0: i128,
    /// Linear coefficients [∂S, ∂T, ∂L]
    pub c1: [i128; 3],
    /// Quadratic coefficients (diagonal only for efficiency)
    pub c2: [i128; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position3D {
    pub s: i64, // Spot dimension (log scale)
    pub t: i64, // Time dimension
    pub l: i64, // Leverage dimension
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Radius3D {
    pub r_s: u64, // Spot radius
    pub r_t: u64, // Time radius
    pub r_l: u64, // Leverage radius
}

/// Global bounds for Mode B verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBounds {
    /// Maximum allowed work across any route
    pub max_work: u128,
    /// Minimum fee in basis points
    pub min_fee_bps: u16,
    /// Maximum slippage allowed
    pub max_slippage_bps: u16,
    /// Hub pool bounds (FeelsSOL pairs)
    pub hub_bounds: HubPoolBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubPoolBounds {
    /// Maximum price impact for single hop through hub
    pub max_single_hop_impact_bps: u16,
    /// Maximum cumulative impact for 2-hop routes
    pub max_two_hop_impact_bps: u16,
    /// Segment count limits
    pub max_segments_per_hop: u8,
    pub max_segments_total: u8,
}

/// Mode B field computer extension
pub struct ModeBFieldComputer {
    base_computer: FieldComputer,
    approximations: Vec<LocalApproximation>,
    global_bounds: GlobalBounds,
}

impl ModeBFieldComputer {
    pub fn new() -> Self {
        Self {
            base_computer: FieldComputer::new(),
            approximations: Vec::new(),
            global_bounds: GlobalBounds {
                max_work: 1_000_000_000, // 1000 units
                min_fee_bps: 1, // 0.01%
                max_slippage_bps: 100, // 1%
                hub_bounds: HubPoolBounds {
                    max_single_hop_impact_bps: 50, // 0.5%
                    max_two_hop_impact_bps: 100, // 1%
                    max_segments_per_hop: 10,
                    max_segments_total: 20,
                },
            },
        }
    }

    /// Compute Mode B field commitment with local approximations
    pub fn compute_mode_b_commitment(
        &mut self,
        market_state: &MarketState,
        hub_pools: &[Pubkey],
    ) -> FeelsResult<ModeBCommitment> {
        // Get base field commitment
        let base_commitment = self.base_computer.compute_field_commitment(market_state)?;
        
        // Compute local approximations for hub pools
        let approximations = self.compute_local_approximations(market_state, hub_pools)?;
        
        // Compute global bounds based on current state
        let bounds = self.compute_global_bounds(market_state, &approximations)?;
        
        // Build Mode B commitment
        Ok(ModeBCommitment {
            base_commitment,
            approximations,
            bounds,
            hub_pools: hub_pools.to_vec(),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Compute local Taylor approximations around operating points
    fn compute_local_approximations(
        &self,
        market_state: &MarketState,
        hub_pools: &[Pubkey],
    ) -> FeelsResult<Vec<LocalApproximation>> {
        let mut approximations = Vec::new();
        
        // For each hub pool, compute local approximation
        for pool in hub_pools {
            // Get current position
            let position = self.get_pool_position(market_state, pool)?;
            
            // Compute Taylor coefficients
            let coeffs = self.compute_taylor_coefficients(&position, market_state)?;
            
            // Determine validity radius
            let radius = self.compute_validity_radius(&position, &coeffs)?;
            
            // Compute Lipschitz bound for error estimation
            let lipschitz = self.compute_lipschitz_bound(&position, &radius)?;
            
            approximations.push(LocalApproximation {
                center: position,
                coefficients: coeffs,
                radius,
                lipschitz_bound: lipschitz,
            });
        }
        
        Ok(approximations)
    }

    /// Compute Taylor coefficients at position
    fn compute_taylor_coefficients(
        &self,
        position: &Position3D,
        market_state: &MarketState,
    ) -> FeelsResult<TaylorCoefficients> {
        // Convert to internal position type for gradient computation
        let pos_3d = feels_types::Position3D::new(
            position.s as f64,
            position.t as f64,
            position.l as f64,
        );
        
        // Get gradient (first order)
        let gradient = self.base_computer.compute_gradient(&pos_3d, market_state)?;
        
        // Get Hessian (second order)
        let hessian = self.base_computer.compute_hessian(&pos_3d, market_state)?;
        
        // Extract coefficients
        Ok(TaylorCoefficients {
            c0: 0, // Constant term (potential at center)
            c1: [
                (gradient.ds * 1e9) as i128, // Scale for precision
                (gradient.dt * 1e9) as i128,
                (gradient.dl * 1e9) as i128,
            ],
            c2: [
                (hessian.d2S * 1e9) as i128, // Diagonal terms only
                (hessian.d2T * 1e9) as i128,
                (hessian.d2L * 1e9) as i128,
            ],
        })
    }

    /// Compute validity radius for approximation
    fn compute_validity_radius(
        &self,
        position: &Position3D,
        coeffs: &TaylorCoefficients,
    ) -> FeelsResult<Radius3D> {
        // Radius where Taylor error < threshold
        // Based on third derivative bounds (simplified)
        
        Ok(Radius3D {
            r_s: 1000, // 0.1% price movement
            r_t: 3600, // 1 hour time window
            r_l: 100,  // 10% leverage change
        })
    }

    /// Compute Lipschitz bound for error estimation
    fn compute_lipschitz_bound(
        &self,
        position: &Position3D,
        radius: &Radius3D,
    ) -> FeelsResult<u64> {
        // Maximum gradient magnitude in neighborhood
        // Simplified: use second derivative bounds
        
        Ok(10000) // Placeholder - would compute from Hessian eigenvalues
    }

    /// Get pool position in 3D space
    fn get_pool_position(
        &self,
        market_state: &MarketState,
        pool: &Pubkey,
    ) -> FeelsResult<Position3D> {
        // For hub pools, position based on FeelsSOL side
        // Simplified for now
        
        Ok(Position3D {
            s: (market_state.current_sqrt_price as i64) >> 32, // Scale down
            t: 0, // Neutral time position
            l: 0, // Neutral leverage
        })
    }

    /// Compute global bounds from approximations
    fn compute_global_bounds(
        &self,
        market_state: &MarketState,
        approximations: &[LocalApproximation],
    ) -> FeelsResult<GlobalBounds> {
        // Analyze approximations to set safe global bounds
        
        let mut max_work = 0u128;
        let mut max_impact = 0u16;
        
        for approx in approximations {
            // Estimate maximum work in validity region
            let work_bound = self.estimate_max_work(approx)?;
            max_work = max_work.max(work_bound);
            
            // Estimate maximum price impact
            let impact = self.estimate_max_impact(approx)?;
            max_impact = max_impact.max(impact);
        }
        
        Ok(GlobalBounds {
            max_work: max_work * 2, // Safety factor
            min_fee_bps: 1,
            max_slippage_bps: max_impact * 2,
            hub_bounds: HubPoolBounds {
                max_single_hop_impact_bps: max_impact,
                max_two_hop_impact_bps: max_impact * 2,
                max_segments_per_hop: 10,
                max_segments_total: 20,
            },
        })
    }

    /// Estimate maximum work in approximation region
    fn estimate_max_work(&self, approx: &LocalApproximation) -> FeelsResult<u128> {
        // Work = integral of gradient magnitude
        // Upper bound using Lipschitz constant
        
        let radius_norm = (approx.radius.r_s.pow(2) + 
                          approx.radius.r_t.pow(2) + 
                          approx.radius.r_l.pow(2)) as f64;
        
        let max_gradient = approx.c1.iter()
            .map(|&c| c.abs() as u64)
            .max()
            .unwrap_or(0);
            
        Ok((max_gradient as u128) * (radius_norm.sqrt() as u128))
    }

    /// Estimate maximum price impact
    fn estimate_max_impact(&self, approx: &LocalApproximation) -> FeelsResult<u16> {
        // Impact based on curvature (second derivatives)
        
        let max_curvature = approx.c2.iter()
            .map(|&c| c.abs() as u64)
            .max()
            .unwrap_or(0);
            
        // Convert to basis points
        Ok((max_curvature / 1_000_000) as u16)
    }
}

/// Mode B commitment structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeBCommitment {
    /// Base field commitment (Mode A compatible)
    pub base_commitment: FieldCommitmentData,
    /// Local approximations for efficient computation
    pub approximations: Vec<LocalApproximation>,
    /// Global bounds for safety
    pub bounds: GlobalBounds,
    /// Hub pools covered
    pub hub_pools: Vec<Pubkey>,
    /// Timestamp
    pub timestamp: i64,
}

impl ModeBCommitment {
    /// Convert to on-chain update parameters
    pub fn to_update_params(&self) -> KeeperUpdateParams {
        let mut params = self.base_commitment.to_keeper_update_params();
        
        // Add Mode B specific data
        params.local_coeffs = Some(LocalCoefficients {
            // Aggregate approximations (simplified)
            c0_s: self.approximations.first().map(|a| a.coefficients.c0).unwrap_or(0),
            c1_s: self.approximations.first().map(|a| a.coefficients.c1[0]).unwrap_or(0),
            c0_t: 0,
            c1_t: self.approximations.first().map(|a| a.coefficients.c1[1]).unwrap_or(0),
            c0_l: 0,
            c1_l: self.approximations.first().map(|a| a.coefficients.c1[2]).unwrap_or(0),
        });
        
        params.micro_field = Some(MicroFieldCommitment {
            lipschitz_L: self.approximations.first().map(|a| a.lipschitz_bound).unwrap_or(0),
            gap_bps: self.bounds.max_slippage_bps,
            max_segments: self.bounds.hub_bounds.max_segments_total,
        });
        
        params
    }

    /// Verify route is within bounds
    pub fn verify_route(&self, route_work: u128, segment_count: u8) -> bool {
        route_work <= self.bounds.max_work &&
        segment_count <= self.bounds.hub_bounds.max_segments_total
    }
}

/// Local coefficients for on-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalCoefficients {
    pub c0_s: i128,
    pub c1_s: i128,
    pub c0_t: i128,
    pub c1_t: i128,
    pub c0_l: i128,
    pub c1_l: i128,
}

/// Micro field commitment for bounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroFieldCommitment {
    pub lipschitz_L: u64,
    pub gap_bps: u16,
    pub max_segments: u8,
}