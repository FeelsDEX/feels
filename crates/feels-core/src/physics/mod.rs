//! # Physics Module
//! 
//! Thermodynamic calculations for the 3D AMM including potential functions,
//! work calculations, and conservation laws.

pub mod potential;
pub mod work;
pub mod conservation;
pub mod field;

// Re-export main types
pub use potential::{MarketField, calculate_potential_linear, calculate_gradient};
pub use work::{calculate_path_work, calculate_segment_work, calculate_detailed_work};
pub use conservation::{ConservationData, GrowthFactors};
pub use field::{validate_field_update, FieldUpdateParams};