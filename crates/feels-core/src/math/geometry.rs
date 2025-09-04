//! # 3D Geometry Functions
//! 
//! Geometric calculations for the 3D thermodynamic AMM.
//! These functions are primarily used off-chain by the keeper for
//! market analytics and path optimization.

use crate::errors::{CoreResult, FeelsCoreError};
use crate::math::safe_math::{safe_add_u128, safe_sub_u128, safe_mul_u128, safe_div_u128, safe_add_i128, safe_sub_i128, safe_mul_i128, safe_div_i128, sqrt_u128};
use crate::types::{Position3D, PathSegment};

/// Calculate Euclidean distance between two 3D positions
#[cfg(feature = "advanced")]
pub fn distance_3d(p1: &Position3D, p2: &Position3D) -> CoreResult<u128> {
    // Calculate squared differences
    let dx = if p1.S > p2.S {
        safe_sub_u128(p1.S, p2.S)?
    } else {
        safe_sub_u128(p2.S, p1.S)?
    };
    
    let dy = if p1.T > p2.T {
        safe_sub_u128(p1.T, p2.T)?
    } else {
        safe_sub_u128(p2.T, p1.T)?
    };
    
    let dz = if p1.L > p2.L {
        safe_sub_u128(p1.L, p2.L)?
    } else {
        safe_sub_u128(p2.L, p1.L)?
    };
    
    // Calculate dx² + dy² + dz²
    let dx_squared = safe_mul_u128(dx, dx)?;
    let dy_squared = safe_mul_u128(dy, dy)?;
    let dz_squared = safe_mul_u128(dz, dz)?;
    
    let sum = safe_add_u128(dx_squared, safe_add_u128(dy_squared, dz_squared)?)?;
    
    // Return square root
    Ok(sqrt_u128(sum))
}

/// Calculate normalized direction vector between two positions
#[cfg(feature = "advanced")]
pub fn direction_vector_3d(from: &Position3D, to: &Position3D) -> CoreResult<(i128, i128, i128)> {
    let distance = distance_3d(from, to)?;
    
    if distance == 0 {
        return Err(FeelsCoreError::InvalidParameter);
    }
    
    // Calculate components
    let dx = if to.S > from.S {
        (safe_sub_u128(to.S, from.S)? as i128)
    } else {
        -(safe_sub_u128(from.S, to.S)? as i128)
    };
    
    let dy = if to.T > from.T {
        (safe_sub_u128(to.T, from.T)? as i128)
    } else {
        -(safe_sub_u128(from.T, to.T)? as i128)
    };
    
    let dz = if to.L > from.L {
        (safe_sub_u128(to.L, from.L)? as i128)
    } else {
        -(safe_sub_u128(from.L, to.L)? as i128)
    };
    
    // Normalize by distance
    let norm_x = safe_div_i128(dx, distance as i128)?;
    let norm_y = safe_div_i128(dy, distance as i128)?;
    let norm_z = safe_div_i128(dz, distance as i128)?;
    
    Ok((norm_x, norm_y, norm_z))
}

/// Calculate dot product of two 3D vectors
#[cfg(feature = "advanced")]
pub fn dot_product_3d(v1: (i128, i128, i128), v2: (i128, i128, i128)) -> CoreResult<i128> {
    let x_prod = safe_mul_i128(v1.0, v2.0)?;
    let y_prod = safe_mul_i128(v1.1, v2.1)?;
    let z_prod = safe_mul_i128(v1.2, v2.2)?;
    
    safe_add_i128(x_prod, safe_add_i128(y_prod, z_prod)?)
}

/// Calculate cross product of two 3D vectors
#[cfg(feature = "advanced")]
pub fn cross_product_3d(v1: (i128, i128, i128), v2: (i128, i128, i128)) -> CoreResult<(i128, i128, i128)> {
    // Cross product: (a2*b3 - a3*b2, a3*b1 - a1*b3, a1*b2 - a2*b1)
    let x = safe_sub_i128(safe_mul_i128(v1.1, v2.2)?, safe_mul_i128(v1.2, v2.1)?)?;
    let y = safe_sub_i128(safe_mul_i128(v1.2, v2.0)?, safe_mul_i128(v1.0, v2.2)?)?;
    let z = safe_sub_i128(safe_mul_i128(v1.0, v2.1)?, safe_mul_i128(v1.1, v2.0)?)?;
    
    Ok((x, y, z))
}

/// Create a path segment between two positions
#[cfg(feature = "advanced")]
pub fn create_path_segment(start: Position3D, end: Position3D) -> CoreResult<PathSegment> {
    let distance = distance_3d(&start, &end)?;
    
    Ok(PathSegment {
        start,
        end,
        distance,
        liquidity: 0,  // Default to 0, caller should set actual liquidity
        dimension: crate::types::TradeDimension::Spot,  // Default to Spot, caller should set actual dimension
    })
}

/// Calculate the center of mass for a set of weighted positions
#[cfg(feature = "advanced")]
pub fn center_of_mass_3d(positions: &[(Position3D, u128)]) -> CoreResult<Position3D> {
    if positions.is_empty() {
        return Err(FeelsCoreError::InvalidParameter);
    }
    
    let mut total_weight = 0u128;
    let mut weighted_s = 0u128;
    let mut weighted_t = 0u128;
    let mut weighted_l = 0u128;
    
    for (pos, weight) in positions {
        total_weight = safe_add_u128(total_weight, *weight)?;
        weighted_s = safe_add_u128(weighted_s, safe_mul_u128(pos.S, *weight)?)?;
        weighted_t = safe_add_u128(weighted_t, safe_mul_u128(pos.T, *weight)?)?;
        weighted_l = safe_add_u128(weighted_l, safe_mul_u128(pos.L, *weight)?)?;
    }
    
    if total_weight == 0 {
        return Err(FeelsCoreError::DivisionByZero);
    }
    
    Ok(Position3D {
        S: safe_div_u128(weighted_s, total_weight)?,
        T: safe_div_u128(weighted_t, total_weight)?,
        L: safe_div_u128(weighted_l, total_weight)?,
    })
}