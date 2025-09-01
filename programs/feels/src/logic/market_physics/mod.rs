/// Market physics calculations including potential functions, gradients, and work.
/// These modules implement the mathematical foundation of the 3D AMM model.

pub mod conservation;
pub mod potential;
pub mod gradient;
pub mod hessian;
pub mod work;

// Re-export commonly used items
pub use conservation::{verify_conservation, solve_conservation_factor};
pub use potential::{calculate_potential, FixedPoint, ln_fixed, exp_fixed};
pub use gradient::{calculate_gradient_3d, Gradient3D, GradientCalculator};
pub use hessian::{calculate_hessian_3x3, Hessian3x3, HessianCalculator};
pub use work::{calculate_work, calculate_fee_and_rebate, WorkCalculator};