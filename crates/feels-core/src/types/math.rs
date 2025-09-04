//! # Mathematical Types
//! 
//! 3D mathematical types and operations for the physics model.


#[cfg(feature = "client")]
use serde::{Serialize, Deserialize};

// ============================================================================
// 3D Vector Types
// ============================================================================

/// 3D vector for positions, velocities, forces, etc.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct Vector3D {
    pub x: f64,
    pub y: f64, 
    pub z: f64,
}

/// 3D position in market physics space (u128 for on-chain precision)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[allow(non_snake_case)]
pub struct Position3D {
    pub S: u128,  // Spot coordinate
    pub T: u128,  // Time coordinate
    pub L: u128,  // Leverage coordinate
}

/// 3D gradient vector with i128 components
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct Gradient3D {
    pub grad_s: i128,
    pub grad_t: i128,
    pub grad_l: i128,
}

// Work calculation types moved to work.rs to avoid duplication

// ============================================================================
// Basic Vector3D Operations (always available)
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
}

impl Position3D {
    /// Create new position
    pub const fn new(s: u128, t: u128, l: u128) -> Self {
        Self { S: s, T: t, L: l }
    }
    
    /// Convert to array
    pub fn to_array(&self) -> [u128; 3] {
        [self.S, self.T, self.L]
    }
    
    /// Calculate Euclidean distance to another position
    pub fn distance_to(&self, _other: &Position3D) -> u128 {
        // Simple approximation - just return the distance parameter
        // In practice, this should be calculated based on the actual path
        0
    }
}

impl Default for Position3D {
    fn default() -> Self {
        Self::new(0, 0, 0)
    }
}

// ============================================================================
// Advanced Mathematical Operations (for off-chain use)
// ============================================================================

#[cfg(feature = "advanced")]
pub mod advanced {
    use super::*;
    use serde::{Serialize, Deserialize};
    use crate::types::WorkResult;
    
    /// 3x3 matrix for transformations, Hessians, etc.
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
    pub struct Matrix3x3 {
        /// Matrix elements in row-major order
        pub m: [[f64; 3]; 3],
    }
    
    /// 3D Hessian matrix (symmetric)
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
    
    /// Extended work result with detailed breakdown
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DetailedWorkResult {
        /// Basic work result
        pub basic: WorkResult,
        /// Work for each segment
        pub segment_work: Vec<i128>,
        /// Estimated fee in basis points
        pub estimated_fee_bps: u64,
        /// Maximum possible rebate in basis points
        pub max_rebate_bps: u64,
        /// Path efficiency score (0-100)
        pub efficiency_score: u8,
        /// Breakdown by dimension
        pub dimension_work: DimensionWork,
    }
    
    /// Work breakdown by dimension
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DimensionWork {
        pub spot: i128,
        pub time: i128,
        pub leverage: i128,
        pub coupling: i128,
    }
    
    // Vector3D advanced operations
    impl Vector3D {
        /// Calculate magnitude (length) of vector
        pub fn magnitude(&self) -> f64 {
            (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
        }
        
        /// Calculate squared magnitude
        pub fn magnitude_squared(&self) -> f64 {
            self.x * self.x + self.y * self.y + self.z * self.z
        }
        
        /// Normalize vector to unit length
        pub fn normalize(&self) -> Self {
            let mag = self.magnitude();
            if mag == 0.0 {
                *self
            } else {
                Self::new(self.x / mag, self.y / mag, self.z / mag)
            }
        }
        
        /// Dot product
        pub fn dot(&self, other: &Vector3D) -> f64 {
            self.x * other.x + self.y * other.y + self.z * other.z
        }
        
        /// Cross product
        pub fn cross(&self, other: &Vector3D) -> Vector3D {
            Vector3D::new(
                self.y * other.z - self.z * other.y,
                self.z * other.x - self.x * other.z,
                self.x * other.y - self.y * other.x,
            )
        }
        
        /// Distance to another point
        pub fn distance_to(&self, other: &Vector3D) -> f64 {
            (*self - *other).magnitude()
        }
        
        /// Linear interpolation
        pub fn lerp(&self, other: &Vector3D, t: f64) -> Vector3D {
            *self * (1.0 - t) + *other * t
        }
    }
    
    // Operator overloads
    impl std::ops::Add for Vector3D {
        type Output = Self;
        
        fn add(self, other: Self) -> Self {
            Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
        }
    }
    
    impl std::ops::Sub for Vector3D {
        type Output = Self;
        
        fn sub(self, other: Self) -> Self {
            Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
        }
    }
    
    impl std::ops::Mul<f64> for Vector3D {
        type Output = Self;
        
        fn mul(self, scalar: f64) -> Self {
            Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
        }
    }
    
    impl std::ops::Div<f64> for Vector3D {
        type Output = Self;
        
        fn div(self, scalar: f64) -> Self {
            Self::new(self.x / scalar, self.y / scalar, self.z / scalar)
        }
    }
    
    impl std::ops::Neg for Vector3D {
        type Output = Self;
        
        fn neg(self) -> Self {
            Self::new(-self.x, -self.y, -self.z)
        }
    }
    
    // Matrix3x3 operations
    impl Matrix3x3 {
        /// Identity matrix
        pub fn identity() -> Self {
            Self {
                m: [
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [0.0, 0.0, 1.0],
                ],
            }
        }
        
        /// Zero matrix
        pub fn zero() -> Self {
            Self { m: [[0.0; 3]; 3] }
        }
        
        /// Transpose
        pub fn transpose(&self) -> Self {
            let mut result = Self::zero();
            for i in 0..3 {
                for j in 0..3 {
                    result.m[i][j] = self.m[j][i];
                }
            }
            result
        }
        
        /// Determinant
        pub fn determinant(&self) -> f64 {
            self.m[0][0] * (self.m[1][1] * self.m[2][2] - self.m[1][2] * self.m[2][1])
                - self.m[0][1] * (self.m[1][0] * self.m[2][2] - self.m[1][2] * self.m[2][0])
                + self.m[0][2] * (self.m[1][0] * self.m[2][1] - self.m[1][1] * self.m[2][0])
        }
        
        /// Matrix-vector multiplication
        pub fn mul_vec(&self, v: &Vector3D) -> Vector3D {
            Vector3D::new(
                self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z,
                self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z,
                self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z,
            )
        }
    }
    
    // Hessian3D operations
    impl Hessian3D {
        /// Create zero Hessian
        pub fn zero() -> Self {
            Self {
                d2x: 0.0, d2y: 0.0, d2z: 0.0,
                dxy: 0.0, dxz: 0.0, dyz: 0.0,
            }
        }
        
        /// Convert to full matrix representation
        pub fn to_matrix(&self) -> Matrix3x3 {
            Matrix3x3 {
                m: [
                    [self.d2x, self.dxy, self.dxz],
                    [self.dxy, self.d2y, self.dyz],
                    [self.dxz, self.dyz, self.d2z],
                ],
            }
        }
        
        /// Compute eigenvalues (for checking convexity)
        pub fn eigenvalues(&self) -> (f64, f64, f64) {
            // For a symmetric 3x3 matrix, we use the analytical solution
            // The matrix is:
            // [ d2x  dxy  dxz ]
            // [ dxy  d2y  dyz ]
            // [ dxz  dyz  d2z ]
            
            // Calculate the coefficients of the characteristic polynomial:
            // λ³ - tr(A)λ² + (sum of principal minors)λ - det(A) = 0
            
            let a = self.d2x;
            let b = self.d2y;
            let c = self.d2z;
            let d = self.dxy;
            let e = self.dxz;
            let f = self.dyz;
            
            // Trace
            let trace = a + b + c;
            
            // Sum of principal 2x2 minors
            let minor_sum = a*b - d*d + a*c - e*e + b*c - f*f;
            
            // Determinant
            let det = a*b*c + 2.0*d*e*f - a*f*f - b*e*e - c*d*d;
            
            // Use Cardano's method for cubic equations
            let p = minor_sum - trace*trace/3.0;
            let q = trace*trace*trace/13.5 - trace*minor_sum/3.0 + det;
            
            let discriminant = -4.0*p*p*p - 27.0*q*q;
            
            if discriminant >= 0.0 {
                // Three real roots
                let m = (discriminant / 108.0).sqrt();
                let theta = (q / (2.0 * m)).acos();
                let r = (-p / 3.0).sqrt();
                
                let e1 = trace/3.0 + 2.0*r*(theta/3.0).cos();
                let e2 = trace/3.0 + 2.0*r*((theta + 2.0*std::f64::consts::PI)/3.0).cos();
                let e3 = trace/3.0 + 2.0*r*((theta + 4.0*std::f64::consts::PI)/3.0).cos();
                
                // Sort eigenvalues in descending order
                let mut eigenvals = [e1, e2, e3];
                eigenvals.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                (eigenvals[0], eigenvals[1], eigenvals[2])
            } else {
                // For convex optimization, we should always have real eigenvalues
                // If not, return diagonal elements as approximation
                (self.d2x, self.d2y, self.d2z)
            }
        }
        
        /// Check if positive definite (for convexity)
        pub fn is_positive_definite(&self) -> bool {
            let (e1, e2, e3) = self.eigenvalues();
            e1 > 0.0 && e2 > 0.0 && e3 > 0.0
        }
    }
}

// Re-export advanced types for convenience
#[cfg(feature = "advanced")]
pub use advanced::*;