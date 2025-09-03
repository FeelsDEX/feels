/// 3D mathematical types and operations for the physics model

use anchor_lang::prelude::*;
use serde::{Deserialize, Serialize};
use fixed::types::I64F64 as FixedPoint;
use crate::constants::*;

// ============================================================================
// 3D Vector Types
// ============================================================================

/// 3D vector for positions, velocities, forces, etc.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Vector3D {
    pub x: f64,
    pub y: f64, 
    pub z: f64,
}

/// 3D position in market physics space
pub type Position3D = Vector3D;

/// 3D velocity in market physics space
pub type Velocity3D = Vector3D;

/// 3D gradient vector (first derivatives)
pub type Gradient3D = Vector3D;

// ============================================================================
// 3D Matrix Types
// ============================================================================

/// 3x3 matrix for transformations, Hessians, etc.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Matrix3x3 {
    /// Matrix elements in row-major order:
    /// | m00  m01  m02 |
    /// | m10  m11  m12 |
    /// | m20  m21  m22 |
    pub m: [[f64; 3]; 3],
}

/// 3D Hessian matrix (symmetric, so we can optimize storage)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Hessian3D {
    /// Diagonal elements (pure second derivatives)
    pub d2x: f64,   // ∂²f/∂x²
    pub d2y: f64,   // ∂²f/∂y²
    pub d2z: f64,   // ∂²f/∂z²
    
    /// Off-diagonal elements (cross derivatives, symmetric)
    pub dxy: f64,   // ∂²f/∂x∂y = ∂²f/∂y∂x
    pub dxz: f64,   // ∂²f/∂x∂z = ∂²f/∂z∂x
    pub dyz: f64,   // ∂²f/∂y∂z = ∂²f/∂z∂y
}

// ============================================================================
// Work Calculation Types
// ============================================================================

/// Path segment for work calculation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathSegment {
    /// Starting position in 3D space
    pub start: Position3D,
    /// Ending position in 3D space
    pub end: Position3D,
    /// Dimension being primarily traded
    pub primary_dimension: TradeDimension,
    /// Reserve changes (for spot dimension)
    pub reserve_change_a: i128,
    pub reserve_change_b: i128,
}

/// Trading dimension identifier
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TradeDimension {
    Spot,      // S dimension (traditional AMM trading)
    Time,      // T dimension (duration-based trading)
    Leverage,  // L dimension (leveraged trading)
    Mixed,     // Multiple dimensions simultaneously
}

/// Work calculation result for a path
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkResult {
    /// Total work along the path (can be negative for rebates)
    pub total_work: i128,
    /// Work for each segment
    pub segment_work: Vec<i128>,
    /// Estimated fee in token units
    pub estimated_fee: u64,
    /// Maximum possible rebate
    pub max_rebate: u64,
    /// Path efficiency score (0-100)
    pub efficiency_score: u8,
}

// ============================================================================
// Fixed-Point Math Types
// ============================================================================

/// Fixed-point number for high-precision calculations
pub type Fixed64 = FixedPoint;

/// Convert f64 to Fixed64
pub fn to_fixed(value: f64) -> Fixed64 {
    Fixed64::from_num(value)
}

/// Convert Fixed64 to f64
pub fn from_fixed(value: Fixed64) -> f64 {
    value.to_num()
}

// ============================================================================
// Vector3D Implementation
// ============================================================================

impl Vector3D {
    /// Create new 3D vector
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    
    /// Zero vector
    pub const fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
    
    /// Unit vectors
    pub const fn unit_x() -> Self { Self::new(1.0, 0.0, 0.0) }
    pub const fn unit_y() -> Self { Self::new(0.0, 1.0, 0.0) }
    pub const fn unit_z() -> Self { Self::new(0.0, 0.0, 1.0) }
    
    /// Calculate magnitude (length) of vector
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    
    /// Calculate squared magnitude (more efficient when you don't need the square root)
    pub fn magnitude_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }
    
    /// Normalize vector to unit length
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag == 0.0 {
            *self
        } else {
            Self {
                x: self.x / mag,
                y: self.y / mag,
                z: self.z / mag,
            }
        }
    }
    
    /// Dot product with another vector
    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    
    /// Cross product with another vector
    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
    
    /// Distance to another vector
    pub fn distance_to(&self, other: &Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx*dx + dy*dy + dz*dz).sqrt()
    }
    
    /// Linear interpolation between two vectors
    pub fn lerp(&self, other: &Self, t: f64) -> Self {
        Self {
            x: self.x + t * (other.x - self.x),
            y: self.y + t * (other.y - self.y),
            z: self.z + t * (other.z - self.z),
        }
    }
    
    /// Component-wise minimum
    pub fn min(&self, other: &Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
        }
    }
    
    /// Component-wise maximum
    pub fn max(&self, other: &Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
        }
    }
    
    /// Check if vector is within bounding box
    pub fn is_within_bounds(&self, min_bound: &Self, max_bound: &Self) -> bool {
        self.x >= min_bound.x && self.x <= max_bound.x &&
        self.y >= min_bound.y && self.y <= max_bound.y &&
        self.z >= min_bound.z && self.z <= max_bound.z
    }
}

// ============================================================================
// Vector3D Operator Implementations
// ============================================================================

impl std::ops::Add for Vector3D {
    type Output = Self;
    
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl std::ops::Sub for Vector3D {
    type Output = Self;
    
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl std::ops::Mul<f64> for Vector3D {
    type Output = Self;
    
    fn mul(self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl std::ops::Div<f64> for Vector3D {
    type Output = Self;
    
    fn div(self, scalar: f64) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
        }
    }
}

impl std::ops::Neg for Vector3D {
    type Output = Self;
    
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

// ============================================================================
// Matrix3x3 Implementation
// ============================================================================

impl Matrix3x3 {
    /// Create new 3x3 matrix
    pub fn new(m: [[f64; 3]; 3]) -> Self {
        Self { m }
    }
    
    /// Identity matrix
    pub fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ]
        }
    }
    
    /// Zero matrix
    pub fn zero() -> Self {
        Self { m: [[0.0; 3]; 3] }
    }
    
    /// Create diagonal matrix
    pub fn diagonal(d0: f64, d1: f64, d2: f64) -> Self {
        Self {
            m: [
                [d0, 0.0, 0.0],
                [0.0, d1, 0.0],
                [0.0, 0.0, d2],
            ]
        }
    }
    
    /// Matrix determinant
    pub fn determinant(&self) -> f64 {
        let m = &self.m;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) -
        m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) +
        m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }
    
    /// Matrix trace (sum of diagonal elements)
    pub fn trace(&self) -> f64 {
        self.m[0][0] + self.m[1][1] + self.m[2][2]
    }
    
    /// Matrix transpose
    pub fn transpose(&self) -> Self {
        let m = &self.m;
        Self {
            m: [
                [m[0][0], m[1][0], m[2][0]],
                [m[0][1], m[1][1], m[2][1]],
                [m[0][2], m[1][2], m[2][2]],
            ]
        }
    }
    
    /// Multiply matrix by vector
    pub fn mul_vector(&self, v: &Vector3D) -> Vector3D {
        Vector3D {
            x: self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z,
            y: self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z,
            z: self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z,
        }
    }
    
    /// Multiply two matrices
    pub fn mul_matrix(&self, other: &Self) -> Self {
        let mut result = Self::zero();
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    result.m[i][j] += self.m[i][k] * other.m[k][j];
                }
            }
        }
        result
    }
}

// ============================================================================
// Hessian3D Implementation  
// ============================================================================

impl Hessian3D {
    /// Create new Hessian matrix
    pub fn new(d2x: f64, d2y: f64, d2z: f64, dxy: f64, dxz: f64, dyz: f64) -> Self {
        Self { d2x, d2y, d2z, dxy, dxz, dyz }
    }
    
    /// Zero Hessian
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    }
    
    /// Convert to full 3x3 matrix representation
    pub fn to_matrix(&self) -> Matrix3x3 {
        Matrix3x3::new([
            [self.d2x, self.dxy, self.dxz],
            [self.dxy, self.d2y, self.dyz],
            [self.dxz, self.dyz, self.d2z],
        ])
    }
    
    /// Calculate determinant
    pub fn determinant(&self) -> f64 {
        self.to_matrix().determinant()
    }
    
    /// Calculate trace
    pub fn trace(&self) -> f64 {
        self.d2x + self.d2y + self.d2z
    }
    
    /// Check if matrix is positive definite (all eigenvalues positive)
    pub fn is_positive_definite(&self) -> bool {
        // Check Sylvester's criterion for 3x3 symmetric matrix
        let m1 = self.d2x;
        let m2 = self.d2x * self.d2y - self.dxy * self.dxy;
        let m3 = self.determinant();
        
        m1 > 0.0 && m2 > 0.0 && m3 > 0.0
    }
    
    /// Check if matrix is negative definite (all eigenvalues negative)
    pub fn is_negative_definite(&self) -> bool {
        // Check modified Sylvester's criterion
        let m1 = self.d2x;
        let m2 = self.d2x * self.d2y - self.dxy * self.dxy;
        let m3 = self.determinant();
        
        m1 < 0.0 && m2 > 0.0 && m3 < 0.0
    }
    
    /// Compute eigenvalues (simplified approximation)
    pub fn eigenvalues_approx(&self) -> (f64, f64, f64) {
        // This is a simplified approximation
        // For exact eigenvalues, would need more sophisticated algorithm
        let trace = self.trace();
        let det = self.determinant();
        
        // Use characteristic polynomial approximation
        // λ³ - trace*λ² + ... - det = 0
        
        // For now, return diagonal elements as approximation
        (self.d2x, self.d2y, self.d2z)
    }
}

// ============================================================================
// Work Calculation Functions
// ============================================================================

/// Calculate work along a 3D path using line integral
pub fn calculate_path_work(
    segments: &[PathSegment],
    field_gradient: impl Fn(&Position3D) -> Gradient3D,
) -> WorkResult {
    let mut total_work = 0i128;
    let mut segment_work = Vec::new();
    
    for segment in segments {
        // Approximate line integral using midpoint rule
        let midpoint = Position3D {
            x: (segment.start.x + segment.end.x) / 2.0,
            y: (segment.start.y + segment.end.y) / 2.0,
            z: (segment.start.z + segment.end.z) / 2.0,
        };
        
        let gradient = field_gradient(&midpoint);
        let displacement = segment.end - segment.start;
        
        // Work = ∫ F⃗ · dr⃗ ≈ F⃗(midpoint) · Δr⃗
        let work_f64 = gradient.dot(&displacement);
        let work = (work_f64 * (Q64 as f64)) as i128;
        
        segment_work.push(work);
        total_work += work;
    }
    
    // Calculate fee and rebate estimates
    let estimated_fee = if total_work > 0 {
        ((total_work as u128) / (Q64 / 1000)) as u64  // Convert to basis points
    } else {
        0
    };
    
    let max_rebate = if total_work < 0 {
        (((-total_work) as u128) / (Q64 / 100)) as u64  // 1% max rebate
    } else {
        0
    };
    
    // Calculate efficiency score (0-100)
    let efficiency_score = calculate_path_efficiency(segments, total_work);
    
    WorkResult {
        total_work,
        segment_work,
        estimated_fee,
        max_rebate,
        efficiency_score,
    }
}

/// Calculate path efficiency score
fn calculate_path_efficiency(segments: &[PathSegment], total_work: i128) -> u8 {
    if segments.is_empty() {
        return 0;
    }
    
    // Calculate total path length
    let mut total_length = 0.0;
    for segment in segments {
        total_length += segment.start.distance_to(&segment.end);
    }
    
    // Calculate direct distance
    let direct_distance = if let (Some(first), Some(last)) = (segments.first(), segments.last()) {
        first.start.distance_to(&last.end)
    } else {
        total_length
    };
    
    // Efficiency based on path directness and work magnitude
    let directness = if total_length > 0.0 {
        (direct_distance / total_length).min(1.0)
    } else {
        1.0
    };
    
    // Work efficiency (lower absolute work is more efficient for given distance)
    let work_efficiency = if total_length > 0.0 {
        let work_per_distance = (total_work.abs() as f64) / (Q64 as f64) / total_length;
        (1.0 / (1.0 + work_per_distance)).min(1.0)
    } else {
        1.0
    };
    
    let efficiency = (directness * 0.6 + work_efficiency * 0.4);
    (efficiency * 100.0).round() as u8
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Convert market coordinates to 3D position
pub fn market_to_position(
    sqrt_price: u128,
    liquidity: u128,
    time_factor: f64,
    leverage_factor: f64,
) -> Position3D {
    // Spot coordinate: logarithmic price scale
    let spot = ((sqrt_price as f64) / (Q64 as f64)).ln();
    
    // Time coordinate: normalized time factor
    let time = time_factor;
    
    // Leverage coordinate: logarithmic leverage scale  
    let leverage = leverage_factor.ln_1p();  // ln(1 + x) for stability
    
    Position3D::new(spot, time, leverage)
}

/// Convert 3D position back to market coordinates (approximate)
pub fn position_to_market(pos: &Position3D) -> (u128, f64, f64) {
    // Convert back from logarithmic scales
    let sqrt_price = ((pos.x.exp() * (Q64 as f64)) as u128).max(1);
    let time_factor = pos.y;
    let leverage_factor = pos.z.exp() - 1.0;
    
    (sqrt_price, time_factor, leverage_factor)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vector3d_operations() {
        let v1 = Vector3D::new(1.0, 2.0, 3.0);
        let v2 = Vector3D::new(4.0, 5.0, 6.0);
        
        assert_eq!(v1 + v2, Vector3D::new(5.0, 7.0, 9.0));
        assert_eq!(v1 - v2, Vector3D::new(-3.0, -3.0, -3.0));
        assert_eq!(v1 * 2.0, Vector3D::new(2.0, 4.0, 6.0));
        
        assert_eq!(v1.dot(&v2), 32.0);  // 1*4 + 2*5 + 3*6 = 32
        
        let magnitude = v1.magnitude();
        let expected = (1.0*1.0 + 2.0*2.0 + 3.0*3.0).sqrt();
        assert!((magnitude - expected).abs() < 1e-10);
    }
    
    #[test]
    fn test_matrix3x3_operations() {
        let m = Matrix3x3::identity();
        assert_eq!(m.determinant(), 1.0);
        assert_eq!(m.trace(), 3.0);
        
        let v = Vector3D::new(2.0, 3.0, 4.0);
        let result = m.mul_vector(&v);
        assert_eq!(result, v);  // Identity * v = v
    }
    
    #[test]
    fn test_hessian3d_properties() {
        // Positive definite matrix
        let h = Hessian3D::new(2.0, 1.0, 3.0, 0.5, 0.1, 0.2);
        assert!(h.is_positive_definite());
        
        // Test conversion to matrix
        let matrix = h.to_matrix();
        assert_eq!(matrix.m[0][0], h.d2x);
        assert_eq!(matrix.m[0][1], h.dxy);
        assert_eq!(matrix.m[1][0], h.dxy);  // Symmetric
    }
    
    #[test]
    fn test_market_position_conversion() {
        let sqrt_price = Q64;  // Price of 1.0
        let pos = market_to_position(sqrt_price, 1000000, 0.5, 2.0);
        
        let (recovered_price, time_factor, leverage_factor) = position_to_market(&pos);
        
        // Should be approximately equal (within floating point precision)
        assert!((recovered_price as f64 - sqrt_price as f64).abs() / (sqrt_price as f64) < 0.01);
        assert!((time_factor - 0.5).abs() < 1e-10);
        assert!((leverage_factor - 2.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_path_work_calculation() {
        let segments = vec![
            PathSegment {
                start: Position3D::new(0.0, 0.0, 0.0),
                end: Position3D::new(1.0, 0.0, 0.0),
                primary_dimension: TradeDimension::Spot,
                reserve_change_a: 1000,
                reserve_change_b: -900,
            }
        ];
        
        // Simple constant gradient field
        let gradient_fn = |_pos: &Position3D| -> Gradient3D {
            Gradient3D::new(1.0, 0.0, 0.0)  // Unit gradient in x direction
        };
        
        let result = calculate_path_work(&segments, gradient_fn);
        
        // Work should be positive (1.0 * 1.0 = 1.0 in this case)
        assert!(result.total_work > 0);
        assert_eq!(result.segment_work.len(), 1);
    }
}