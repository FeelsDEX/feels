/// Volatility tracking structures for market risk management
use anchor_lang::prelude::*;

/// A single volatility observation containing log return data
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VolatilityObservation {
    /// Unix timestamp when this observation was recorded
    pub timestamp: i64,
    /// Squared log return scaled by 10^6 for precision
    pub log_return_squared: u32,
}

impl VolatilityObservation {
    pub const SIZE: usize = 8 + 4; // timestamp (i64) + log_return_squared (u32)
}