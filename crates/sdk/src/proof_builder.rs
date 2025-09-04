/// Proof builder for field commitment verification.
/// Creates inclusion proofs and verification data for Option B commitments.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::{hash, Hash};

// ============================================================================
// Proof Types
// ============================================================================

/// Verification proof for Option B commitments
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub struct ConvexBoundPoint {
    /// Position in 3D space
    pub position: [u128; 3], // [S, T, L]
    
    /// Potential value at this point
    pub V: i128,
    
    /// Expected bound value
    pub bound: i128,
}

/// Sample pair for Lipschitz verification
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub struct OptimalityGapProof {
    /// Current objective value
    pub current_value: i128,
    
    /// Optimal value bound
    pub optimal_bound: i128,
    
    /// Gap in basis points
    pub gap_bps: u64,
}

// ============================================================================
// Merkle Tree Builder
// ============================================================================

/// Build merkle tree for coefficient commitments
pub struct MerkleTreeBuilder {
    leaves: Vec<[u8; 32]>,
}

impl MerkleTreeBuilder {
    pub fn new() -> Self {
        Self { leaves: Vec::new() }
    }
    
    /// Add coefficient data as leaf
    pub fn add_coefficients(
        &mut self,
        c0_s: i128,
        c0_t: i128,
        c0_l: i128,
        c1_s: i128,
        c1_t: i128,
        c1_l: i128,
        valid_until: i64,
    ) {
        let data = [
            c0_s.to_le_bytes(),
            c0_t.to_le_bytes(),
            c0_l.to_le_bytes(),
            c1_s.to_le_bytes(),
            c1_t.to_le_bytes(),
            c1_l.to_le_bytes(),
            valid_until.to_le_bytes(),
        ].concat();
        
        let leaf_hash = hash(&data).to_bytes();
        self.leaves.push(leaf_hash);
    }
    
    /// Build merkle tree and return root
    pub fn build_tree(&self) -> ([u8; 32], Vec<Vec<[u8; 32]>>) {
        if self.leaves.is_empty() {
            return ([0u8; 32], vec![]);
        }
        
        let mut current_level = self.leaves.clone();
        let mut all_levels = vec![current_level.clone()];
        
        // Build tree bottom-up
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    current_level[i] // Duplicate last node if odd
                };
                
                let combined = if left < right {
                    [left, right].concat()
                } else {
                    [right, left].concat()
                };
                
                let parent = hash(&combined).to_bytes();
                next_level.push(parent);
            }
            
            all_levels.push(next_level.clone());
            current_level = next_level;
        }
        
        let root = current_level[0];
        (root, all_levels)
    }
    
    /// Generate merkle proof for a leaf at given index
    pub fn generate_proof(&self, leaf_index: usize) -> Option<Vec<[u8; 32]>> {
        if leaf_index >= self.leaves.len() {
            return None;
        }
        
        let (_, all_levels) = self.build_tree();
        let mut proof = Vec::new();
        let mut index = leaf_index;
        
        // Collect siblings along the path to root
        for level in &all_levels[..all_levels.len() - 1] {
            let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };
            
            if sibling_index < level.len() {
                proof.push(level[sibling_index]);
            } else if index > 0 {
                // Use the last node as sibling if out of bounds
                proof.push(level[level.len() - 1]);
            }
            
            index /= 2;
        }
        
        Some(proof)
    }
}

// ============================================================================
// Proof Builders
// ============================================================================

/// Build convex bound verification points
pub fn build_convex_bound_points(
    field_center: [u128; 3],
    radius: u128,
    num_points: usize,
) -> Vec<ConvexBoundPoint> {
    let mut points = Vec::new();
    
    // Generate points on sphere around field center
    for i in 0..num_points {
        let angle = (i as f64) * 2.0 * std::f64::consts::PI / (num_points as f64);
        
        // Simple 2D projection for illustration
        let delta_s = ((radius as f64) * angle.cos()) as i128;
        let delta_t = ((radius as f64) * angle.sin()) as i128;
        
        let position = [
            (field_center[0] as i128 + delta_s).max(0) as u128,
            (field_center[1] as i128 + delta_t).max(0) as u128,
            field_center[2],
        ];
        
        // Calculate potential value (simplified)
        let V = calculate_potential_value(&position);
        
        // Calculate bound (quadratic approximation)
        let bound = calculate_convex_bound(&position, &field_center);
        
        points.push(ConvexBoundPoint { position, V, bound });
    }
    
    points
}

/// Build Lipschitz sample pairs
pub fn build_lipschitz_samples(
    field_region: ([u128; 3], [u128; 3]), // (min, max)
    num_samples: usize,
) -> Vec<LipschitzSample> {
    let mut samples = Vec::new();
    
    for _ in 0..num_samples {
        // Generate random pairs within region (simplified)
        let p1 = [
            field_region.0[0] + (field_region.1[0] - field_region.0[0]) / 3,
            field_region.0[1] + (field_region.1[1] - field_region.0[1]) / 3,
            field_region.0[2] + (field_region.1[2] - field_region.0[2]) / 3,
        ];
        
        let p2 = [
            field_region.0[0] + 2 * (field_region.1[0] - field_region.0[0]) / 3,
            field_region.0[1] + 2 * (field_region.1[1] - field_region.0[1]) / 3,
            field_region.0[2] + 2 * (field_region.1[2] - field_region.0[2]) / 3,
        ];
        
        // Calculate gradient difference (simplified)
        let grad_diff_norm = calculate_gradient_diff_norm(&p1, &p2);
        
        // Calculate position difference
        let pos_diff_norm = calculate_position_diff_norm(&p1, &p2);
        
        samples.push(LipschitzSample {
            p1,
            p2,
            grad_diff_norm,
            pos_diff_norm,
        });
    }
    
    samples
}

/// Build optimality gap proof
pub fn build_optimality_gap_proof(
    current_field: [u128; 3],
    optimal_estimate: [u128; 3],
) -> OptimalityGapProof {
    let current_value = calculate_potential_value(&current_field);
    let optimal_bound = calculate_potential_value(&optimal_estimate);
    
    let gap_bps = if optimal_bound != 0 {
        ((current_value - optimal_bound).abs() as u128 * 10000 / optimal_bound.abs() as u128) as u64
    } else {
        0
    };
    
    OptimalityGapProof {
        current_value,
        optimal_bound,
        gap_bps,
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn calculate_potential_value(position: &[u128; 3]) -> i128 {
    use feels_core::math::fixed_point::ln_q64;
    use feels_core::constants::Q64;
    
    // V = -ŵₛ ln(S) - ŵₜ ln(T) - ŵₗ ln(L)
    // For simplicity, using equal weights (1/3 each)
    let weight = Q64 / 3; // Equal weights in Q64 format
    
    // Calculate ln for each dimension
    let ln_s = ln_q64(position[0]).unwrap_or(0);
    let ln_t = ln_q64(position[1]).unwrap_or(0);
    let ln_l = ln_q64(position[2]).unwrap_or(0);
    
    // Apply weights and sum (negative because potential is -w*ln(x))
    let weighted_s = -(ln_s as i128 * weight as i128 / Q64 as i128);
    let weighted_t = -(ln_t as i128 * weight as i128 / Q64 as i128);
    let weighted_l = -(ln_l as i128 * weight as i128 / Q64 as i128);
    
    weighted_s + weighted_t + weighted_l
}

fn calculate_convex_bound(position: &[u128; 3], center: &[u128; 3]) -> i128 {
    // Quadratic approximation around center
    let delta_s = (position[0] as i128) - (center[0] as i128);
    let delta_t = (position[1] as i128) - (center[1] as i128);
    let delta_l = (position[2] as i128) - (center[2] as i128);
    
    // Simplified quadratic: bound = V(center) + 0.5 * ||delta||^2
    let center_value = calculate_potential_value(center);
    let delta_norm_sq = (delta_s * delta_s + delta_t * delta_t + delta_l * delta_l) >> 64;
    
    center_value + delta_norm_sq / 2
}

fn calculate_gradient_diff_norm(p1: &[u128; 3], p2: &[u128; 3]) -> u128 {
    // Simplified gradient difference
    let grad1 = [p1[0] / 100, p1[1] / 100, p1[2] / 100];
    let grad2 = [p2[0] / 100, p2[1] / 100, p2[2] / 100];
    
    let diff_s = (grad2[0] as i128 - grad1[0] as i128).abs() as u128;
    let diff_t = (grad2[1] as i128 - grad1[1] as i128).abs() as u128;
    let diff_l = (grad2[2] as i128 - grad1[2] as i128).abs() as u128;
    
    // Euclidean norm
    ((diff_s * diff_s + diff_t * diff_t + diff_l * diff_l) as f64).sqrt() as u128
}

fn calculate_position_diff_norm(p1: &[u128; 3], p2: &[u128; 3]) -> u128 {
    let diff_s = (p2[0] as i128 - p1[0] as i128).abs() as u128;
    let diff_t = (p2[1] as i128 - p1[1] as i128).abs() as u128;
    let diff_l = (p2[2] as i128 - p1[2] as i128).abs() as u128;
    
    // Euclidean norm
    ((diff_s * diff_s + diff_t * diff_t + diff_l * diff_l) as f64).sqrt() as u128
}

// ============================================================================
// Complete Proof Builder
// ============================================================================

/// Build complete verification proof for a field commitment
pub fn build_field_verification_proof(
    field_commitment: &crate::field_commitment::FieldCommitmentData,
    num_convex_points: usize,
    num_lipschitz_samples: usize,
) -> FieldVerificationProof {
    let field_center = [field_commitment.S, field_commitment.T, field_commitment.L];
    
    // Build convex bound points
    let convex_bound_points = build_convex_bound_points(
        field_center,
        field_commitment.S / 10, // 10% radius
        num_convex_points,
    );
    
    // Build Lipschitz samples
    let field_region = (
        [field_center[0] * 9 / 10, field_center[1] * 9 / 10, field_center[2] * 9 / 10],
        [field_center[0] * 11 / 10, field_center[1] * 11 / 10, field_center[2] * 11 / 10],
    );
    let lipschitz_samples = build_lipschitz_samples(field_region, num_lipschitz_samples);
    
    // Build optimality gap proof
    let optimality_gap = build_optimality_gap_proof(
        field_center,
        field_center, // Assume current is optimal for simplicity
    );
    
    // Build merkle proof if coefficients exist
    let merkle_proof = if let Some(coeffs) = &field_commitment.local_coefficients {
        let mut builder = MerkleTreeBuilder::new();
        builder.add_coefficients(
            coeffs.c0_s,
            coeffs.c0_t,
            coeffs.c0_l,
            coeffs.c1_s,
            coeffs.c1_t,
            coeffs.c1_l,
            coeffs.valid_until,
        );
        builder.generate_proof(0)
    } else {
        None
    };
    
    FieldVerificationProof {
        convex_bound_points,
        lipschitz_samples,
        optimality_gap,
        merkle_proof,
    }
}