//! # Work and Path Types
//! 
//! Types for thermodynamic work calculations along trading paths.

use crate::types::{Position3D, field::TradeDimension};

/// Result of work calculation along a trading path
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct WorkResult {
    /// Total work performed (always positive)
    pub total_work: u128,
    /// Net work (can be negative if price improves)
    pub net_work: i128,
    /// Path-weighted average work
    pub weighted_work: u128,
    /// Number of segments in path
    pub segments: u32,
}

/// Path segment for work calculation
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct PathSegment {
    /// Starting position
    pub start: Position3D,
    /// Ending position
    pub end: Position3D,
    /// Liquidity in this segment
    pub liquidity: u128,
    /// Distance traveled (amount swapped)
    pub distance: u128,
    /// Primary dimension being traded
    pub dimension: TradeDimension,
}

impl PathSegment {
    /// Create a new path segment
    pub fn new(start: Position3D, end: Position3D, liquidity: u128, distance: u128, dimension: TradeDimension) -> Self {
        Self { start, end, liquidity, distance, dimension }
    }
    
    /// Calculate segment length in 3D space
    pub fn length(&self) -> u128 {
        self.start.distance_to(&self.end)
    }
}

/// Market field update for verify-apply pattern
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy)]
pub struct MarketFieldUpdate {
    /// New spot scalar
    pub spot_scalar: u128,
    /// New time scalar
    pub time_scalar: u128,
    /// New leverage scalar
    pub leverage_scalar: u128,
    /// Update sequence number
    pub sequence: u64,
    /// Update timestamp
    pub timestamp: i64,
    /// Update authority
    #[cfg(feature = "anchor")]
    pub authority: anchor_lang::prelude::Pubkey,
    #[cfg(not(feature = "anchor"))]
    pub authority: [u8; 32],
}

/// Work calculation parameters
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct WorkCalculationParams {
    /// Path segments to calculate work along
    pub segments: Vec<PathSegment>,
    /// Domain weights for potential calculation
    pub weights: crate::types::DomainWeights,
    /// Current market position
    pub current_position: Position3D,
}